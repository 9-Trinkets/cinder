use super::common::*;
use cinder_core::content::types::{
    ActionDefinition, ActionItemCreation, ActionItemStorageTarget, ActorDefinition, CommandEffect,
    CommandTargetMode, ItemStorageTarget,
};
use cinder_core::engine::state::{ActorStance, WorldState};
use serde_json::json;

#[test]
fn encirclement_conversion_narration_follows_the_flag_placement() {
    let mut pack = reducer_test_pack();
    pack.actions.push(ActionDefinition {
        id: "place-flag".to_string(),
        command: "place-flag".to_string(),
        target_mode: CommandTargetMode::None,
        effects: vec![CommandEffect::DropItem],
        item_id: "stone-marker".to_string(),
        event_text: "{actor_name} drives a stone marker into the ground.".to_string(),
        ..ActionDefinition::default()
    });
    pack.messages.insert(
        "conversion.encircled".to_string(),
        "The {actor} turns toward you, no longer hostile.".to_string(),
    );
    pack.messages.insert(
        "conversion.encircled_follows".to_string(),
        "The {actor} falls in behind you.".to_string(),
    );
    // A golem in the lounge converts via the `actor.surrounded` hook once its
    // only neighbor (the kitchen) holds a stone marker.
    pack.actors.push(ActorDefinition {
        id: "golem".to_string(),
        name: "dark golem".to_string(),
        room_id: LOUNGE_ID.to_string(),
        ..test_actor("golem", "dark golem", LOUNGE_ID)
    });
    pack.hooks.insert(
        "actor.surrounded".to_string(),
        effect_hook(vec![json!({
            "kind": "convert_actor_to_ally",
            "actor_id": "$input.actor_id",
            "follows_player": true,
            "messages": ["conversion.encircled", "conversion.encircled_follows"],
        })]),
    );
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    state.add_item("stone-marker");
    state.add_item_to_storage("stone-marker", ItemStorageTarget::CurrentRoom, KITCHEN_ID);

    let lines = drive_actor_command(
        &mut state,
        &pack,
        "place-flag",
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
            freeform_text: None,
        },
    )
    .lines;

    let placed_at = lines
        .iter()
        .position(|line| line.text.contains("drives a stone marker"))
        .expect("flag placement narration present");
    let conversion_at = lines
        .iter()
        .position(|line| line.text.contains("turns toward you"))
        .expect("conversion narration present");
    assert!(
        placed_at < conversion_at,
        "flag placement must precede the conversion, got {lines:?}"
    );
}

#[test]
fn creating_an_item_in_a_room_can_complete_an_encirclement() {
    let mut pack = reducer_test_pack();
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
        "The {actor} turns toward you, no longer hostile.".to_string(),
    );
    // A golem in the lounge is encircled once its only neighbor (the kitchen)
    // holds a chalk marking created by the trace action.
    pack.actors.push(ActorDefinition {
        id: "golem".to_string(),
        name: "dark golem".to_string(),
        room_id: LOUNGE_ID.to_string(),
        ..test_actor("golem", "dark golem", LOUNGE_ID)
    });
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

    let lines = drive_actor_command(
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
    )
    .lines;

    assert!(state.has_item_in_storage("charm-sigil", ItemStorageTarget::CurrentRoom, KITCHEN_ID));
    assert_eq!(state.stance("golem"), ActorStance::Allied);
    assert!(
        lines
            .iter()
            .any(|line| line.text.contains("turns toward you")),
        "got: {lines:?}"
    );
}
