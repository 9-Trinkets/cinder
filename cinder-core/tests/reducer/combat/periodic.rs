use super::super::common::*;
use cinder_core::content::types::{
    CombatSettingsDefinition, DropSpec, ItemDefinition, ItemStorageTarget, LevelDefinition,
    PackMessage, PeriodicActorEffect, PeriodicActorEffectDefinition, PeriodicActorEffectTargets,
    PeriodicActorEffectTrigger, RoomDefinition, RoomExitDefinition,
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
        ..PeriodicActorEffectDefinition::default()
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
    assert_eq!(state.combat.actor_xp.get(ACTOR_A_ID), Some(&4));
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

#[test]
fn adjacent_room_emits_ambient_cue_and_suppresses_raw_damage_numbers() {
    let mut pack = reducer_test_pack();
    pack.items.push(ItemDefinition {
        id: "drain-sigil".to_string(),
        label: "drain sigil".to_string(),
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
        PackMessage::Narration("{actor} loses {damage}; {remaining} remains.".to_string()),
    );
    pack.messages.insert(
        "combat.room_hazard_adjacent".to_string(),
        PackMessage::Narration(
            "From {direction}, light flares as {item} triggers in {room}.".to_string(),
        ),
    );
    let mut goblin = test_actor("goblin", "goblin", KITCHEN_ID);
    goblin.initial_stats = BTreeMap::from([("stamina".to_string(), 10)]);
    pack.actors.push(goblin);
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    state.set_stance("goblin", ActorStance::Hostile);
    state.add_item_to_storage("drain-sigil", ItemStorageTarget::CurrentRoom, KITCHEN_ID);

    let output = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(
            WorldEvent::PeriodicActorEffectApplied {
                actor_id: "goblin".to_string(),
                effect_id: "room_hazard".to_string(),
            },
        )],
    );

    assert_eq!(state.actor_stat("goblin", "stamina"), 8);
    let text = output
        .lines
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        text.contains("From the kitchen, light flares as drain sigil triggers in Kitchen."),
        "Expected ambient cue, got: {text}"
    );
    assert!(
        !text.contains("goblin loses 2"),
        "Raw damage numbers must be suppressed offscreen: {text}"
    );
}

#[test]
fn multiple_triggers_in_same_adjacent_room_deduplicate_ambient_cue() {
    let mut pack = reducer_test_pack();
    pack.items.push(ItemDefinition {
        id: "drain-sigil".to_string(),
        label: "drain sigil".to_string(),
        ..ItemDefinition::default()
    });
    pack.settings.combat = CombatSettingsDefinition {
        player_actor_id: ACTOR_A_ID.to_string(),
        health_stat_id: "stamina".to_string(),
        ..CombatSettingsDefinition::default()
    };
    pack.settings.periodic_actor_effects = vec![periodic_damage_definition()];
    pack.messages.insert(
        "combat.room_hazard_adjacent".to_string(),
        PackMessage::Narration(
            "From {direction}, light flares as {item} triggers in {room}.".to_string(),
        ),
    );
    let mut g1 = test_actor("g1", "goblin one", KITCHEN_ID);
    g1.initial_stats = BTreeMap::from([("stamina".to_string(), 10)]);
    let mut g2 = test_actor("g2", "goblin two", KITCHEN_ID);
    g2.initial_stats = BTreeMap::from([("stamina".to_string(), 10)]);
    pack.actors.push(g1);
    pack.actors.push(g2);
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    state.set_stance("g1", ActorStance::Hostile);
    state.set_stance("g2", ActorStance::Hostile);
    state.add_item_to_storage("drain-sigil", ItemStorageTarget::CurrentRoom, KITCHEN_ID);

    let output = apply_events(
        &mut state,
        &pack,
        &[
            TimestampedWorldEvent::now(WorldEvent::PeriodicActorEffectApplied {
                actor_id: "g1".to_string(),
                effect_id: "room_hazard".to_string(),
            }),
            TimestampedWorldEvent::now(WorldEvent::PeriodicActorEffectApplied {
                actor_id: "g2".to_string(),
                effect_id: "room_hazard".to_string(),
            }),
        ],
    );

    assert_eq!(state.actor_stat("g1", "stamina"), 8);
    assert_eq!(state.actor_stat("g2", "stamina"), 8);
    let ambient_lines: Vec<_> = output
        .lines
        .iter()
        .filter(|l| l.text.contains("light flares"))
        .collect();
    assert_eq!(
        ambient_lines.len(),
        1,
        "Only one ambient cue per room should be emitted in a turn"
    );
}

#[test]
fn boss_triggers_emit_boss_cues_for_adjacent_and_distant_rooms() {
    let mut pack = reducer_test_pack();
    pack.items.push(ItemDefinition {
        id: "drain-sigil".to_string(),
        label: "drain sigil".to_string(),
        ..ItemDefinition::default()
    });
    // Add distant room "cellar" connected only to KITCHEN
    pack.rooms.push(RoomDefinition {
        id: "cellar".to_string(),
        title: "Cellar".to_string(),
        summary: String::new(),
        inspect_text: String::new(),
        allow_rest: false,
        features: vec![],
        exits: vec![RoomExitDefinition {
            room_id: KITCHEN_ID.to_string(),
            label: "Kitchen".to_string(),
            aliases: vec![],
            menu_label: None,
            requires_story_var: String::new(),
        }],
        descriptions: vec![],
    });
    // Connect KITCHEN to cellar as well
    pack.rooms[1].exits.push(RoomExitDefinition {
        room_id: "cellar".to_string(),
        label: "Cellar".to_string(),
        aliases: vec![],
        menu_label: None,
        requires_story_var: String::new(),
    });
    pack.settings.combat = CombatSettingsDefinition {
        player_actor_id: ACTOR_A_ID.to_string(),
        health_stat_id: "stamina".to_string(),
        ..CombatSettingsDefinition::default()
    };
    pack.settings.periodic_actor_effects = vec![periodic_damage_definition()];
    pack.messages.insert(
        "combat.room_hazard_adjacent_boss".to_string(),
        PackMessage::Narration(
            "A deep hiss echoes from {direction} — {actor} has triggered {item} in {room}!"
                .to_string(),
        ),
    );
    pack.messages.insert(
        "combat.room_hazard_distant_boss".to_string(),
        PackMessage::Narration(
            "A heavy shockwave shakes distant walls — {actor} has triggered {item} in {room}!"
                .to_string(),
        ),
    );
    let mut elemental = test_actor("fire-elemental", "fire elemental", KITCHEN_ID);
    elemental.initial_stats = BTreeMap::from([("stamina".to_string(), 18)]);
    pack.actors.push(elemental);
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    state.set_stance("fire-elemental", ActorStance::Hostile);
    state.add_item_to_storage("drain-sigil", ItemStorageTarget::CurrentRoom, KITCHEN_ID);
    state.add_item_to_storage("drain-sigil", ItemStorageTarget::CurrentRoom, "cellar");

    // 1. Adjacent boss trigger
    let output1 = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(
            WorldEvent::PeriodicActorEffectApplied {
                actor_id: "fire-elemental".to_string(),
                effect_id: "room_hazard".to_string(),
            },
        )],
    );
    let text1 = output1
        .lines
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        text1.contains("A deep hiss echoes from the kitchen — fire elemental has triggered drain sigil in Kitchen!"),
        "Expected adjacent boss cue, got: {text1}"
    );

    // 2. Distant boss trigger
    state.clock.turn_number += 1;
    state
        .actors
        .actor_room_overrides
        .insert("fire-elemental".to_string(), "cellar".to_string());
    let output2 = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(
            WorldEvent::PeriodicActorEffectApplied {
                actor_id: "fire-elemental".to_string(),
                effect_id: "room_hazard".to_string(),
            },
        )],
    );
    let text2 = output2
        .lines
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        text2.contains("A heavy shockwave shakes distant walls — fire elemental has triggered drain sigil in Cellar!"),
        "Expected distant boss cue, got: {text2}"
    );
}

#[test]
fn distant_minion_is_silent_but_depletion_emits_distant_cue() {
    let mut pack = reducer_test_pack();
    pack.items.push(ItemDefinition {
        id: "drain-sigil".to_string(),
        label: "drain sigil".to_string(),
        ..ItemDefinition::default()
    });
    pack.rooms.push(RoomDefinition {
        id: "cellar".to_string(),
        title: "Cellar".to_string(),
        summary: String::new(),
        inspect_text: String::new(),
        allow_rest: false,
        features: vec![],
        exits: vec![RoomExitDefinition {
            room_id: KITCHEN_ID.to_string(),
            label: "Kitchen".to_string(),
            aliases: vec![],
            menu_label: None,
            requires_story_var: String::new(),
        }],
        descriptions: vec![],
    });
    pack.rooms[1].exits.push(RoomExitDefinition {
        room_id: "cellar".to_string(),
        label: "Cellar".to_string(),
        aliases: vec![],
        menu_label: None,
        requires_story_var: String::new(),
    });
    pack.settings.combat = CombatSettingsDefinition {
        player_actor_id: ACTOR_A_ID.to_string(),
        health_stat_id: "stamina".to_string(),
        ..CombatSettingsDefinition::default()
    };
    let mut effect = periodic_damage_definition();
    effect.trigger.max_activations = Some(2);
    effect.trigger.deplete_distant_message =
        Some("combat.room_hazard_distant_depleted".to_string());
    pack.settings.periodic_actor_effects = vec![effect];
    pack.messages.insert(
        "combat.room_hazard".to_string(),
        PackMessage::Narration("goblin loses 2".to_string()),
    );
    pack.messages.insert(
        "combat.room_hazard_distant_depleted".to_string(),
        PackMessage::Narration(
            "Far off, the faint hum of spent {item} in {room} dissolves into silence.".to_string(),
        ),
    );
    let mut goblin = test_actor("goblin", "goblin", "cellar");
    goblin.initial_stats = BTreeMap::from([("stamina".to_string(), 10)]);
    pack.actors.push(goblin);
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    state.set_stance("goblin", ActorStance::Hostile);
    state.add_item_to_storage("drain-sigil", ItemStorageTarget::CurrentRoom, "cellar");

    // First activation (charge 2 -> 1): distant minion should be completely silent!
    let output1 = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(
            WorldEvent::PeriodicActorEffectApplied {
                actor_id: "goblin".to_string(),
                effect_id: "room_hazard".to_string(),
            },
        )],
    );
    assert!(
        output1.lines.is_empty(),
        "Distant minion damage should be silent, got: {:?}",
        output1.lines
    );

    // Second activation (charge 1 -> 0): depleted! Should emit distant depletion cue
    state.clock.turn_number += 1;
    let output2 = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(
            WorldEvent::PeriodicActorEffectApplied {
                actor_id: "goblin".to_string(),
                effect_id: "room_hazard".to_string(),
            },
        )],
    );
    let text2 = output2
        .lines
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        text2.contains(
            "Far off, the faint hum of spent drain sigil in Cellar dissolves into silence."
        ),
        "Expected distant depletion cue, got: {text2}"
    );
}

#[test]
fn offscreen_kills_emit_anonymous_kill_cues_and_deduplicate() {
    let mut pack = reducer_test_pack();
    pack.items.push(ItemDefinition {
        id: "drain-sigil".to_string(),
        label: "drain sigil".to_string(),
        ..ItemDefinition::default()
    });
    pack.rooms.push(RoomDefinition {
        id: "cellar".to_string(),
        title: "Cellar".to_string(),
        summary: String::new(),
        inspect_text: String::new(),
        allow_rest: false,
        features: vec![],
        exits: vec![RoomExitDefinition {
            room_id: KITCHEN_ID.to_string(),
            label: "Kitchen".to_string(),
            aliases: vec![],
            menu_label: None,
            requires_story_var: String::new(),
        }],
        descriptions: vec![],
    });
    pack.rooms[1].exits.push(RoomExitDefinition {
        room_id: "cellar".to_string(),
        label: "Cellar".to_string(),
        aliases: vec![],
        menu_label: None,
        requires_story_var: String::new(),
    });
    pack.settings.combat = CombatSettingsDefinition {
        player_actor_id: ACTOR_A_ID.to_string(),
        health_stat_id: "stamina".to_string(),
        ..CombatSettingsDefinition::default()
    };
    pack.settings.periodic_actor_effects = vec![periodic_damage_definition()];
    pack.messages.insert(
        "combat.room_hazard_adjacent_kill".to_string(),
        PackMessage::Narration(
            "From {direction}, a sharp flare of cold light rises before falling quiet — something in {room} crumbles to dust.".to_string(),
        ),
    );
    pack.messages.insert(
        "combat.room_hazard_distant_kill".to_string(),
        PackMessage::Narration(
            "Far across the halls, a distant chalk spiral runs silent as something in {room} crumbles to dust.".to_string(),
        ),
    );
    // Two goblins in kitchen at 2 HP each
    let mut g1 = test_actor("g1", "goblin one", KITCHEN_ID);
    g1.initial_stats = BTreeMap::from([("stamina".to_string(), 2)]);
    let mut g2 = test_actor("g2", "goblin two", KITCHEN_ID);
    g2.initial_stats = BTreeMap::from([("stamina".to_string(), 2)]);
    // One goblin in cellar at 2 HP
    let mut g3 = test_actor("g3", "goblin three", "cellar");
    g3.initial_stats = BTreeMap::from([("stamina".to_string(), 2)]);

    pack.actors.push(g1);
    pack.actors.push(g2);
    pack.actors.push(g3);
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    state.set_stance("g1", ActorStance::Hostile);
    state.set_stance("g2", ActorStance::Hostile);
    state.set_stance("g3", ActorStance::Hostile);
    state.add_item_to_storage("drain-sigil", ItemStorageTarget::CurrentRoom, KITCHEN_ID);
    state.add_item_to_storage("drain-sigil", ItemStorageTarget::CurrentRoom, "cellar");

    // Both g1 and g2 die in adjacent kitchen in the same turn
    let output1 = apply_events(
        &mut state,
        &pack,
        &[
            TimestampedWorldEvent::now(WorldEvent::PeriodicActorEffectApplied {
                actor_id: "g1".to_string(),
                effect_id: "room_hazard".to_string(),
            }),
            TimestampedWorldEvent::now(WorldEvent::PeriodicActorEffectApplied {
                actor_id: "g2".to_string(),
                effect_id: "room_hazard".to_string(),
            }),
        ],
    );

    assert_eq!(state.actor_stat("g1", "stamina"), 0);
    assert_eq!(state.actor_stat("g2", "stamina"), 0);

    let kill_lines: Vec<_> = output1
        .lines
        .iter()
        .filter(|l| l.text.contains("crumbles to dust"))
        .collect();
    assert_eq!(
        kill_lines.len(),
        1,
        "Multiple kills in the same room must emit exactly one anonymous kill cue: {:?}",
        output1.lines
    );
    assert!(
        kill_lines[0].text.contains("From the kitchen, a sharp flare of cold light rises before falling quiet — something in Kitchen crumbles to dust."),
        "Unexpected cue: {}",
        kill_lines[0].text
    );
    // Verify no mob name or count is leaked
    assert!(!kill_lines[0].text.contains("g1") && !kill_lines[0].text.contains("goblin"));

    // Now test distant kill in cellar
    state.clock.turn_number += 1;
    let output2 = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(
            WorldEvent::PeriodicActorEffectApplied {
                actor_id: "g3".to_string(),
                effect_id: "room_hazard".to_string(),
            },
        )],
    );
    assert_eq!(state.actor_stat("g3", "stamina"), 0);
    let distant_kills: Vec<_> = output2
        .lines
        .iter()
        .filter(|l| l.text.contains("crumbles to dust"))
        .collect();
    assert_eq!(distant_kills.len(), 1);
    assert!(
        distant_kills[0].text.contains("Far across the halls, a distant chalk spiral runs silent as something in Cellar crumbles to dust."),
        "Unexpected distant kill text: {}",
        distant_kills[0].text
    );
}

#[test]
fn exit_direction_phrase_uses_clean_direction_and_bare_room_title() {
    let mut pack = reducer_test_pack();
    pack.settings.combat = CombatSettingsDefinition {
        player_actor_id: ACTOR_A_ID.to_string(),
        health_stat_id: "stamina".to_string(),
        ..CombatSettingsDefinition::default()
    };
    pack.settings.periodic_actor_effects = vec![periodic_damage_definition()];
    pack.messages.insert(
        "combat.room_hazard_adjacent_kill".to_string(),
        PackMessage::Narration(
            "From {direction}, a sharp flare of cold light rises before falling quiet — something in the {room} crumbles to dust.".to_string(),
        ),
    );

    // Set exit label to clean cardinal direction
    if let Some(lounge) = pack.rooms.iter_mut().find(|r| r.id == LOUNGE_ID) {
        if let Some(exit) = lounge.exits.iter_mut().find(|e| e.room_id == KITCHEN_ID) {
            exit.label = "Southwest".to_string();
            exit.menu_label = Some("Southwest to Courtyard Center".to_string());
        }
    }
    // Set kitchen title to bare name without leading article
    if let Some(kitchen) = pack.rooms.iter_mut().find(|r| r.id == KITCHEN_ID) {
        kitchen.title = "Frost Courtyard".to_string();
    }

    let mut goblin = test_actor("g1", "goblin one", KITCHEN_ID);
    goblin.initial_stats = BTreeMap::from([("stamina".to_string(), 2)]);
    pack.actors.push(goblin);
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    state.set_stance("g1", ActorStance::Hostile);
    state.add_item_to_storage("drain-sigil", ItemStorageTarget::CurrentRoom, KITCHEN_ID);

    let output = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(
            WorldEvent::PeriodicActorEffectApplied {
                actor_id: "g1".to_string(),
                effect_id: "room_hazard".to_string(),
            },
        )],
    );

    let kill_lines: Vec<_> = output
        .lines
        .iter()
        .filter(|l| l.text.contains("crumbles to dust"))
        .collect();
    assert_eq!(kill_lines.len(), 1);
    assert_eq!(
        kill_lines[0].text,
        "From the southwest, a sharp flare of cold light rises before falling quiet — something in the Frost Courtyard crumbles to dust."
    );
}
