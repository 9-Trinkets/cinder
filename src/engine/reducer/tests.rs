use super::beat_advance::{advance_conditions_met, evaluate_advance_condition};
use super::*;
use crate::content::loader::load_named_pack;
use crate::engine::state::ConversationMemoryKind;

#[test]
fn actor_action_is_injected_into_roommate_recent_memory() {
    let pack = load_named_pack("aera", None).expect("load aera pack");
    let mut state = WorldState::new(&pack);
    state
        .actor_room_overrides
        .insert("aera".to_string(), "lounge".to_string());
    state
        .actor_room_overrides
        .insert("ren".to_string(), "lounge".to_string());
    let events = [TimestampedWorldEvent::now(WorldEvent::ActorCommandUsed {
        actor_id: "aera".to_string(),
        actor_name: "Aera".to_string(),
        room_id: "lounge".to_string(),
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
            .any(|line| line == "Aera sits on the couch.")
    );
    let history = state.conversation_history("aera", "ren");
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].kind, ConversationMemoryKind::Action);
    assert_eq!(history[0].text, "Aera sits on the couch.");
}

#[test]
fn hug_increases_attraction_and_safety_for_the_pair() {
    let mut pack = load_named_pack("aera", None).expect("load aera pack");
    for actor in &mut pack.actors {
        actor.initial_pair_stats.clear();
    }
    let mut state = WorldState::new(&pack);
    state.current_room_id = "lounge".to_string();
    let events = [TimestampedWorldEvent::now(WorldEvent::ActorCommandUsed {
        actor_id: "aera".to_string(),
        actor_name: "Aera".to_string(),
        room_id: "lounge".to_string(),
        command_id: "hug".to_string(),
        target_room_id: None,
        target_actor_id: Some("mio".to_string()),
        target_actor_name: Some("Mio".to_string()),
        context_label: None,
        feature_id: None,
        consumable_id: None,
        freeform_text: None,
    })];

    let output = apply_events(&mut state, &pack, &events);

    assert!(output.lines.iter().any(|line| line == "Aera hugs Mio."));
    assert_eq!(state.pair_stat("aera", "mio", "safety"), 1);
    assert_eq!(state.pair_stat_u32("aera", "mio", "attraction"), 1);
}

#[test]
fn shared_room_tick_increases_pair_safety() {
    let pack = load_named_pack("aera", None).expect("load aera pack");
    let mut state = WorldState::new(&pack);
    let events = [TimestampedWorldEvent::now(WorldEvent::TurnStarted {
        turn_number: 1,
        raw_input: "tick".to_string(),
        advances_time: true,
    })];

    apply_events(&mut state, &pack, &events);

    assert_eq!(state.pair_stat("aera", "ren", "safety"), 1);
}

#[test]
fn tick_progression_updates_hunger_without_reducing_stamina() {
    let pack = load_named_pack("aera", None).expect("load aera pack");
    let mut state = WorldState::new(&pack);
    state.current_time_minutes = 18 * 60 + 55;
    let starting_hunger = state.actor_stat_u32("aera", "hunger");
    let starting_stamina = state.actor_stat_u32("aera", "stamina");
    let events = [TimestampedWorldEvent::now(WorldEvent::TurnStarted {
        turn_number: 1,
        raw_input: "tick".to_string(),
        advances_time: true,
    })];

    apply_events(&mut state, &pack, &events);

    assert_eq!(state.actor_stat_u32("aera", "hunger"), starting_hunger + 1);
    assert_eq!(state.actor_stat_u32("aera", "stamina"), starting_stamina);
}

#[test]
fn rest_recovers_stamina() {
    let pack = load_named_pack("aera", None).expect("load aera pack");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "lounge".to_string();
    state
        .adjust_actor_stat("aera", "stamina", -4)
        .expect("known actor stat");
    let starting_stamina = state.actor_stat_u32("aera", "stamina");
    let events = [TimestampedWorldEvent::now(WorldEvent::ActorCommandUsed {
        actor_id: "aera".to_string(),
        actor_name: "Aera".to_string(),
        room_id: "lounge".to_string(),
        command_id: "rest".to_string(),
        target_room_id: None,
        target_actor_id: None,
        target_actor_name: None,
        context_label: Some("long sofa".to_string()),
        feature_id: None,
        consumable_id: None,
        freeform_text: None,
    })];

    let output = apply_events(&mut state, &pack, &events);

    assert!(
        output
            .lines
            .iter()
            .any(|line| line == "Aera takes a quiet moment to rest on the long sofa.")
    );
    assert_eq!(
        state.actor_stat_u32("aera", "stamina"),
        starting_stamina + 1
    );
}

#[test]
fn speech_increases_connection_and_confidence() {
    let pack = load_named_pack("aera", None).expect("load aera pack");
    let mut state = WorldState::new(&pack);
    let starting_confidence = state.actor_stat("aera", "confidence");
    let events = [TimestampedWorldEvent::now(WorldEvent::ActorSpoke {
        actor_id: "aera".to_string(),
        actor_name: "Aera".to_string(),
        other_person_id: "ren".to_string(),
        other_person_name: "Ren".to_string(),
        other_person_message: None,
        room_id: "lounge".to_string(),
        text: "Hey.".to_string(),
    })];

    apply_events(&mut state, &pack, &events);

    assert_eq!(state.pair_stat("aera", "ren", "connection"), 1);
    assert_eq!(
        state.actor_stat("aera", "confidence"),
        starting_confidence + 1
    );
}

#[test]
fn pair_stat_adjusted_event_increases_attraction() {
    let pack = load_named_pack("aera", None).expect("load aera pack");
    let mut state = WorldState::new(&pack);
    let starting_attraction = state.pair_stat("aera", "ren", "attraction");
    let events = [TimestampedWorldEvent::now(WorldEvent::PairStatAdjusted {
        participant_a_id: "aera".to_string(),
        participant_b_id: "ren".to_string(),
        stat: "attraction".to_string(),
        delta: 2,
    })];

    apply_events(&mut state, &pack, &events);

    assert_eq!(
        state.pair_stat("aera", "ren", "attraction"),
        starting_attraction + 2
    );
}

#[test]
fn visible_speech_lines_include_target_when_present() {
    let pack = load_named_pack("aera", None).expect("load aera pack");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "lounge".to_string();
    let events = [TimestampedWorldEvent::now(WorldEvent::ActorSpoke {
        actor_id: "aera".to_string(),
        actor_name: "Aera".to_string(),
        other_person_id: "ren".to_string(),
        other_person_name: "Ren".to_string(),
        other_person_message: None,
        room_id: "lounge".to_string(),
        text: "Hey.".to_string(),
    })];

    let output = apply_events(&mut state, &pack, &events);

    assert!(
        output
            .lines
            .iter()
            .any(|line| line == "Aera (to Ren): Hey.")
    );
}

#[test]
fn leaving_before_reply_on_next_tick_decreases_safety() {
    let pack = load_named_pack("aera", None).expect("load aera pack");
    let mut state = WorldState::new(&pack);
    state.turn_number = 1;
    state.set_pending_reply("aera", "ren", "lounge", 1);
    let events = [
        TimestampedWorldEvent::now(WorldEvent::TurnStarted {
            turn_number: 2,
            raw_input: "tick".to_string(),
            advances_time: true,
        }),
        TimestampedWorldEvent::now(WorldEvent::ActorMoved {
            actor_id: "aera".to_string(),
            from_room_id: "lounge".to_string(),
            to_room_id: "kitchen".to_string(),
        }),
    ];

    apply_events(&mut state, &pack, &events);

    assert_eq!(state.pair_stat("aera", "ren", "safety"), 0);
    assert!(state.pending_reply("aera", "ren").is_none());
}

#[test]
fn offscreen_move_command_shows_arrival_when_actor_enters_current_room() {
    let pack = load_named_pack("aera", None).expect("load aera pack");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "kitchen".to_string();
    state
        .actor_room_overrides
        .insert("aera".to_string(), "lounge".to_string());
    let events = [TimestampedWorldEvent::now(WorldEvent::ActorCommandUsed {
        actor_id: "aera".to_string(),
        actor_name: "Aera".to_string(),
        room_id: "lounge".to_string(),
        command_id: "move".to_string(),
        target_room_id: Some("kitchen".to_string()),
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
            .any(|line| line == "Aera comes in from the Lounge.")
    );
    assert_eq!(state.actor_room_id("aera", "lounge"), "kitchen");
}

#[test]
fn actor_observation_events_feed_recent_observation_memory() {
    let pack = load_named_pack("aera", None).expect("load aera pack");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "lounge".to_string();
    let events = [
        TimestampedWorldEvent::now(WorldEvent::ActorObservedRoom {
            actor_id: "aera".to_string(),
            actor_name: "Aera".to_string(),
            room_id: "lounge".to_string(),
        }),
        TimestampedWorldEvent::now(WorldEvent::ActorObservedFeature {
            actor_id: "aera".to_string(),
            actor_name: "Aera".to_string(),
            room_id: "lounge".to_string(),
            feature_id: "sofa".to_string(),
        }),
        TimestampedWorldEvent::now(WorldEvent::ActorObservedActor {
            actor_id: "aera".to_string(),
            actor_name: "Aera".to_string(),
            room_id: "lounge".to_string(),
            target_actor_id: "ren".to_string(),
            target_actor_name: "Ren".to_string(),
        }),
    ];

    let output = apply_events(&mut state, &pack, &events);

    assert!(
        output
            .lines
            .iter()
            .any(|line| line == "Aera pauses to take in the Lounge more carefully.")
    );
    assert!(
        output
            .lines
            .iter()
            .any(|line| line == "Aera studies Ren more closely.")
    );
    assert!(
        output
            .lines
            .iter()
            .any(|line| line == "Aera studies the long sofa.")
    );
    assert!(state.actor_has_seen_feature("aera", "lounge", "sofa"));
    assert!(state.actor_has_studied_actor("aera", "ren"));
    assert_eq!(state.actor_recent_observation_notes("aera").len(), 3);
}

#[test]
fn advance_conditions_met_simple_signal_always_passes() {
    use crate::content::types::AdvanceSignal;
    let pack = load_named_pack("aera", None).expect("load aera pack");
    let state = WorldState::new(&pack);
    let signal = AdvanceSignal::Simple("time_reached:20:00".to_string());
    assert!(advance_conditions_met(&state, signal.conditions()));
}

#[test]
fn advance_conditions_met_conditional_signal_fires_when_condition_met() {
    use crate::content::types::AdvanceCondition;
    let pack = load_named_pack("aera", None).expect("load aera pack");
    let mut state = WorldState::new(&pack);
    state
        .pair_stats
        .entry("aera".to_string())
        .or_default()
        .insert("ren.connection".to_string(), 5);
    let flat_input = serde_json::json!({
        "score": 5
    });
    let flat_cond = AdvanceCondition {
        path: "score".to_string(),
        operator: "gte".to_string(),
        value: serde_json::json!(4),
    };
    assert!(evaluate_advance_condition(&flat_input, &flat_cond));
}

#[test]
fn advance_conditions_met_conditional_signal_blocked_when_condition_not_met() {
    use crate::content::types::AdvanceCondition;
    let flat_input = serde_json::json!({
        "score": 2
    });
    let flat_cond = AdvanceCondition {
        path: "score".to_string(),
        operator: "gte".to_string(),
        value: serde_json::json!(4),
    };
    assert!(!evaluate_advance_condition(&flat_input, &flat_cond));
}
