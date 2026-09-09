use super::common::*;
use cinder_core::content::types::{
    ActionDefinition, ActionItemCreation, ActionItemStorageTarget, CommandTargetMode,
    CombatSettingsDefinition, ItemDefinition, ItemStorageTarget, PackMessage, PeriodicActorEffect,
    PeriodicActorEffectDefinition, PeriodicActorEffectTargets, PeriodicActorEffectTrigger,
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
    assert!(state.has_item_in_storage(
        "drain-sigil",
        ItemStorageTarget::CurrentRoom,
        LOUNGE_ID
    ));

    // The fifth application spends the sigil and narrates its fading.
    let spent = apply_events(&mut state, &pack, &[drain_application("goblin")]);
    assert_eq!(state.actor_stat("goblin", "stamina"), 5);
    assert!(!state.has_item_in_storage(
        "drain-sigil",
        ItemStorageTarget::CurrentRoom,
        LOUNGE_ID
    ));
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
    assert!(state.has_item_in_storage(
        "drain-sigil",
        ItemStorageTarget::CurrentRoom,
        LOUNGE_ID
    ));
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
    assert!(!state.has_item_in_storage(
        "charm-sigil",
        ItemStorageTarget::CurrentRoom,
        KITCHEN_ID
    ));
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