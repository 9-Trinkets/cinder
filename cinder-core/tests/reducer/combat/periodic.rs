use super::super::common::*;
use cinder_core::content::types::{
    CombatSettingsDefinition, DropSpec, ItemDefinition, ItemStorageTarget, LevelDefinition,
    PackMessage, PeriodicActorEffect, PeriodicActorEffectDefinition,
    PeriodicActorEffectTargets, PeriodicActorEffectTrigger,
};
use cinder_core::engine::events::{TimestampedWorldEvent, WorldEvent};
use cinder_core::engine::reducer::apply_events;
use cinder_core::engine::state::{ActorStance, GamePhase, WorldState};
use std::collections::BTreeMap;

fn periodic_damage_definition() -> PeriodicActorEffectDefinition {
    PeriodicActorEffectDefinition {
        id: "room_hazard".to_string(),
        trigger: PeriodicActorEffectTrigger {
            room_item: "drain-sigil".to_string(),
            ..PeriodicActorEffectTrigger::default()
        },
        targets: PeriodicActorEffectTargets::HostileLiving,
        effect: PeriodicActorEffect::Damage { amount: 2 },
        message: "combat.room_hazard".to_string(),
    }
}

#[test]
fn configured_periodic_damage_affects_only_matching_actors() {
    let mut pack = reducer_test_pack();
    pack.settings.combat = CombatSettingsDefinition {
        player_actor_id: ACTOR_A_ID.to_string(),
        health_stat_id: "stamina".to_string(),
        ..CombatSettingsDefinition::default()
    };
    pack.settings.periodic_actor_effects = vec![periodic_damage_definition()];
    pack.messages.insert(
        "combat.room_hazard".to_string(),
        PackMessage::Narration("{actor} loses {damage}; {remaining} remains.".to_string()),
    );
    let mut goblin = test_actor("goblin", "goblin", LOUNGE_ID);
    goblin.initial_stats = BTreeMap::from([("stamina".to_string(), 5)]);
    pack.actors.push(goblin);
    let mut quiet = test_actor("quiet", "quiet goblin", KITCHEN_ID);
    quiet.initial_stats = BTreeMap::from([("stamina".to_string(), 5)]);
    pack.actors.push(quiet);
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    state.set_stance("goblin", ActorStance::Hostile);
    state.set_stance("quiet", ActorStance::Hostile);
    state.add_item_to_storage("drain-sigil", ItemStorageTarget::CurrentRoom, LOUNGE_ID);

    let events = [
        TimestampedWorldEvent::now(WorldEvent::PeriodicActorEffectApplied {
            actor_id: "goblin".to_string(),
            effect_id: "room_hazard".to_string(),
        }),
        TimestampedWorldEvent::now(WorldEvent::PeriodicActorEffectApplied {
            actor_id: "quiet".to_string(),
            effect_id: "room_hazard".to_string(),
        }),
    ];
    let output = apply_events(&mut state, &pack, &events);

    assert_eq!(state.actor_stat("goblin", "stamina"), 3);
    assert_eq!(state.actor_stat("quiet", "stamina"), 5);
    assert_eq!(output.lines[0].text, "goblin loses 2; 3 remains.");
}

#[test]
fn periodic_damage_at_zero_uses_normal_defeat_drop_and_xp_path() {
    let mut pack = reducer_test_pack();
    pack.items.push(ItemDefinition {
        id: "drain-sigil".to_string(),
        label: "drain sigil".to_string(),
        ..ItemDefinition::default()
    });
    pack.items.push(ItemDefinition {
        id: "herb-salve".to_string(),
        label: "herb salve".to_string(),
        ..ItemDefinition::default()
    });
    pack.settings.combat = CombatSettingsDefinition {
        player_actor_id: ACTOR_A_ID.to_string(),
        health_stat_id: "stamina".to_string(),
        ..CombatSettingsDefinition::default()
    };
    pack.settings.periodic_actor_effects = vec![periodic_damage_definition()];
    pack.messages.insert(
        "combat.room_hazard".to_string(),
        PackMessage::Narration(String::new()),
    );
    pack.levels.default = vec![LevelDefinition {
        exp_required: 10,
        ..LevelDefinition::default()
    }];
    pack.hooks.insert(
        cinder_core::engine::hook_ids::ACTOR_DEFEATED.to_string(),
        effect_hook(vec![serde_json::json!({
            "kind": "adjust_actor_stat",
            "actor_id": ACTOR_A_ID,
            "stat": "stamina",
            "delta": 1
        })]),
    );
    let mut goblin = test_actor("goblin", "goblin", LOUNGE_ID);
    goblin.initial_stats = BTreeMap::from([("stamina".to_string(), 1)]);
    goblin.drops = BTreeMap::from([("herb-salve".to_string(), DropSpec::Always(2))]);
    goblin.xp_drop = 4;
    pack.actors.push(goblin);
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    state.set_stance("goblin", ActorStance::Hostile);
    state.add_item_to_storage("drain-sigil", ItemStorageTarget::CurrentRoom, LOUNGE_ID);

    let events = [TimestampedWorldEvent::now(
        WorldEvent::PeriodicActorEffectApplied {
            actor_id: "goblin".to_string(),
            effect_id: "room_hazard".to_string(),
        },
    )];
    apply_events(&mut state, &pack, &events);

    assert!(state.actor_is_defeated("goblin", "stamina"));
    assert_eq!(
        state.loose_room_items(LOUNGE_ID),
        vec![
            ("drain-sigil".to_string(), 1),
            ("herb-salve".to_string(), 2)
        ]
    );
    assert_eq!(state.actor_xp.get(ACTOR_A_ID), Some(&4));
    assert_eq!(
        state.actor_stat(ACTOR_A_ID, "stamina"),
        state.actor_stat_maximum(&pack, ACTOR_A_ID, "stamina")
    );
}

#[test]
fn periodic_damage_uses_player_defeat_path_for_the_player_actor() {
    let mut pack = reducer_test_pack();
    pack.settings.combat.player_actor_id = ACTOR_A_ID.to_string();
    pack.settings.combat.health_stat_id = "stamina".to_string();
    let mut effect = periodic_damage_definition();
    effect.targets = PeriodicActorEffectTargets::AnyLiving;
    effect.effect = PeriodicActorEffect::Damage { amount: 99 };
    pack.settings.periodic_actor_effects = vec![effect];
    pack.items.push(ItemDefinition {
        id: "drain-sigil".to_string(),
        label: "drain sigil".to_string(),
        ..ItemDefinition::default()
    });
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.add_item_to_storage("drain-sigil", ItemStorageTarget::CurrentRoom, LOUNGE_ID);

    let output = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(
            WorldEvent::PeriodicActorEffectApplied {
                actor_id: ACTOR_A_ID.to_string(),
                effect_id: "room_hazard".to_string(),
            },
        )],
    );

    assert_eq!(state.phase, GamePhase::GameEnded);
    assert_eq!(output.phase, GamePhase::GameEnded);
}

#[test]
fn drain_damage_bypasses_physical_resistance() {
    let mut pack = reducer_test_pack();
    pack.settings.combat = CombatSettingsDefinition {
        player_actor_id: ACTOR_A_ID.to_string(),
        health_stat_id: "stamina".to_string(),
        ..CombatSettingsDefinition::default()
    };
    pack.settings.periodic_actor_effects = vec![periodic_damage_definition()];
    pack.messages.insert(
        "combat.room_hazard".to_string(),
        PackMessage::Narration("{actor} loses {damage}; {remaining} remains.".to_string()),
    );
    let mut elemental = test_actor("elemental", "fire elemental", LOUNGE_ID);
    elemental.initial_stats = BTreeMap::from([("stamina".to_string(), 5)]);
    elemental.resistances = BTreeMap::from([("physical".to_string(), 999)]);
    pack.actors.push(elemental);
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    state.set_stance("elemental", ActorStance::Hostile);
    state.add_item_to_storage("drain-sigil", ItemStorageTarget::CurrentRoom, LOUNGE_ID);

    apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(
            WorldEvent::PeriodicActorEffectApplied {
                actor_id: "elemental".to_string(),
                effect_id: "room_hazard".to_string(),
            },
        )],
    );

    assert_eq!(
        state.actor_stat("elemental", "stamina"),
        3,
        "drain damage ignores physical resistance"
    );
}