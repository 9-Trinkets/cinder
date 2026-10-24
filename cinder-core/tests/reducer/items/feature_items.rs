//! Items held by room features: seeding, taking them, and putting them back.

use super::super::common::*;
use super::item;
use cinder_core::content::types::{ConsumableKind, ConsumableSpec};
use cinder_core::engine::events::{TimestampedWorldEvent, WorldEvent};
use cinder_core::engine::reducer::apply_events;
use cinder_core::engine::state::WorldState;

fn pack_with_a_ring_on_the_sofa() -> cinder_core::content::types::ContentPack {
    let mut pack = reducer_test_pack();
    pack.items = vec![item("ring", "warden ring", "A warm ring.")];
    pack.rooms[0].features[0]
        .items
        .insert("ring".to_string(), 1);
    rebuild_test_pack_indexes(&mut pack);
    pack
}

#[test]
fn a_feature_starts_out_holding_its_items() {
    let pack = pack_with_a_ring_on_the_sofa();
    let state = WorldState::new(&pack);
    assert_eq!(state.feature_item_count(LOUNGE_ID, SOFA_ID, "ring"), 1);
    assert_eq!(
        state.feature_items(LOUNGE_ID, SOFA_ID),
        vec![("ring".to_string(), 1)]
    );
    assert!(
        state.loose_room_items(LOUNGE_ID).is_empty(),
        "an item on a feature is not lying on the floor"
    );
}

#[test]
fn taking_an_item_from_a_feature_moves_it_into_the_inventory() {
    let pack = pack_with_a_ring_on_the_sofa();
    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();

    let output = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::PlayerTookItem {
            item_id: "ring".to_string(),
            feature_id: Some(SOFA_ID.to_string()),
        })],
    );

    assert!(state.has_item("ring"));
    assert_eq!(state.feature_item_count(LOUNGE_ID, SOFA_ID, "ring"), 0);
    assert!(
        output
            .lines
            .iter()
            .any(|line| line.text.contains("warden ring"))
    );
}

#[test]
fn an_item_returned_by_a_swap_goes_back_to_its_feature() {
    let mut pack = reducer_test_pack();
    let mut ring = item("ring", "warden ring", "A warm ring.");
    ring.exclusive_group = "jewelry".to_string();
    let mut brooch = item("brooch", "silver brooch", "A cold brooch.");
    brooch.exclusive_group = "jewelry".to_string();
    pack.items = vec![ring, brooch];
    pack.rooms[0].features[0]
        .items
        .extend([("ring".to_string(), 1), ("brooch".to_string(), 1)]);
    rebuild_test_pack_indexes(&mut pack);
    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();

    for item_id in ["ring", "brooch"] {
        apply_events(
            &mut state,
            &pack,
            &[TimestampedWorldEvent::now(WorldEvent::PlayerTookItem {
                item_id: item_id.to_string(),
                feature_id: Some(SOFA_ID.to_string()),
            })],
        );
    }

    assert!(state.has_item("brooch"));
    assert!(!state.has_item("ring"));
    assert_eq!(state.feature_item_count(LOUNGE_ID, SOFA_ID, "ring"), 1);
    assert!(state.loose_room_items(LOUNGE_ID).is_empty());
}

#[test]
fn consumables_are_used_in_place_and_cannot_be_taken() {
    let mut pack = reducer_test_pack();
    let mut tea = item("tea", "iced tea", "Cold tea.");
    tea.consumable = Some(ConsumableSpec {
        kind: ConsumableKind::Drink,
        hunger_recovery: 0,
        stamina_recovery: 2,
    });
    pack.items = vec![tea, item("ring", "warden ring", "A warm ring.")];
    pack.rooms[0].features[0]
        .items
        .extend([("tea".to_string(), 3), ("ring".to_string(), 1)]);
    rebuild_test_pack_indexes(&mut pack);
    let state = WorldState::new(&pack);

    let takeable = state.takeable_feature_items(&pack, LOUNGE_ID);
    assert_eq!(
        takeable,
        vec![(SOFA_ID.to_string(), "ring".to_string(), 1)],
        "only the ring can be carried off"
    );
    assert_eq!(pack.room_consumables(LOUNGE_ID).len(), 1);
}

#[test]
fn feature_stock_can_be_added_to_and_removed_from() {
    let pack = pack_with_a_ring_on_the_sofa();
    let mut state = WorldState::new(&pack);
    state.add_feature_item(LOUNGE_ID, SOFA_ID, "ring", 2);
    assert_eq!(state.feature_item_count(LOUNGE_ID, SOFA_ID, "ring"), 3);
    assert!(state.remove_feature_item(LOUNGE_ID, SOFA_ID, "ring"));
    assert_eq!(state.feature_item_count(LOUNGE_ID, SOFA_ID, "ring"), 2);
    assert!(!state.remove_feature_item(LOUNGE_ID, SOFA_ID, "missing"));
}

#[test]
fn a_saved_game_from_before_the_rename_still_loads_its_stock() {
    let pack = pack_with_a_ring_on_the_sofa();
    let state = WorldState::new(&pack);
    let mut json: serde_json::Value = serde_json::to_value(&state).expect("serializes");
    let stock = json
        .as_object_mut()
        .expect("object")
        .remove("feature_item_stock")
        .expect("field");
    json["feature_consumable_stock"] = stock;
    let loaded: WorldState = serde_json::from_value(json).expect("old saves still load");
    assert_eq!(loaded.feature_item_count(LOUNGE_ID, SOFA_ID, "ring"), 1);
}
