//! Aera's food, drink and clips are items held by the features that serve them.

use cinder_core::content::loader::load_named_pack;
use cinder_core::content::types::{ConsumableKind, ItemStorageTarget};
use cinder_core::engine::events::{TimestampedWorldEvent, WorldEvent};
use cinder_core::engine::reducer::apply_events;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::WorldState;

fn pack() -> cinder_core::content::types::ContentPack {
    load_named_pack("aera", Some("en")).expect("aera loads")
}

#[test]
#[ignore = "rebooted aera pack: items pending reimplementation"]
fn the_kitchen_starts_with_its_drinks_and_nothing_cooked() {
    let pack = pack();
    let state = WorldState::new(&pack);
    assert_eq!(
        state.feature_item_count("kitchen", "island", "citrus-iced-tea"),
        6
    );
    assert_eq!(
        state.feature_item_count("kitchen", "fridge", "sparkling-water"),
        6
    );
    assert_eq!(
        state.feature_item_count("kitchen", "island", "garlic-noodles"),
        0
    );
}

#[test]
#[ignore = "rebooted aera pack: items pending reimplementation"]
fn every_feature_item_that_can_be_consumed_says_how() {
    let pack = pack();
    let kitchen = pack.room_consumables("kitchen");
    assert_eq!(kitchen.len(), 10, "eight at the island, two in the fridge");
    let noodles = kitchen
        .iter()
        .find(|c| c.consumable.id == "garlic-noodles")
        .expect("noodles");
    assert_eq!(noodles.consumable.kind, ConsumableKind::Eat);
    assert_eq!(noodles.consumable.hunger_recovery, 8);
    assert_eq!(noodles.consumable.tags, ["vegan"]);
    assert_eq!(pack.room_consumables("studio").len(), 4);
}

#[test]
#[ignore = "rebooted aera pack: items pending reimplementation"]
fn drinking_from_the_fridge_uses_up_one_and_recovers_stamina() {
    let pack = pack();
    let mut state = WorldState::new(&pack);
    let before = state.actor_stat_u32("ren", "stamina");
    apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::ActorCommandUsed {
            actor_id: "ren".to_string(),
            actor_name: "Ren".to_string(),
            room_id: "kitchen".to_string(),
            command_id: "drink".to_string(),
            target_room_id: None,
            target_actor_id: None,
            target_actor_name: None,
            context_label: None,
            feature_id: Some("fridge".to_string()),
            consumable_id: Some("sparkling-water".to_string()),
            freeform_text: None,
        })],
    );
    assert_eq!(
        state.feature_item_count("kitchen", "fridge", "sparkling-water"),
        5
    );
    assert!(state.actor_stat_u32("ren", "stamina") >= before);
}

#[test]
#[ignore = "rebooted aera pack: items pending reimplementation"]
fn inspecting_a_feature_lists_what_it_holds_and_what_was_just_cooked() {
    let pack = pack();
    let state = WorldState::new(&pack);
    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime");
    runtime.run_turn("go kitchen").expect("go kitchen");
    let fridge = runtime.run_turn("look at fridge").expect("look").text();
    assert!(
        fridge.contains("Available at the double-door fridge: sparkling water."),
        "{fridge}"
    );

    let mut state = runtime.export_state().expect("state");
    state.add_item_to_storage("garlic-noodles", ItemStorageTarget::CurrentRoom, "kitchen");
    let pack = self::pack();
    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime");
    let island = runtime
        .run_turn("look at kitchen island")
        .expect("look")
        .text();
    assert!(island.contains("citrus iced tea"), "{island}");
    assert!(island.contains("garlic butter noodles"), "{island}");
}

#[test]
#[ignore = "rebooted aera pack: items pending reimplementation"]
fn nothing_in_aera_can_be_carried_off_a_feature() {
    let pack = pack();
    let state = WorldState::new(&pack);
    assert!(state.takeable_feature_items(&pack, "kitchen").is_empty());
}
