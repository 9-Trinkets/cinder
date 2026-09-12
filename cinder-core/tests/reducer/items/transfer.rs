//! Inventory and room-item transfer tests: take/drop, storage targets, and
//! trace-mark anchoring rules.

use super::super::common::*;
use super::{item, plain_command, trace_item};
use cinder_core::content::types::{
    ActionDefinition, CommandEffect, CommandTargetMode, ItemStorageTarget,
};
use cinder_core::engine::events::{TimestampedWorldEvent, WorldEvent};
use cinder_core::engine::reducer::apply_events;
use cinder_core::engine::state::WorldState;

#[test]
fn item_events_can_store_and_consume_items_in_current_room() {
    let mut pack = reducer_test_pack();
    pack.items = vec![item("coffee", "coffee", "Fresh coffee.")];
    rebuild_test_pack_indexes(&mut pack);
    let mut state = WorldState::new(&pack);
    state.current_room_id = KITCHEN_ID.to_string();

    apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::ItemAcquired {
            item_id: "coffee".to_string(),
            storage: ItemStorageTarget::CurrentRoom,
        })],
    );

    assert!(state.has_item_in_storage("coffee", ItemStorageTarget::CurrentRoom, KITCHEN_ID,));

    apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::ItemConsumed {
            item_id: "coffee".to_string(),
            storage: ItemStorageTarget::CurrentRoom,
            consumer_id: None,
            consumer_name: None,
        })],
    );

    assert!(!state.has_item_in_storage("coffee", ItemStorageTarget::CurrentRoom, KITCHEN_ID,));
}

#[test]
fn item_events_keep_player_inventory_behavior_unchanged() {
    let mut pack = reducer_test_pack();
    pack.items = vec![item("tea", "tea", "Hot tea.")];
    rebuild_test_pack_indexes(&mut pack);
    let mut state = WorldState::new(&pack);

    let output = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::ItemAcquired {
            item_id: "tea".to_string(),
            storage: ItemStorageTarget::PlayerInventory,
        })],
    );

    assert!(state.has_item("tea"));
    assert!(
        output
            .lines
            .iter()
            .any(|line| line.text == "You have tea ready.")
    );
}

#[test]
fn player_take_moves_loose_item_to_inventory() {
    let mut pack = reducer_test_pack();
    pack.items = vec![item("ring", "warden ring", "A warm ring.")];
    rebuild_test_pack_indexes(&mut pack);
    let mut state = WorldState::new(&pack);
    state.current_room_id = KITCHEN_ID.to_string();

    apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::ItemAcquired {
            item_id: "ring".to_string(),
            storage: ItemStorageTarget::CurrentRoom,
        })],
    );
    assert!(state.has_item_in_storage("ring", ItemStorageTarget::CurrentRoom, KITCHEN_ID,));

    let output = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::PlayerTookItem {
            item_id: "ring".to_string(),
        })],
    );

    assert!(state.has_item("ring"));
    assert!(!state.has_item_in_storage("ring", ItemStorageTarget::CurrentRoom, KITCHEN_ID,));
    assert!(
        output
            .lines
            .iter()
            .any(|line| line.text.contains("warden ring"))
    );
}

#[test]
fn player_drop_moves_inventory_item_to_current_room() {
    let mut pack = reducer_test_pack();
    pack.items = vec![item("ring", "warden ring", "A warm ring.")];
    rebuild_test_pack_indexes(&mut pack);
    let mut state = WorldState::new(&pack);
    state.current_room_id = KITCHEN_ID.to_string();
    state.add_item("ring");
    assert!(state.has_item("ring"));

    let output = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::PlayerDroppedItem {
            item_id: "ring".to_string(),
        })],
    );

    assert!(!state.has_item("ring"));
    assert!(state.has_item_in_storage("ring", ItemStorageTarget::CurrentRoom, KITCHEN_ID,));
    assert!(
        output
            .lines
            .iter()
            .any(|line| line.text.contains("warden ring"))
    );
}

#[test]
fn player_take_is_denied_for_trace_mark_items() {
    let mut pack = reducer_test_pack();
    pack.items = vec![trace_item("chalk-sigil", "chalk sigil", "A chalk mark drawn on the floor.")];
    rebuild_test_pack_indexes(&mut pack);
    let mut state = WorldState::new(&pack);
    state.current_room_id = KITCHEN_ID.to_string();

    apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::ItemAcquired {
            item_id: "chalk-sigil".to_string(),
            storage: ItemStorageTarget::CurrentRoom,
        })],
    );
    assert!(state.has_item_in_storage(
        "chalk-sigil",
        ItemStorageTarget::CurrentRoom,
        KITCHEN_ID
    ));

    let output = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::PlayerTookItem {
            item_id: "chalk-sigil".to_string(),
        })],
    );

    assert!(
        state.has_item_in_storage("chalk-sigil", ItemStorageTarget::CurrentRoom, KITCHEN_ID),
        "trace mark must stay anchored in the room"
    );
    assert!(!state.has_item("chalk-sigil"), "trace mark cannot enter inventory");
    assert!(
        output
            .lines
            .iter()
            .any(|line| line.text.contains("can't carry it away")),
        "take denial should narrate the takedenied line"
    );
}

#[test]
fn player_drop_is_denied_for_trace_mark_items() {
    let mut pack = reducer_test_pack();
    pack.items = vec![trace_item("chalk-sigil", "chalk sigil", "A chalk mark drawn on the floor.")];
    rebuild_test_pack_indexes(&mut pack);
    let mut state = WorldState::new(&pack);
    state.current_room_id = KITCHEN_ID.to_string();
    state.add_item("chalk-sigil");

    let output = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::PlayerDroppedItem {
            item_id: "chalk-sigil".to_string(),
        })],
    );

    assert!(state.has_item("chalk-sigil"), "trace mark should not be droppable");
    assert!(state.loose_room_items(KITCHEN_ID).is_empty());
    assert!(
        output
            .lines
            .iter()
            .any(|line| line.text.contains("can't carry it away"))
    );
}

#[test]
fn drop_and_pick_up_item_move_it_between_inventory_and_room() {
    let mut pack = reducer_test_pack();
    pack.actions.push(ActionDefinition {
        id: "drop-marker".to_string(),
        command: "drop-marker".to_string(),
        target_mode: CommandTargetMode::None,
        effects: vec![CommandEffect::DropItem],
        item_id: "stone-marker".to_string(),
        event_text: "{actor_name} places the stone marker on the ground.".to_string(),
        ..ActionDefinition::default()
    });
    pack.actions.push(ActionDefinition {
        id: "pick-up-marker".to_string(),
        command: "pick-up-marker".to_string(),
        target_mode: CommandTargetMode::None,
        effects: vec![CommandEffect::PickUpItem],
        item_id: "stone-marker".to_string(),
        event_text: "{actor_name} picks up the stone marker.".to_string(),
        ..ActionDefinition::default()
    });
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    state.add_item("stone-marker");

    drive_actor_command(&mut state, &pack, "drop-marker", plain_command(LOUNGE_ID, None));

    assert!(!state.has_item("stone-marker"));
    assert_eq!(
        state.loose_room_items(LOUNGE_ID),
        vec![("stone-marker".to_string(), 1)]
    );

    drive_actor_command(&mut state, &pack, "pick-up-marker", plain_command(LOUNGE_ID, None));

    assert!(state.has_item("stone-marker"));
    assert!(state.loose_room_items(LOUNGE_ID).is_empty());
}