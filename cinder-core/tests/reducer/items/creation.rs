//! Action-driven item creation tests: story-var recipes, target-derived
//! templates, and the trace craftable gating.

use super::super::common::*;
use super::plain_command;
use cinder_core::content::types::{
    ActionDefinition, ActionItemCreation, ActionItemStorageTarget, CommandEffect,
    CommandTargetMode, ItemDefinition, ItemStorageTarget,
};
use cinder_core::engine::events::{TimestampedWorldEvent, WorldEvent};
use cinder_core::engine::reducer::apply_events;
use cinder_core::engine::state::WorldState;
use std::collections::BTreeMap;

#[test]
fn actor_commands_can_create_room_items_from_story_vars() {
    let mut pack = reducer_test_pack();
    pack.items = vec![
        ItemDefinition {
            id: "garlic-noodles".to_string(),
            label: "garlic noodles".to_string(),
            description: "Noodles.".to_string(),
            ..ItemDefinition::default()
        },
        ItemDefinition {
            id: "vegetable-stir-fry".to_string(),
            label: "vegetable stir-fry".to_string(),
            description: "Stir-fry.".to_string(),
            ..ItemDefinition::default()
        },
    ];
    pack.actions.push(ActionDefinition {
        id: "cook".to_string(),
        command: "COOK".to_string(),
        effects: vec![CommandEffect::RememberInRoom],
        event_text: "{actor_name} finishes dinner.".to_string(),
        item_creation: Some(ActionItemCreation {
            creates_item: "garlic-noodles".to_string(),
            creates_item_story_var: "cook_recipe".to_string(),
            creates_item_target_template: String::new(),
            craftable_items: Vec::new(),
            craftable_item_gates: BTreeMap::new(),
            storage: ActionItemStorageTarget::CurrentRoom,
        }),
        ..ActionDefinition::default()
    });
    rebuild_test_pack_indexes(&mut pack);
    let mut state = WorldState::new(&pack);
    state.current_room_id = KITCHEN_ID.to_string();
    state
        .story_vars
        .set_unchecked("cook_recipe", "vegetable-stir-fry");

    apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::ActorCommandUsed {
            actor_id: ACTOR_C_ID.to_string(),
            actor_name: ACTOR_C_NAME.to_string(),
            room_id: KITCHEN_ID.to_string(),
            command_id: "cook".to_string(),
            target_room_id: None,
            target_actor_id: None,
            target_actor_name: None,
            context_label: None,
            feature_id: None,
            consumable_id: None,
            freeform_text: None,
        })],
    );

    assert!(state.has_item_in_storage(
        "vegetable-stir-fry",
        ItemStorageTarget::CurrentRoom,
        KITCHEN_ID,
    ));
    assert!(!state.has_item_in_storage(
        "garlic-noodles",
        ItemStorageTarget::CurrentRoom,
        KITCHEN_ID,
    ));
}

#[test]
fn actor_commands_can_create_target_derived_items_from_content_templates() {
    let mut pack = reducer_test_pack();
    pack.items.push(ItemDefinition {
        id: "memory-blair".to_string(),
        label: "memory of Blair".to_string(),
        description: "A captured moment.".to_string(),
        ..ItemDefinition::default()
    });
    pack.actions.push(ActionDefinition {
        id: "capture".to_string(),
        command: "capture".to_string(),
        target_mode: CommandTargetMode::Actor,
        event_text: "{actor_name} captures a memory of {target_actor_name}.".to_string(),
        item_creation: Some(ActionItemCreation {
            creates_item: "memory-blair".to_string(),
            creates_item_target_template: "memory-{target_actor_id}".to_string(),
            storage: ActionItemStorageTarget::CurrentRoom,
            ..ActionItemCreation::default()
        }),
        ..ActionDefinition::default()
    });
    rebuild_test_pack_indexes(&mut pack);
    let mut state = WorldState::new(&pack);

    apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::ActorCommandUsed {
            actor_id: ACTOR_A_ID.to_string(),
            actor_name: ACTOR_A_NAME.to_string(),
            room_id: LOUNGE_ID.to_string(),
            command_id: "capture".to_string(),
            target_room_id: None,
            target_actor_id: Some(ACTOR_B_ID.to_string()),
            target_actor_name: Some(ACTOR_B_NAME.to_string()),
            context_label: None,
            feature_id: None,
            consumable_id: None,
            freeform_text: None,
        })],
    );

    assert!(state.has_item_in_storage("memory-blair", ItemStorageTarget::CurrentRoom, LOUNGE_ID,));
}

#[test]
fn trace_craftable_is_gated_by_its_story_variable() {
    let mut pack = reducer_test_pack();
    pack.items.extend([
        ItemDefinition {
            id: "charm-sigil".to_string(),
            label: "charm sigil".to_string(),
            description: "A ring-shaped chalk mark.".to_string(),
            trace_mark: true,
            ..ItemDefinition::default()
        },
        ItemDefinition {
            id: "drain-sigil".to_string(),
            label: "drain sigil".to_string(),
            description: "A spiral chalk mark.".to_string(),
            trace_mark: true,
            ..ItemDefinition::default()
        },
    ]);
    pack.actions.push(ActionDefinition {
        id: "trace".to_string(),
        command: "trace".to_string(),
        target_mode: CommandTargetMode::None,
        item_creation: Some(ActionItemCreation {
            creates_item: "charm-sigil".to_string(),
            craftable_items: vec!["charm-sigil".to_string(), "drain-sigil".to_string()],
            craftable_item_gates: BTreeMap::from([(
                "drain-sigil".to_string(),
                "knows_drain".to_string(),
            )]),
            storage: ActionItemStorageTarget::CurrentRoom,
            ..ActionItemCreation::default()
        }),
        event_text: "{actor_name} draws chalk across the floor.".to_string(),
        ..ActionDefinition::default()
    });
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.current_room_id = KITCHEN_ID.to_string();

    // Without the story var, choosing the drain sigil falls back to the
    // unlocked charm sigil; the locked craftable is never created.
    drive_actor_command(
        &mut state,
        &pack,
        "trace",
        plain_command(KITCHEN_ID, Some("drain-sigil")),
    );
    assert!(state.has_item_in_storage("charm-sigil", ItemStorageTarget::CurrentRoom, KITCHEN_ID));
    assert!(!state.has_item_in_storage("drain-sigil", ItemStorageTarget::CurrentRoom, KITCHEN_ID));

    // Once the scroll is read, bare trace selects the next available mark
    // instead of trying to duplicate the existing charm sigil.
    state.story_vars.set_unchecked("knows_drain", "true");
    drive_actor_command(&mut state, &pack, "trace", plain_command(KITCHEN_ID, None));
    assert!(state.has_item_in_storage("drain-sigil", ItemStorageTarget::CurrentRoom, KITCHEN_ID));

    // Different trace marks may coexist, but an explicitly selected duplicate
    // cannot be stacked.
    drive_actor_command(
        &mut state,
        &pack,
        "trace",
        plain_command(KITCHEN_ID, Some("drain-sigil")),
    );
    assert_eq!(
        state.item_count_in_storage("charm-sigil", ItemStorageTarget::CurrentRoom, KITCHEN_ID,),
        1
    );
    assert_eq!(
        state.item_count_in_storage("drain-sigil", ItemStorageTarget::CurrentRoom, KITCHEN_ID,),
        1
    );

    let output = drive_actor_command(&mut state, &pack, "trace", plain_command(KITCHEN_ID, None));
    assert!(output.lines.is_empty());
}