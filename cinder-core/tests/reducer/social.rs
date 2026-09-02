use super::common::*;
use cinder_core::engine::events::{TimestampedWorldEvent, WorldEvent};
use cinder_core::engine::reducer::apply_events;
use cinder_core::engine::state::{ConversationMemoryKind, WorldState};

#[test]
fn actor_action_is_injected_into_roommate_recent_memory() {
    let pack = reducer_test_pack();
    let mut state = WorldState::new(&pack);
    state
        .actor_room_overrides
        .insert(ACTOR_A_ID.to_string(), LOUNGE_ID.to_string());
    state
        .actor_room_overrides
        .insert(ACTOR_B_ID.to_string(), LOUNGE_ID.to_string());
    let events = [TimestampedWorldEvent::now(WorldEvent::ActorCommandUsed {
        actor_id: ACTOR_A_ID.to_string(),
        actor_name: ACTOR_A_NAME.to_string(),
        room_id: LOUNGE_ID.to_string(),
        command_id: "act".to_string(),
        target_room_id: None,
        target_actor_id: None,
        target_actor_name: None,
        context_label: None,
        feature_id: None,
        consumable_id: None,
        freeform_text: Some("sits on the couch".to_string()),
    })];

    let output = apply_events(&mut state, &pack, &events);

    assert!(
        output
            .lines
            .iter()
            .any(|line| line.text == "Alex sits on the couch.")
    );
    let history = state.conversation_history(ACTOR_A_ID, ACTOR_B_ID);
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].kind, ConversationMemoryKind::Action);
    assert_eq!(history[0].text, "Alex sits on the couch.");
}

#[test]
fn hug_increases_attraction_and_safety_for_the_pair() {
    let pack = reducer_test_pack();
    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    let events = [TimestampedWorldEvent::now(WorldEvent::ActorCommandUsed {
        actor_id: ACTOR_A_ID.to_string(),
        actor_name: ACTOR_A_NAME.to_string(),
        room_id: LOUNGE_ID.to_string(),
        command_id: "hug".to_string(),
        target_room_id: None,
        target_actor_id: Some(ACTOR_C_ID.to_string()),
        target_actor_name: Some(ACTOR_C_NAME.to_string()),
        context_label: None,
        feature_id: None,
        consumable_id: None,
        freeform_text: None,
    })];

    let output = apply_events(&mut state, &pack, &events);

    assert!(
        output
            .lines
            .iter()
            .any(|line| line.text == "Alex hugs Casey.")
    );
    assert_eq!(state.pair_stat(ACTOR_A_ID, ACTOR_C_ID, "safety"), 1);
    assert_eq!(state.pair_stat_u32(ACTOR_A_ID, ACTOR_C_ID, "attraction"), 1);
}

#[test]
fn shared_room_tick_increases_pair_safety() {
    let pack = reducer_test_pack();
    let mut state = WorldState::new(&pack);
    let events = [TimestampedWorldEvent::now(WorldEvent::TurnStarted {
        turn_number: 1,
        raw_input: "tick".to_string(),
        advances_time: true,
    })];

    apply_events(&mut state, &pack, &events);

    assert_eq!(state.pair_stat(ACTOR_A_ID, ACTOR_B_ID, "safety"), 1);
}

#[test]
fn tick_progression_updates_hunger_without_reducing_stamina() {
    let pack = reducer_test_pack();
    let mut state = WorldState::new(&pack);
    state.current_time_minutes = 18 * 60 + 55;
    let starting_hunger = state.actor_stat_u32(ACTOR_A_ID, "hunger");
    let starting_stamina = state.actor_stat_u32(ACTOR_A_ID, "stamina");
    let events = [TimestampedWorldEvent::now(WorldEvent::TurnStarted {
        turn_number: 1,
        raw_input: "tick".to_string(),
        advances_time: true,
    })];

    apply_events(&mut state, &pack, &events);

    assert_eq!(
        state.actor_stat_u32(ACTOR_A_ID, "hunger"),
        starting_hunger + 1
    );
    assert_eq!(
        state.actor_stat_u32(ACTOR_A_ID, "stamina"),
        starting_stamina
    );
}

#[test]
fn rest_recovers_stamina() {
    let pack = reducer_test_pack();
    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    state
        .adjust_actor_stat(ACTOR_A_ID, "stamina", -4)
        .expect("known actor stat");
    let starting_stamina = state.actor_stat_u32(ACTOR_A_ID, "stamina");
    let events = [TimestampedWorldEvent::now(WorldEvent::ActorCommandUsed {
        actor_id: ACTOR_A_ID.to_string(),
        actor_name: ACTOR_A_NAME.to_string(),
        room_id: LOUNGE_ID.to_string(),
        command_id: "rest".to_string(),
        target_room_id: None,
        target_actor_id: None,
        target_actor_name: None,
        context_label: Some(SOFA_LABEL.to_string()),
        feature_id: None,
        consumable_id: None,
        freeform_text: None,
    })];

    let output = apply_events(&mut state, &pack, &events);

    assert!(
        output
            .lines
            .iter()
            .any(|line| line.text == "Alex takes a quiet moment to rest on the long sofa.")
    );
    assert_eq!(
        state.actor_stat_u32(ACTOR_A_ID, "stamina"),
        starting_stamina + 1
    );
}

#[test]
fn speech_increases_connection_and_confidence() {
    let pack = reducer_test_pack();
    let mut state = WorldState::new(&pack);
    let starting_confidence = state.actor_stat(ACTOR_A_ID, "confidence");
    let events = [TimestampedWorldEvent::now(WorldEvent::ActorSpoke {
        actor_id: ACTOR_A_ID.to_string(),
        actor_name: ACTOR_A_NAME.to_string(),
        other_person_id: ACTOR_B_ID.to_string(),
        other_person_name: ACTOR_B_NAME.to_string(),
        other_person_message: None,
        room_id: LOUNGE_ID.to_string(),
        text: "Hey.".to_string(),
    })];

    apply_events(&mut state, &pack, &events);

    assert_eq!(state.pair_stat(ACTOR_A_ID, ACTOR_B_ID, "connection"), 1);
    assert_eq!(
        state.actor_stat(ACTOR_A_ID, "confidence"),
        starting_confidence + 1
    );
}

#[test]
fn pair_stat_adjusted_event_increases_attraction() {
    let pack = reducer_test_pack();
    let mut state = WorldState::new(&pack);
    let starting_attraction = state.pair_stat(ACTOR_A_ID, ACTOR_B_ID, "attraction");
    let events = [TimestampedWorldEvent::now(WorldEvent::PairStatAdjusted {
        participant_a_id: ACTOR_A_ID.to_string(),
        participant_b_id: ACTOR_B_ID.to_string(),
        stat: "attraction".to_string(),
        delta: 2,
    })];

    apply_events(&mut state, &pack, &events);

    assert_eq!(
        state.pair_stat(ACTOR_A_ID, ACTOR_B_ID, "attraction"),
        starting_attraction + 2
    );
}

#[test]
fn visible_speech_lines_include_target_when_present() {
    let pack = reducer_test_pack();
    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    let events = [TimestampedWorldEvent::now(WorldEvent::ActorSpoke {
        actor_id: ACTOR_A_ID.to_string(),
        actor_name: ACTOR_A_NAME.to_string(),
        other_person_id: ACTOR_B_ID.to_string(),
        other_person_name: ACTOR_B_NAME.to_string(),
        other_person_message: None,
        room_id: LOUNGE_ID.to_string(),
        text: "Hey.".to_string(),
    })];

    let output = apply_events(&mut state, &pack, &events);

    assert!(
        output
            .lines
            .iter()
            .any(|line| line.text == "Alex (to Blair): Hey.")
    );
}

#[test]
fn leaving_before_reply_on_next_tick_decreases_safety() {
    let pack = reducer_test_pack();
    let mut state = WorldState::new(&pack);
    state.turn_number = 1;
    state.set_pending_reply(ACTOR_A_ID, ACTOR_B_ID, LOUNGE_ID, 1);
    let events = [
        TimestampedWorldEvent::now(WorldEvent::TurnStarted {
            turn_number: 2,
            raw_input: "tick".to_string(),
            advances_time: true,
        }),
        TimestampedWorldEvent::now(WorldEvent::ActorMoved {
            actor_id: ACTOR_A_ID.to_string(),
            from_room_id: LOUNGE_ID.to_string(),
            to_room_id: KITCHEN_ID.to_string(),
        }),
    ];

    apply_events(&mut state, &pack, &events);

    assert_eq!(state.pair_stat(ACTOR_A_ID, ACTOR_B_ID, "safety"), 0);
    assert!(state.pending_reply(ACTOR_A_ID, ACTOR_B_ID).is_none());
}

#[test]
fn offscreen_move_command_shows_arrival_when_actor_enters_current_room() {
    let pack = reducer_test_pack();
    let mut state = WorldState::new(&pack);
    state.current_room_id = KITCHEN_ID.to_string();
    state
        .actor_room_overrides
        .insert(ACTOR_A_ID.to_string(), LOUNGE_ID.to_string());
    let events = [TimestampedWorldEvent::now(WorldEvent::ActorCommandUsed {
        actor_id: ACTOR_A_ID.to_string(),
        actor_name: ACTOR_A_NAME.to_string(),
        room_id: LOUNGE_ID.to_string(),
        command_id: "move".to_string(),
        target_room_id: Some(KITCHEN_ID.to_string()),
        target_actor_id: None,
        target_actor_name: None,
        context_label: None,
        feature_id: None,
        consumable_id: None,
        freeform_text: None,
    })];

    let output = apply_events(&mut state, &pack, &events);

    assert!(
        output
            .lines
            .iter()
            .any(|line| line.text == "Alex comes in from the Lounge.")
    );
    assert_eq!(state.actor_room_id(ACTOR_A_ID, LOUNGE_ID), KITCHEN_ID);
}
