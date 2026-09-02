use super::common::*;
use cinder_core::content::types::{
    CombatSettingsDefinition, ItemDefinition, ItemStorageTarget, RoomDescriptionOverride,
};
use cinder_core::engine::events::{ObservationMode, TimestampedWorldEvent, WorldEvent};
use cinder_core::engine::reducer::apply_events;
use cinder_core::engine::state::{ActorStance, WorldState};

#[test]
fn actor_observation_events_feed_recent_observation_memory() {
    let pack = reducer_test_pack();
    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    let events = [
        TimestampedWorldEvent::now(WorldEvent::ActorObservedRoom {
            actor_id: ACTOR_A_ID.to_string(),
            actor_name: ACTOR_A_NAME.to_string(),
            room_id: LOUNGE_ID.to_string(),
        }),
        TimestampedWorldEvent::now(WorldEvent::ActorObservedFeature {
            actor_id: ACTOR_A_ID.to_string(),
            actor_name: ACTOR_A_NAME.to_string(),
            room_id: LOUNGE_ID.to_string(),
            feature_id: SOFA_ID.to_string(),
        }),
        TimestampedWorldEvent::now(WorldEvent::ActorObservedActor {
            actor_id: ACTOR_A_ID.to_string(),
            actor_name: ACTOR_A_NAME.to_string(),
            room_id: LOUNGE_ID.to_string(),
            target_actor_id: ACTOR_B_ID.to_string(),
            target_actor_name: ACTOR_B_NAME.to_string(),
        }),
    ];

    let output = apply_events(&mut state, &pack, &events);

    assert!(
        output
            .lines
            .iter()
            .any(|line| line.text == "Alex pauses to take in the Lounge more carefully.")
    );
    assert!(
        output
            .lines
            .iter()
            .any(|line| line.text == "Alex studies Blair more closely.")
    );
    assert!(
        output
            .lines
            .iter()
            .any(|line| line.text == "Alex studies the long sofa.")
    );
    assert!(state.actor_has_seen_feature(ACTOR_A_ID, LOUNGE_ID, SOFA_ID));
    assert!(state.actor_has_studied_actor(ACTOR_A_ID, ACTOR_B_ID));
    assert_eq!(state.actor_recent_observation_notes(ACTOR_A_ID).len(), 3);
}

#[test]
fn room_observation_annotates_present_actors_by_stance() {
    let mut pack = reducer_test_pack();
    pack.presentation.presentation_text.ally_suffix = " (ally)".to_string();
    pack.presentation.presentation_text.hostile_suffix = " (enemy)".to_string();
    pack.presentation.presentation_text.room_observation =
        "{room_title} {body} {people}".to_string();
    pack.presentation.presentation_text.people = "Here: {people}.".to_string();
    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    state.set_stance(ACTOR_A_ID, ActorStance::Allied);
    state.set_stance(ACTOR_B_ID, ActorStance::Hostile);

    let text = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(
            WorldEvent::CurrentRoomObserved {
                room_id: LOUNGE_ID.to_string(),
                mode: ObservationMode::Summary,
            },
        )],
    )
    .lines
    .to_text();

    assert!(text.contains("Alex (ally)"), "got: {text}");
    assert!(text.contains("Blair (enemy)"), "got: {text}");
}

#[test]
fn room_observation_lists_loose_items_on_the_ground() {
    let mut pack = reducer_test_pack();
    pack.presentation.presentation_text.room_observation =
        "{room_title} {body} {items} {people}".to_string();
    pack.presentation.presentation_text.loose_items = "On the ground: {items}.".to_string();
    pack.presentation.presentation_text.people = "Here: {people}.".to_string();
    pack.items.push(ItemDefinition {
        id: "stone-marker".to_string(),
        label: "stone marker".to_string(),
        description: "A marker.".to_string(),
        ..ItemDefinition::default()
    });
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    state.add_item_to_storage("stone-marker", ItemStorageTarget::CurrentRoom, LOUNGE_ID);

    let text = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(
            WorldEvent::CurrentRoomObserved {
                room_id: LOUNGE_ID.to_string(),
                mode: ObservationMode::Summary,
            },
        )],
    )
    .lines
    .to_text();

    assert!(text.contains("On the ground: stone marker."), "got: {text}");

    // A room without loose items omits the line entirely.
    state.current_room_id = KITCHEN_ID.to_string();
    let empty = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(
            WorldEvent::CurrentRoomObserved {
                room_id: KITCHEN_ID.to_string(),
                mode: ObservationMode::Summary,
            },
        )],
    )
    .lines
    .to_text();
    assert!(!empty.contains("On the ground"), "got: {empty}");
}

#[test]
fn level_reveal_follows_the_declared_room_prefix() {
    let mut pack = minimal_test_pack();
    assert!(
        pack.levels_revealed_for_room("alpha"),
        "empty prefix: levels always visible"
    );
    pack.settings.level_reveal_room_prefix = "down".to_string();
    assert!(
        !pack.levels_revealed_for_room("topside"),
        "upper stays hidden"
    );
    assert!(
        pack.levels_revealed_for_room("down-barrow"),
        "descended board reveals levels"
    );
}

#[test]
fn room_observation_switches_to_defeat_override_description_once_actor_falls() {
    let mut pack = reducer_test_pack();
    pack.presentation.presentation_text.room_observation = "{room_title} {body}".to_string();
    pack.settings.combat = CombatSettingsDefinition {
        player_actor_id: ACTOR_A_ID.to_string(),
        health_stat_id: "stamina".to_string(),
        ..CombatSettingsDefinition::default()
    };
    let kitchen = pack
        .rooms
        .iter_mut()
        .find(|room| room.id == KITCHEN_ID)
        .expect("kitchen room");
    kitchen.descriptions.push(RoomDescriptionOverride {
        actor_defeated: ACTOR_C_ID.to_string(),
        summary: "The quiet kitchen is empty now.".to_string(),
        inspect_text: "No one stands in the quiet kitchen.".to_string(),
    });
    let mut state = WorldState::new(&pack);

    let before = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(
            WorldEvent::CurrentRoomObserved {
                room_id: KITCHEN_ID.to_string(),
                mode: ObservationMode::Summary,
            },
        )],
    )
    .lines
    .to_text();
    assert!(before.contains("quiet kitchen"), "got: {before}");

    state
        .actor_stats
        .entry(ACTOR_C_ID.to_string())
        .or_default()
        .insert("stamina".to_string(), 0);
    let after = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(
            WorldEvent::CurrentRoomObserved {
                room_id: KITCHEN_ID.to_string(),
                mode: ObservationMode::Summary,
            },
        )],
    )
    .lines
    .to_text();
    assert!(after.contains("empty now"), "got: {after}");
}
