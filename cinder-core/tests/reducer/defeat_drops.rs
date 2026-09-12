use super::common::*;
use cinder_core::content::types::{
    ActionDefinition, CommandEffect, CommandTargetMode, ContentPack, DropChanceSpec, DropPoolEntry,
    DropPoolSpec, DropSpec, ItemDefinition,
};
use cinder_core::engine::state::WorldState;
use serde_json::json;
use std::collections::BTreeMap;

const CLEAN_RUN_FLAG: &str = "layla_attacked_floor_one_mob";

fn attack_action_pack() -> ContentPack {
    let mut pack = equipment_test_pack();
    pack.actions.push(ActionDefinition {
        id: "attack".to_string(),
        command: "attack".to_string(),
        target_mode: CommandTargetMode::Actor,
        effects: vec![CommandEffect::AttackTarget],
        event_text: "{actor_name} strikes {target_actor_name}.".to_string(),
        ..ActionDefinition::default()
    });
    pack
}

fn add_attackable_target(pack: &mut ContentPack, id: &str, tags: &[&str], stamina: i32) {
    let mut actor = test_actor(id, id, LOUNGE_ID);
    actor.attackable = true;
    actor.tags = tags.iter().map(|tag| tag.to_string()).collect();
    actor.initial_stats.insert("stamina".to_string(), stamina);
    pack.actors.push(actor);
    rebuild_test_pack_indexes(pack);
}

fn drive_attack_on(state: &mut WorldState, pack: &ContentPack, target: &str) {
    drive_actor_command(
        state,
        pack,
        "attack",
        ActorCommandInput {
            actor_id: ACTOR_A_ID,
            actor_name: ACTOR_A_NAME,
            room_id: LOUNGE_ID,
            target_room_id: None,
            target_actor_id: Some(target),
            target_actor_name: Some(target),
            context_label: None,
            feature_id: None,
            consumable_id: None,
            freeform_text: None,
        },
    );
}

fn fresh_state(pack: &ContentPack) -> WorldState {
    let mut state = WorldState::new(pack);
    state.current_room_id = LOUNGE_ID.to_string();
    state
}

fn push_droppable_item(pack: &mut ContentPack, id: &str) {
    pack.items.push(ItemDefinition {
        id: id.to_string(),
        label: id.to_string(),
        description: String::new(),
        ..ItemDefinition::default()
    });
}

fn drive_defeat(pack: &ContentPack, drops: BTreeMap<String, DropSpec>) -> Vec<(String, u32)> {
    let mut pack = pack.clone();
    push_droppable_item(&mut pack, "leaf-blade");
    push_droppable_item(&mut pack, "bark-bracer");
    add_attackable_target(&mut pack, "golem", &["golem"], 1);
    let golem = pack.actors.iter_mut().find(|actor| actor.id == "golem").unwrap();
    golem.drops = drops;
    let mut state = fresh_state(&pack);
    drive_attack_on(&mut state, &pack, "golem");
    state.loose_room_items(LOUNGE_ID)
}

#[test]
fn unconditional_drop_scatters_into_the_room() {
    let mut pack = attack_action_pack();
    add_attackable_target(&mut pack, "golem", &["golem"], 1);
    let golem = pack.actors.iter_mut().find(|actor| actor.id == "golem").unwrap();
    golem.drops = BTreeMap::from([("herb-salve".to_string(), DropSpec::Always(2))]);

    let mut state = fresh_state(&pack);
    drive_attack_on(&mut state, &pack, "golem");

    assert!(state.actor_is_defeated("golem", "stamina"));
    assert_eq!(
        state.loose_room_items(LOUNGE_ID),
        vec![("herb-salve".to_string(), 2)]
    );
}

#[test]
fn conditional_drop_is_forfeited_when_its_story_var_is_truthy() {
    let mut pack = attack_action_pack();
    add_attackable_target(&mut pack, "golem", &["golem"], 1);
    let golem = pack.actors.iter_mut().find(|actor| actor.id == "golem").unwrap();
    golem.drops = BTreeMap::from([(
        "herb-salve".to_string(),
        DropSpec::Conditional(cinder_core::content::types::DropConditionSpec {
            count: 2,
            skip_when_story_var: CLEAN_RUN_FLAG.to_string(),
        }),
    )]);

    let mut state = fresh_state(&pack);
    state
        .story_vars
        .set_unchecked(CLEAN_RUN_FLAG, "true");
    drive_attack_on(&mut state, &pack, "golem");

    assert!(state.actor_is_defeated("golem", "stamina"));
    assert!(state.loose_room_items(LOUNGE_ID).is_empty());
}

#[test]
fn conditional_drop_spawns_when_its_story_var_is_absent() {
    let mut pack = attack_action_pack();
    add_attackable_target(&mut pack, "golem", &["golem"], 1);
    let golem = pack.actors.iter_mut().find(|actor| actor.id == "golem").unwrap();
    golem.drops = BTreeMap::from([(
        "herb-salve".to_string(),
        DropSpec::Conditional(cinder_core::content::types::DropConditionSpec {
            count: 2,
            skip_when_story_var: CLEAN_RUN_FLAG.to_string(),
        }),
    )]);

    let mut state = fresh_state(&pack);
    drive_attack_on(&mut state, &pack, "golem");

    assert!(state.actor_is_defeated("golem", "stamina"));
    assert_eq!(
        state.loose_room_items(LOUNGE_ID),
        vec![("herb-salve".to_string(), 2)]
    );
}

#[test]
fn player_attack_records_the_flag_for_floor_mobs_but_not_the_boss() {
    let mut pack = attack_action_pack();
    pack.hooks.insert(
        "actor.attacked".to_string(),
        json!({
            "rule": "effect_table",
            "rule_config": {
                "cases_path": "rules",
                "next_on_match": "complete",
                "next_on_default": "complete",
                "default_payload_template": {
                    "effects": []
                }
            },
            "input_overlay": {
                "rules": [
                    {
                        "conditions": [
                            {
                                "path": "tags",
                                "operator": "array_contains_any",
                                "value": ["goblin", "golem"]
                            },
                            {
                                "path": "actor_id",
                                "operator": "not_equal",
                                "value": "goblin-shaman"
                            }
                        ],
                        "payload_template": {
                            "kind": "set_story_var",
                            "key": CLEAN_RUN_FLAG,
                            "value": "true"
                        }
                    }
                ]
            }
        }),
    );
    add_attackable_target(&mut pack, "goblin-1", &["goblin"], 1);
    add_attackable_target(&mut pack, "goblin-shaman", &["goblin", "shaman"], 12);

    let mut state = fresh_state(&pack);
    assert!(!state.story_vars.has(CLEAN_RUN_FLAG));
    drive_attack_on(&mut state, &pack, "goblin-1");
    assert!(state.story_vars.has(CLEAN_RUN_FLAG), "attacking a goblin forks the flag");

    let mut boss_state = fresh_state(&pack);
    drive_attack_on(&mut boss_state, &pack, "goblin-shaman");
    assert!(
        !boss_state.story_vars.has(CLEAN_RUN_FLAG),
        "attacking the boss alone must not forfeit the clean run"
    );
}

#[test]
fn chance_spec_with_full_probability_always_scatters() {
    let drops = BTreeMap::from([(
        "leaf-blade".to_string(),
        DropSpec::Chance(DropChanceSpec {
            count: 1,
            chance_percent: 100,
        }),
    )]);
    assert_eq!(
        drive_defeat(&attack_action_pack(), drops),
        vec![("leaf-blade".to_string(), 1)]
    );
}

#[test]
fn chance_spec_with_zero_probability_never_scatters() {
    let drops = BTreeMap::from([(
        "leaf-blade".to_string(),
        DropSpec::Chance(DropChanceSpec {
            count: 1,
            chance_percent: 0,
        }),
    )]);
    assert!(drive_defeat(&attack_action_pack(), drops).is_empty());
}

#[test]
fn chance_spec_respects_the_declared_count() {
    let drops = BTreeMap::from([(
        "leaf-blade".to_string(),
        DropSpec::Chance(DropChanceSpec {
            count: 3,
            chance_percent: 100,
        }),
    )]);
    assert_eq!(
        drive_defeat(&attack_action_pack(), drops),
        vec![("leaf-blade".to_string(), 3)]
    );
}

#[test]
fn weighted_pool_scatters_exactly_one_listed_entry_per_roll() {
    let drops = BTreeMap::from([(
        "pawn-kit".to_string(),
        DropSpec::Weighted(DropPoolSpec {
            rolls: 1,
            entries: vec![
                DropPoolEntry {
                    item_id: "leaf-blade".to_string(),
                    weight: 1,
                    count: 1,
                },
                DropPoolEntry {
                    item_id: "bark-bracer".to_string(),
                    weight: 1,
                    count: 1,
                },
            ],
        }),
    )]);
    for _ in 0..20 {
        let dropped = drive_defeat(&attack_action_pack(), drops.clone());
        assert_eq!(dropped.len(), 1, "one roll picks one entry: {dropped:?}");
        let (item_id, count) = &dropped[0];
        assert_eq!(*count, 1);
        assert!(
            item_id == "leaf-blade" || item_id == "bark-bracer",
            "unexpected pool winner: {item_id}"
        );
    }
}

#[test]
fn weighted_pool_rolls_multiple_times_and_accumulates_counts() {
    let drops = BTreeMap::from([(
        "pawn-kit".to_string(),
        DropSpec::Weighted(DropPoolSpec {
            rolls: 3,
            entries: vec![DropPoolEntry {
                item_id: "leaf-blade".to_string(),
                weight: 1,
                count: 2,
            }],
        }),
    )]);
    assert_eq!(
        drive_defeat(&attack_action_pack(), drops),
        vec![("leaf-blade".to_string(), 6)]
    );
}

#[test]
fn mixed_specs_collapse_the_same_item_into_one_scatter_count() {
    let drops = BTreeMap::from([
        (
            "leaf-blade".to_string(),
            DropSpec::Always(1),
        ),
        (
            "pawn-kit".to_string(),
            DropSpec::Weighted(DropPoolSpec {
                rolls: 1,
                entries: vec![DropPoolEntry {
                    item_id: "leaf-blade".to_string(),
                    weight: 1,
                    count: 1,
                }],
            }),
        ),
    ]);
    assert_eq!(
        drive_defeat(&attack_action_pack(), drops),
        vec![("leaf-blade".to_string(), 2)]
    );
}