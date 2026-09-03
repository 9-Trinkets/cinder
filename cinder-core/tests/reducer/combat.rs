use super::common::*;
use cinder_core::content::types::{
    ActionDefinition, CombatSettingsDefinition, CommandEffect, CommandTargetMode, ItemDefinition,
    ItemStorageTarget, LevelDefinition, PeriodicActorEffect, PeriodicActorEffectDefinition,
    PeriodicActorEffectTargets, PeriodicActorEffectTrigger, StatDefinition,
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
        },
        targets: PeriodicActorEffectTargets::HostileLiving,
        effect: PeriodicActorEffect::Damage { amount: 2 },
        message: "combat.room_hazard".to_string(),
    }
}

#[test]
fn hostile_strike_uses_pack_declared_combat_vocabulary() {
    let mut pack = reducer_test_pack();
    pack.stats.actor.insert(
        "vitality".to_string(),
        StatDefinition {
            default: 10,
            ..StatDefinition::default()
        },
    );
    pack.stats
        .actor
        .insert("fury".to_string(), StatDefinition::default());
    pack.stats
        .actor
        .insert("ward".to_string(), StatDefinition::default());
    for actor in &mut pack.actors {
        actor.initial_stats.insert("vitality".to_string(), 10);
        actor.initial_stats.insert("ward".to_string(), 0);
    }
    pack.settings.combat = CombatSettingsDefinition {
        player_actor_id: ACTOR_A_NAME.to_string(),
        health_stat_id: "vitality".to_string(),
        attack_stat_id: "fury".to_string(),
        defense_stat_id: "ward".to_string(),
        minimum_damage: 2,
        default_attack_interval_minutes: 7,
        ..CombatSettingsDefinition::default()
    };
    let mut state = WorldState::new(&pack);
    state
        .actor_stats
        .entry(ACTOR_A_ID.to_string())
        .or_default()
        .insert("fury".to_string(), 5);
    state.set_stance(ACTOR_A_ID, ActorStance::Hostile);
    let events = [TimestampedWorldEvent::now(WorldEvent::HostileStrike {
        actor_id: ACTOR_A_ID.to_string(),
    })];

    apply_events(&mut state, &pack, &events);

    assert_eq!(state.actor_stat(ACTOR_A_NAME, "vitality"), 5);
    assert_eq!(
        state.next_hostile_strike_at.get(ACTOR_A_ID),
        Some(&(state.current_time_minutes + 7))
    );
}

#[test]
fn player_defeat_narration_and_phase_come_from_content() {
    let mut pack = reducer_test_pack();
    let defeat_text = "The lounge lights fade for good.";
    pack.settings.combat = CombatSettingsDefinition {
        player_actor_id: ACTOR_B_NAME.to_string(),
        health_stat_id: "stamina".to_string(),
        attack_stat_id: "confidence".to_string(),
        minimum_damage: 9,
        player_defeat_text: defeat_text.to_string(),
        ..CombatSettingsDefinition::default()
    };
    let mut state = WorldState::new(&pack);
    // Move Casey into the player's room so the strike is in range.
    state
        .actor_room_overrides
        .insert(ACTOR_C_ID.to_string(), LOUNGE_ID.to_string());
    state.set_stance(ACTOR_C_ID, ActorStance::Hostile);
    // Casey shares the lounge with the player actor (Blair).
    let events = [TimestampedWorldEvent::now(WorldEvent::HostileStrike {
        actor_id: ACTOR_C_ID.to_string(),
    })];

    let output = apply_events(&mut state, &pack, &events);

    assert_eq!(state.phase, GamePhase::GameEnded);
    assert_eq!(output.phase, GamePhase::GameEnded);
    assert!(
        output.lines.iter().any(|line| line.text == defeat_text),
        "expected pack-authored defeat line, got {:?}",
        output.lines
    );
}

#[test]
fn defeating_an_actor_scatters_its_drops_into_the_room() {
    let mut pack = equipment_test_pack();
    pack.actions.push(ActionDefinition {
        id: "attack".to_string(),
        command: "attack".to_string(),
        target_mode: CommandTargetMode::Actor,
        effects: vec![CommandEffect::AttackTarget],
        event_text: "{actor_name} strikes {target_actor_name}.".to_string(),
        ..ActionDefinition::default()
    });
    let mut golem = test_actor("golem", "dark golem", LOUNGE_ID);
    golem.attackable = true;
    golem.initial_stats = BTreeMap::from([("stamina".to_string(), 1)]);
    golem.drops = BTreeMap::from([("herb-salve".to_string(), 2)]);
    pack.actors.push(golem);
    pack.items
        .push(cinder_core::content::types::ItemDefinition {
            id: "stone-marker".to_string(),
            label: "stone marker".to_string(),
            description: "A flat stone.".to_string(),
            ..ItemDefinition::default()
        });
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();

    let lines = drive_actor_command(
        &mut state,
        &pack,
        "attack",
        ActorCommandInput {
            actor_id: ACTOR_A_ID,
            actor_name: ACTOR_A_NAME,
            room_id: LOUNGE_ID,
            target_room_id: None,
            target_actor_id: Some("golem"),
            target_actor_name: Some("dark golem"),
            context_label: None,
            feature_id: None,
            consumable_id: None,
            freeform_text: None,
        },
    )
    .lines;

    assert!(state.actor_is_defeated("golem", "stamina"));
    assert_eq!(
        state.loose_room_items(LOUNGE_ID),
        vec![("herb-salve".to_string(), 2)]
    );
    let transcript = lines
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        transcript.contains("Left behind") && transcript.contains("herb salve"),
        "got: {transcript}"
    );
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
        "{actor} loses {damage}; {remaining} remains.".to_string(),
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
    assert_eq!(output.lines.0[0].text, "goblin loses 2; 3 remains.");
}

#[test]
fn periodic_damage_at_zero_uses_normal_defeat_drop_and_xp_path() {
    let mut pack = reducer_test_pack();
    pack.items
        .push(cinder_core::content::types::ItemDefinition {
            id: "drain-sigil".to_string(),
            label: "drain sigil".to_string(),
            ..ItemDefinition::default()
        });
    pack.items
        .push(cinder_core::content::types::ItemDefinition {
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
    pack.messages
        .insert("combat.room_hazard".to_string(), String::new());
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
    goblin.drops = BTreeMap::from([("herb-salve".to_string(), 2)]);
    goblin.xp_drop = 4;
    pack.actors.push(goblin);
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    let player_stamina = state.actor_stat(ACTOR_A_ID, "stamina");
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
    assert_eq!(state.actor_stat(ACTOR_A_ID, "stamina"), player_stamina + 1);
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
fn defeating_an_actor_awards_full_xp_to_every_party_member_with_own_curve() {
    let mut pack = reducer_test_pack();
    pack.settings.combat = CombatSettingsDefinition {
        player_actor_id: ACTOR_A_ID.to_string(),
        health_stat_id: "stamina".to_string(),
        attack_stat_id: "confidence".to_string(),
        defense_stat_id: "hunger".to_string(),
        ..CombatSettingsDefinition::default()
    };
    // The default table levels anyone at 10 XP (L1 -> L2, +5 stamina). Blair
    // gets a per-actor override needing 20 XP, so she does NOT level here.
    pack.levels.default = vec![cinder_core::content::types::LevelDefinition {
        exp_required: 10,
        stat_changes: BTreeMap::from([("stamina".to_string(), 5)]),
        unlocks: vec!["power_slice".to_string()],
    }];
    pack.levels.actors = BTreeMap::from([(
        ACTOR_B_ID.to_string(),
        vec![cinder_core::content::types::LevelDefinition {
            exp_required: 20,
            stat_changes: BTreeMap::from([("stamina".to_string(), 2)]),
            unlocks: vec![],
        }],
    )]);
    pack.actions.push(ActionDefinition {
        id: "attack".to_string(),
        command: "attack".to_string(),
        target_mode: CommandTargetMode::Actor,
        effects: vec![CommandEffect::AttackTarget],
        event_text: "{actor_name} strikes {target_actor_name}.".to_string(),
        ..ActionDefinition::default()
    });
    // A one-hp, 10-xp mob in the lounge.
    let mut goblin = test_actor("goblin", "goblin", LOUNGE_ID);
    goblin.attackable = true;
    goblin.initial_stats = BTreeMap::from([("stamina".to_string(), 1)]);
    goblin.xp_drop = 10;
    pack.actors.push(goblin);
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    state.set_relationship(
        ACTOR_B_ID,
        cinder_core::engine::state::ActorRelationship {
            stance: ActorStance::Allied,
            follows_player: true,
        },
    );

    let lines = drive_actor_command(
        &mut state,
        &pack,
        "attack",
        ActorCommandInput {
            actor_id: ACTOR_A_ID,
            actor_name: ACTOR_A_NAME,
            room_id: LOUNGE_ID,
            target_room_id: None,
            target_actor_id: Some("goblin"),
            target_actor_name: Some("goblin"),
            context_label: None,
            feature_id: None,
            consumable_id: None,
            freeform_text: None,
        },
    )
    .lines;

    // Full 10 XP goes to the player (leveled 1->2 on the default curve, +5
    // stamina) and to follower Blair — who keeps it on her own 20-XP curve,
    // so she stays level 1 with 10/20 and no stat bonus.
    assert!(state.actor_is_defeated("goblin", "stamina"));
    assert_eq!(state.actor_xp(ACTOR_A_ID), 0);
    assert_eq!(state.actor_level(ACTOR_A_ID), 2);
    assert_eq!(state.actor_xp(ACTOR_B_ID), 10);
    assert_eq!(state.actor_level(ACTOR_B_ID), 1);
    // Narration is per actor: only the leveling actor is named with its own
    // new level.
    let transcripts = lines
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>();
    assert!(
        transcripts
            .iter()
            .any(|t| t.contains(ACTOR_A_NAME) && t.contains("Level 2")),
        "expected per-actor narration naming Alex at Level 2, got: {transcripts:?}"
    );
    assert!(
        !transcripts
            .iter()
            .any(|t| t.contains(ACTOR_B_NAME) && t.contains("Level 2")),
        "Blair did not level; she should not be narrated, got: {transcripts:?}"
    );
    let alex_stamina = state.actor_stat(ACTOR_A_ID, "stamina");
    let blair_stamina = state.actor_stat(ACTOR_B_ID, "stamina");
    assert_eq!(
        alex_stamina,
        blair_stamina + 5,
        "Alex should be +5 stamina; Alex={alex_stamina}, Blair={blair_stamina}"
    );
}
