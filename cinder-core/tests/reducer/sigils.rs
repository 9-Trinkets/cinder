use super::common::*;
use cinder_core::content::types::{
    ActionDefinition, ActionItemCreation, ActionItemStorageTarget, CombatSettingsDefinition,
    CommandTargetMode, ItemDefinition, ItemStorageTarget, PackMessage, PeriodicActorEffect,
    PeriodicActorEffectDefinition, PeriodicActorEffectTargets, PeriodicActorEffectTrigger,
    StatDefinition,
};
use cinder_core::engine::events::{TimestampedWorldEvent, WorldEvent};
use cinder_core::engine::reducer::apply_events;
use cinder_core::engine::state::{ActorStance, WorldState};
use serde_json::json;
use std::collections::BTreeMap;

fn drain_effect(max_activations: Option<u32>) -> PeriodicActorEffectDefinition {
    PeriodicActorEffectDefinition {
        id: "room_hazard".to_string(),
        trigger: PeriodicActorEffectTrigger {
            room_item: "drain-sigil".to_string(),
            max_activations,
            deplete_message: Some("combat.room_hazard_spent".to_string()),
        },
        targets: PeriodicActorEffectTargets::HostileLiving,
        effect: PeriodicActorEffect::Damage { amount: 2 },
        message: "combat.room_hazard".to_string(),
    }
}

fn drain_application(actor_id: &str) -> TimestampedWorldEvent {
    TimestampedWorldEvent::now(WorldEvent::PeriodicActorEffectApplied {
        actor_id: actor_id.to_string(),
        effect_id: "room_hazard".to_string(),
    })
}

#[test]
fn drain_sigil_has_fixed_activations_and_fades_when_spent() {
    let mut pack = reducer_test_pack();
    pack.settings.combat = CombatSettingsDefinition {
        player_actor_id: ACTOR_A_ID.to_string(),
        health_stat_id: "stamina".to_string(),
        ..CombatSettingsDefinition::default()
    };
    pack.settings.periodic_actor_effects = vec![drain_effect(Some(5))];
    pack.messages.insert(
        "combat.room_hazard".to_string(),
        PackMessage::Narration("{actor} loses {damage}; {remaining} remains.".to_string()),
    );
    pack.messages.insert(
        "combat.room_hazard_spent".to_string(),
        PackMessage::Narration("The spiral gutters out.".to_string()),
    );
    let mut goblin = test_actor("goblin", "goblin", LOUNGE_ID);
    goblin.initial_stats = BTreeMap::from([("stamina".to_string(), 15)]);
    pack.actors.push(goblin);
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    state.set_stance("goblin", ActorStance::Hostile);
    state.add_item_to_storage("drain-sigil", ItemStorageTarget::CurrentRoom, LOUNGE_ID);

    for _ in 0..4 {
        apply_events(&mut state, &pack, &[drain_application("goblin")]);
    }
    assert_eq!(state.actor_stat("goblin", "stamina"), 7);
    assert!(state.has_item_in_storage("drain-sigil", ItemStorageTarget::CurrentRoom, LOUNGE_ID));

    // The fifth application spends the sigil and narrates its fading.
    let spent = apply_events(&mut state, &pack, &[drain_application("goblin")]);
    assert_eq!(state.actor_stat("goblin", "stamina"), 5);
    assert!(!state.has_item_in_storage("drain-sigil", ItemStorageTarget::CurrentRoom, LOUNGE_ID));
    assert!(
        spent
            .lines
            .0
            .iter()
            .any(|line| line.text.contains("gutters out")),
        "expected the deplete line, got: {:?}",
        spent.lines.0
    );

    // With no sigil left the effect cannot fire again.
    apply_events(&mut state, &pack, &[drain_application("goblin")]);
    assert_eq!(state.actor_stat("goblin", "stamina"), 5);
}

#[test]
fn drain_sigil_without_a_charge_limit_never_fades() {
    let mut pack = reducer_test_pack();
    pack.settings.combat = CombatSettingsDefinition {
        player_actor_id: ACTOR_A_ID.to_string(),
        health_stat_id: "stamina".to_string(),
        ..CombatSettingsDefinition::default()
    };
    pack.settings.periodic_actor_effects = vec![drain_effect(None)];
    pack.messages.insert(
        "combat.room_hazard".to_string(),
        PackMessage::Narration("{actor} loses {damage}; {remaining} remains.".to_string()),
    );
    let mut goblin = test_actor("goblin", "goblin", LOUNGE_ID);
    goblin.initial_stats = BTreeMap::from([("stamina".to_string(), 15)]);
    pack.actors.push(goblin);
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    state.set_stance("goblin", ActorStance::Hostile);
    state.add_item_to_storage("drain-sigil", ItemStorageTarget::CurrentRoom, LOUNGE_ID);

    for _ in 0..10 {
        apply_events(&mut state, &pack, &[drain_application("goblin")]);
    }
    assert!(state.has_item_in_storage("drain-sigil", ItemStorageTarget::CurrentRoom, LOUNGE_ID));
}

#[test]
fn charm_sigil_converts_a_single_actor_and_is_consumed() {
    let mut pack = reducer_test_pack();
    pack.settings.combat.player_actor_id = ACTOR_A_ID.to_string();
    pack.items.push(ItemDefinition {
        id: "charm-sigil".to_string(),
        label: "charm sigil".to_string(),
        description: "A chalk ring.".to_string(),
        trace_mark: true,
        consumed_on_surround_conversion: true,
        ..ItemDefinition::default()
    });
    pack.actions.push(ActionDefinition {
        id: "trace".to_string(),
        command: "trace".to_string(),
        target_mode: CommandTargetMode::None,
        item_creation: Some(ActionItemCreation {
            creates_item: "charm-sigil".to_string(),
            storage: ActionItemStorageTarget::CurrentRoom,
            ..ActionItemCreation::default()
        }),
        event_text: "{actor_name} draws chalk across the floor.".to_string(),
        ..ActionDefinition::default()
    });
    pack.messages.insert(
        "conversion.encircled".to_string(),
        PackMessage::Narration("The {actor} turns toward you, no longer hostile.".to_string()),
    );
    // Both golems share the lounge; its only neighbor (the kitchen) fills with
    // the freshly traced charm, so either would convert from the same placement.
    pack.actors = vec![
        test_actor(ACTOR_A_ID, ACTOR_A_NAME, LOUNGE_ID),
        test_actor("golem-1", "first golem", LOUNGE_ID),
        test_actor("golem-2", "second golem", LOUNGE_ID),
    ];
    pack.hooks.insert(
        "actor.surrounded".to_string(),
        effect_hook(vec![json!({
            "kind": "convert_actor_to_ally",
            "actor_id": "$input.actor_id",
            "follows_player": true,
            "messages": ["conversion.encircled"],
        })]),
    );
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.current_room_id = KITCHEN_ID.to_string();

    let output = drive_actor_command(
        &mut state,
        &pack,
        "trace",
        ActorCommandInput {
            actor_id: ACTOR_A_ID,
            actor_name: ACTOR_A_NAME,
            room_id: KITCHEN_ID,
            target_room_id: None,
            target_actor_id: None,
            target_actor_name: None,
            context_label: None,
            feature_id: None,
            consumable_id: None,
            freeform_text: None,
        },
    );

    assert_eq!(state.stance("golem-1"), ActorStance::Allied);
    assert_eq!(state.stance("golem-2"), ActorStance::Neutral);
    assert!(!state.has_item_in_storage("charm-sigil", ItemStorageTarget::CurrentRoom, KITCHEN_ID));
    assert!(
        output
            .lines
            .0
            .iter()
            .any(|line| line.text.contains("turns toward you")),
        "got: {:?}",
        output.lines.0
    );
}

#[test]
fn spawn_sigil_scales_with_intelligence_and_spawns_allied_follower() {
    let mut pack = reducer_test_pack();
    pack.settings.combat.player_actor_id = ACTOR_A_ID.to_string();
    pack.settings.combat.health_stat_id = "stamina".to_string();
    pack.stats.actor.insert(
        "mp".to_string(),
        StatDefinition {
            default: 10,
            min: Some(0),
            max: Some(20),
            time_step_minutes: Some(2),
        },
    );
    pack.stats.actor.insert(
        "intelligence".to_string(),
        StatDefinition {
            default: 6,
            min: Some(0),
            max: Some(20),
            time_step_minutes: None,
        },
    );
    pack.items.push(ItemDefinition {
        id: "spawn-sigil".to_string(),
        label: "spawn sigil".to_string(),
        description: "An amber sigil.".to_string(),
        trace_mark: true,
        mp_cost: 5,
        max_active_instances: Some(2),
        spawn_template_id: "summoned-fire-sprite".to_string(),
        placement_hook: "item.spawn_sigil_placed".to_string(),
        consumed_on_placement: true,
        ..ItemDefinition::default()
    });
    pack.actions.push(ActionDefinition {
        id: "trace".to_string(),
        command: "trace".to_string(),
        target_mode: CommandTargetMode::None,
        item_creation: Some(ActionItemCreation {
            creates_item: "spawn-sigil".to_string(),
            craftable_items: vec!["spawn-sigil".to_string()],
            storage: ActionItemStorageTarget::CurrentRoom,
            ..ActionItemCreation::default()
        }),
        event_text: "{actor_name} traces a sigil.".to_string(),
        ..ActionDefinition::default()
    });
    let mut sprite = test_actor("summoned-fire-sprite", "fire sprite", "");
    sprite.tags = vec!["sprite".to_string(), "summoned".to_string()];
    sprite.initial_stats = BTreeMap::from([
        ("stamina".to_string(), 4),
        ("strength".to_string(), 2),
        ("intelligence".to_string(), 2),
    ]);
    sprite.attackable = true;
    pack.actors.push(sprite);
    pack.messages.insert(
        "sigil.spawned".to_string(),
        PackMessage::Narration(
            "A {actor} rises! HP {hp}, STR {strength}, INT {intelligence}.".to_string(),
        ),
    );
    pack.hooks.insert(
        "item.spawn_sigil_placed".to_string(),
        effect_hook(vec![json!({
            "kind": "spawn_actor",
            "template_id": "summoned-fire-sprite",
            "stance": "allied",
            "follows_player": true,
            "scale_with_actor_id": ACTOR_A_ID,
            "scale_stat": "intelligence",
            "max_active_instances": 2,
            "messages": ["sigil.spawned"],
        })]),
    );
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    state
        .adjust_actor_stat(&pack, ACTOR_A_ID, "intelligence", 3)
        .unwrap(); // 6 + 3 = 9 INT
    assert_eq!(state.actor_stat(ACTOR_A_ID, "mp"), 10);

    let output = drive_actor_command(
        &mut state,
        &pack,
        "trace",
        ActorCommandInput {
            actor_id: ACTOR_A_ID,
            actor_name: ACTOR_A_NAME,
            room_id: LOUNGE_ID,
            target_room_id: None,
            target_actor_id: None,
            target_actor_name: None,
            context_label: None,
            feature_id: None,
            consumable_id: None,
            freeform_text: Some("spawn-sigil"),
        },
    );

    // 1. MP deducted: 10 - 5 = 5 MP
    assert_eq!(state.actor_stat(ACTOR_A_ID, "mp"), 5);

    // 2. Sigil consumed on placement
    assert!(!state.has_item_in_storage("spawn-sigil", ItemStorageTarget::CurrentRoom, LOUNGE_ID));

    // 3. Spawned actor exists with scaled stats:
    // Scaler = 9 INT
    // HP: 4 + 9 = 13
    // STR: 2 + 9 / 3 = 5
    // INT: 2 + 9 / 2 = 6
    let instance_id = "summoned-fire-sprite-1";
    assert_eq!(state.stance(instance_id), ActorStance::Allied);
    assert!(state.relationship(instance_id).follows_player);
    assert_eq!(state.actor_stat(instance_id, "stamina"), 13);
    assert_eq!(state.actor_stat(instance_id, "strength"), 5);
    assert_eq!(state.actor_stat(instance_id, "intelligence"), 6);

    // 4. Narration matches
    assert!(
        output
            .lines
            .0
            .iter()
            .any(|l| l.text.contains("HP 13, STR 5, INT 6")),
        "got: {:?}",
        output.lines.0
    );
}

#[test]
fn spawn_sigil_enforces_limit_of_two_and_rejects_without_spending_mp() {
    let mut pack = reducer_test_pack();
    pack.settings.combat.player_actor_id = ACTOR_A_ID.to_string();
    pack.stats.actor.insert(
        "mp".to_string(),
        StatDefinition {
            default: 15,
            min: Some(0),
            max: Some(20),
            time_step_minutes: Some(2),
        },
    );
    pack.items.push(ItemDefinition {
        id: "spawn-sigil".to_string(),
        label: "spawn sigil".to_string(),
        description: "An amber sigil.".to_string(),
        trace_mark: true,
        mp_cost: 5,
        max_active_instances: Some(2),
        spawn_template_id: "summoned-fire-sprite".to_string(),
        placement_hook: "item.spawn_sigil_placed".to_string(),
        consumed_on_placement: true,
        ..ItemDefinition::default()
    });
    pack.actions.push(ActionDefinition {
        id: "trace".to_string(),
        command: "trace".to_string(),
        target_mode: CommandTargetMode::None,
        item_creation: Some(ActionItemCreation {
            creates_item: "spawn-sigil".to_string(),
            craftable_items: vec!["spawn-sigil".to_string()],
            storage: ActionItemStorageTarget::CurrentRoom,
            ..ActionItemCreation::default()
        }),
        event_text: "{actor_name} traces a sigil.".to_string(),
        ..ActionDefinition::default()
    });
    let mut sprite = test_actor("summoned-fire-sprite", "fire sprite", "");
    sprite.tags = vec!["sprite".to_string(), "summoned".to_string()];
    sprite.initial_stats = BTreeMap::from([("stamina".to_string(), 4)]);
    sprite.attackable = true;
    pack.actors.push(sprite);
    pack.messages.insert(
        "sigil.spawned".to_string(),
        PackMessage::Narration("A {actor} rises!".to_string()),
    );
    pack.messages.insert(
        "sigil.spawn_limit".to_string(),
        PackMessage::Narration("Limit of {max} sprites reached.".to_string()),
    );
    pack.hooks.insert(
        "item.spawn_sigil_placed".to_string(),
        effect_hook(vec![json!({
            "kind": "spawn_actor",
            "template_id": "summoned-fire-sprite",
            "stance": "allied",
            "follows_player": true,
            "max_active_instances": 2,
            "max_instances_messages": ["sigil.spawn_limit"],
            "messages": ["sigil.spawned"],
        })]),
    );
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();

    // Spawn 1
    let out1 = drive_actor_command(
        &mut state,
        &pack,
        "trace",
        ActorCommandInput {
            actor_id: ACTOR_A_ID,
            actor_name: ACTOR_A_NAME,
            room_id: LOUNGE_ID,
            freeform_text: Some("spawn-sigil"),
            ..ActorCommandInput::default()
        },
    );
    assert!(
        out1.lines
            .0
            .iter()
            .any(|l| l.text.contains("A fire sprite rises!"))
    );
    assert_eq!(state.actor_stat(ACTOR_A_ID, "mp"), 10);
    assert_eq!(
        state.active_spawned_actor_count(&pack, "summoned-fire-sprite"),
        1
    );

    // Spawn 2
    let out2 = drive_actor_command(
        &mut state,
        &pack,
        "trace",
        ActorCommandInput {
            actor_id: ACTOR_A_ID,
            actor_name: ACTOR_A_NAME,
            room_id: LOUNGE_ID,
            freeform_text: Some("spawn-sigil"),
            ..ActorCommandInput::default()
        },
    );
    assert!(
        out2.lines
            .0
            .iter()
            .any(|l| l.text.contains("A fire sprite rises!"))
    );
    assert_eq!(state.actor_stat(ACTOR_A_ID, "mp"), 5);
    assert_eq!(
        state.active_spawned_actor_count(&pack, "summoned-fire-sprite"),
        2
    );

    // Spawn 3 -> should be rejected!
    let out3 = drive_actor_command(
        &mut state,
        &pack,
        "trace",
        ActorCommandInput {
            actor_id: ACTOR_A_ID,
            actor_name: ACTOR_A_NAME,
            room_id: LOUNGE_ID,
            freeform_text: Some("spawn-sigil"),
            ..ActorCommandInput::default()
        },
    );
    // MP NOT deducted
    assert_eq!(state.actor_stat(ACTOR_A_ID, "mp"), 5);
    // Count remains 2
    assert_eq!(
        state.active_spawned_actor_count(&pack, "summoned-fire-sprite"),
        2
    );
    // Sigil not in room
    assert!(!state.has_item_in_storage("spawn-sigil", ItemStorageTarget::CurrentRoom, LOUNGE_ID));
    // Narration warns of limit
    assert!(
        out3.lines
            .0
            .iter()
            .any(|l| l.text.contains("Limit of 2 sprites reached")),
        "got: {:?}",
        out3.lines.0
    );
}

#[test]
fn tracing_sigil_rejects_when_mp_is_insufficient() {
    let mut pack = reducer_test_pack();
    pack.settings.combat.player_actor_id = ACTOR_A_ID.to_string();
    pack.stats.actor.insert(
        "mp".to_string(),
        StatDefinition {
            default: 3, // only 3 MP, needs 5
            min: Some(0),
            max: Some(20),
            time_step_minutes: Some(2),
        },
    );
    pack.items.push(ItemDefinition {
        id: "spawn-sigil".to_string(),
        label: "spawn sigil".to_string(),
        description: "An amber sigil.".to_string(),
        trace_mark: true,
        mp_cost: 5,
        ..ItemDefinition::default()
    });
    pack.actions.push(ActionDefinition {
        id: "trace".to_string(),
        command: "trace".to_string(),
        target_mode: CommandTargetMode::None,
        item_creation: Some(ActionItemCreation {
            creates_item: "spawn-sigil".to_string(),
            craftable_items: vec!["spawn-sigil".to_string()],
            storage: ActionItemStorageTarget::CurrentRoom,
            ..ActionItemCreation::default()
        }),
        event_text: "{actor_name} traces a sigil.".to_string(),
        ..ActionDefinition::default()
    });
    pack.messages.insert(
        "magic.insufficient_mp".to_string(),
        PackMessage::Narration(
            "Not enough MP to trace {item}: need {mp_cost}, have {mp}.".to_string(),
        ),
    );
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();

    let output = drive_actor_command(
        &mut state,
        &pack,
        "trace",
        ActorCommandInput {
            actor_id: ACTOR_A_ID,
            actor_name: ACTOR_A_NAME,
            room_id: LOUNGE_ID,
            freeform_text: Some("spawn-sigil"),
            ..ActorCommandInput::default()
        },
    );

    assert_eq!(state.actor_stat(ACTOR_A_ID, "mp"), 3);
    assert!(!state.has_item_in_storage("spawn-sigil", ItemStorageTarget::CurrentRoom, LOUNGE_ID));
    assert!(
        output
            .lines
            .0
            .iter()
            .any(|l| l.text.contains("need 5, have 3")),
        "got: {:?}",
        output.lines.0
    );
}

#[test]
fn mp_regenerates_on_tick_interval_and_clamps_at_max() {
    let mut pack = reducer_test_pack();
    pack.settings.combat.player_actor_id = ACTOR_A_ID.to_string();
    if let Some(hunger) = pack.stats.actor.get_mut("hunger") {
        hunger.time_step_minutes = None;
    }
    pack.stats.actor.insert(
        "mp".to_string(),
        StatDefinition {
            default: 10,
            min: Some(0),
            max: Some(20),
            time_step_minutes: Some(2),
        },
    );
    pack.hooks.insert(
        "actor.time_advanced".to_string(),
        effect_hook(vec![json!({
            "kind": "adjust_actor_stat",
            "actor_id": "$input.actor_id",
            "stat": "mp",
            "delta": 1,
        })]),
    );
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.current_time_minutes = 0;
    state
        .adjust_actor_stat(&pack, ACTOR_A_ID, "mp", -3)
        .unwrap(); // 10 - 3 = 7 MP
    assert_eq!(state.actor_stat(ACTOR_A_ID, "mp"), 7);

    // Turn 1 (0 -> 1 min): no 2-minute interval crossed
    apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::TurnStarted {
            turn_number: 1,
            raw_input: "tick".to_string(),
            advances_time: true,
        })],
    );
    assert_eq!(state.actor_stat(ACTOR_A_ID, "mp"), 7);

    // Turn 2 (1 -> 2 min): 2-minute interval crossed! +1 MP -> 8 MP
    apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::TurnStarted {
            turn_number: 2,
            raw_input: "tick".to_string(),
            advances_time: true,
        })],
    );
    assert_eq!(state.actor_stat(ACTOR_A_ID, "mp"), 8);

    // Turns 3 and 4 (2 -> 4 min): another 2-minute interval crossed -> +1 MP -> 9 MP
    for t in 3..=4 {
        apply_events(
            &mut state,
            &pack,
            &[TimestampedWorldEvent::now(WorldEvent::TurnStarted {
                turn_number: t,
                raw_input: "tick".to_string(),
                advances_time: true,
            })],
        );
    }
    assert_eq!(state.actor_stat(ACTOR_A_ID, "mp"), 9);

    // Turns 5 and 6 (4 -> 6 min): +1 MP -> 10 MP (reaches natural maximum)
    for t in 5..=6 {
        apply_events(
            &mut state,
            &pack,
            &[TimestampedWorldEvent::now(WorldEvent::TurnStarted {
                turn_number: t,
                raw_input: "tick".to_string(),
                advances_time: true,
            })],
        );
    }
    assert_eq!(state.actor_stat(ACTOR_A_ID, "mp"), 10);

    // Turns 7 through 12 (6 -> 12 min): should clamp at 10 MP!
    for t in 7..=12 {
        apply_events(
            &mut state,
            &pack,
            &[TimestampedWorldEvent::now(WorldEvent::TurnStarted {
                turn_number: t,
                raw_input: "tick".to_string(),
                advances_time: true,
            })],
        );
    }
    assert_eq!(state.actor_stat(ACTOR_A_ID, "mp"), 10);
}
