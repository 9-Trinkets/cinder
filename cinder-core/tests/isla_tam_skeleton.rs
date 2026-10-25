//! The Tam episode's skeleton: Wren's act ends on a card that jumps ahead, the
//! world carries on, and Isla begins a new day somewhere else in the house.

use cinder_core::content::loader::load_named_pack;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::{GamePhase, WorldState};

fn pack() -> cinder_core::content::types::ContentPack {
    load_named_pack("isla", Some("en")).expect("isla loads")
}

/// A runtime at the moment Wren's session closes with the low note.
fn after_wren() -> CinderRuntime {
    let pack = pack();
    let mut state = WorldState::new(&pack);
    state.current_room_id = "back-yard".to_string();
    state.active_objective_stage_ids = vec!["stage-read-note".to_string()];
    state.add_item("note-three-low");
    state.add_item("wren-drawing-low");
    // Wren left the house at her farewell.
    state
        .actors
        .actor_room_overrides
        .insert("wren".to_string(), String::new());
    // Wren's story hour left the room like this.
    state.set_feature_state(&pack, "playroom", "lamp", "bright");
    state.set_feature_state(&pack, "playroom", "overhead-light", "off");
    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime");
    let outcome = runtime
        .run_turn("look at note-three-low")
        .expect("read the note");
    assert_eq!(outcome.phase, GamePhase::ActEnded, "{}", outcome.text());
    runtime.continue_after_act().expect("continues");
    runtime
}

fn go(runtime: &CinderRuntime, steps: &[&str]) {
    for step in steps {
        runtime.run_turn(step).expect("turn");
    }
}

fn state(runtime: &CinderRuntime) -> WorldState {
    runtime.export_state().expect("state")
}

#[test]
fn wrens_episode_ends_the_act_and_tams_begins_in_another_room() {
    let runtime = after_wren();
    let state = state(&runtime);
    assert_eq!(state.phase, GamePhase::Active);
    assert_eq!(state.current_room_id, "staff-room");
    assert_eq!(state.story_vars.get("episode_tam"), Some("true"));
    assert!(
        state
            .active_objective_stage_ids
            .contains(&"stage-tam-open".to_string()),
        "{:?}",
        state.active_objective_stage_ids
    );
}

#[test]
fn what_isla_carries_persists_and_the_room_is_reset() {
    let runtime = after_wren();
    let state = state(&runtime);
    let pack = pack();
    assert!(state.has_item("wren-drawing-low"), "Wren's drawing is kept");
    assert!(state.has_item("note-three-low"), "so is her last note");
    for note in ["note-tam-intake", "note-tam-school", "note-tam-margin"] {
        assert!(state.has_item(note), "{note}");
    }
    assert_eq!(
        state.feature_state(&pack, "playroom", "lamp").as_deref(),
        Some("off")
    );
    assert_eq!(
        state
            .feature_state(&pack, "playroom", "overhead-light")
            .as_deref(),
        Some("on")
    );
}

#[test]
fn the_clock_jumps_seventy_five_days() {
    let runtime = after_wren();
    let minutes = state(&runtime).clock.current_time_minutes;
    assert!(minutes >= 75 * 1440, "{minutes}");
    assert_eq!(
        (minutes / 1440) % 7,
        (935 / 1440 + 75) % 7,
        "seventy-five days later is the same weekday plus five"
    );
}

#[test]
fn tam_and_his_grandfather_wait_and_tam_stops_at_the_door() {
    let runtime = after_wren();
    go(
        &runtime,
        &["go landing", "go stairs", "go stairs", "go front"],
    );
    assert_eq!(runtime.current_room_id().unwrap(), "waiting-room");
    let text = runtime.run_turn("look").expect("look").text();
    assert!(text.contains("Tam") && text.contains("Ông"), "{text}");
    go(&runtime, &["go hall", "go playroom"]);
    assert_eq!(runtime.current_room_id().unwrap(), "playroom");
    let state = state(&runtime);
    assert_eq!(
        state.actor_current_room_id(&pack(), "tam"),
        "hall",
        "he does not come in until the room is right"
    );
}

#[test]
fn he_comes_in_once_the_light_is_low_and_the_lamp_is_on_low() {
    let runtime = after_wren();
    go(
        &runtime,
        &[
            "go landing",
            "go stairs",
            "go stairs",
            "go front",
            "go hall",
            "go playroom",
        ],
    );
    go(&runtime, &["do overhead-light:off", "do lamp:on"]);
    assert_eq!(
        state(&runtime).actor_current_room_id(&pack(), "tam"),
        "hall",
        "the lamp is bright, so the room is not right yet"
    );
    go(&runtime, &["do lamp:down"]);
    assert_eq!(
        state(&runtime).actor_current_room_id(&pack(), "tam"),
        "playroom"
    );
}

#[test]
fn tam_and_his_grandfather_are_not_in_the_house_during_wrens_session() {
    let pack = pack();
    let state = WorldState::new(&pack);
    for visitor in ["tam", "ong"] {
        assert!(
            !state.actor_is_in_room(&pack, visitor, "waiting-room"),
            "{visitor} has not arrived yet"
        );
    }
}
