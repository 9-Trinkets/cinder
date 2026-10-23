//! Plays Isla's Hal act through the real runtime using only authored menu options.

use cinder_core::content::loader::load_named_pack;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::WorldState;

fn runtime() -> CinderRuntime {
    let pack = load_named_pack("isla", Some("en")).expect("isla loads");
    let state = WorldState::new(&pack);
    CinderRuntime::from_state(pack, state, false).expect("runtime creates")
}

fn points(runtime: &CinderRuntime) -> i32 {
    runtime
        .export_state()
        .expect("state exports")
        .actor_stat("hal", "points")
}

fn play(runtime: &CinderRuntime, choices: &[&str]) -> Vec<String> {
    choices
        .iter()
        .map(|choice| {
            runtime
                .run_turn(choice)
                .unwrap_or_else(|e| panic!("turn '{choice}' failed: {e}"))
                .text()
        })
        .collect()
}

#[test]
fn the_first_menu_is_open_at_the_start_of_play() {
    let runtime = runtime();
    let info = runtime
        .current_active_menu_info()
        .expect("menu info")
        .expect("an initial stage menu is open");
    assert_eq!(info.options.len(), 5);
}

#[test]
fn fitting_answers_earn_points_and_reach_the_first_secret() {
    let runtime = runtime();
    let texts = play(
        &runtime,
        &["b1-b", "b2-b", "b3-a", "b4-b", "b5-c", "b6-a", "b7-a"],
    );
    assert_eq!(points(&runtime), 7, "seven bids met");
    assert!(
        texts.last().unwrap().contains("two cups every morning"),
        "the first secret is told once the gate is met: {}",
        texts.last().unwrap()
    );
    let held = play(&runtime, &["s1-c"]);
    assert_eq!(points(&runtime), 9, "a held secret scores double");
    assert!(held[0].contains("I wash both"));
}

#[test]
fn reactions_come_before_the_next_line() {
    let runtime = runtime();
    let text = play(&runtime, &["b1-b"]).remove(0);
    let reaction = text.find("Facing the door").expect("reaction present");
    let next = text
        .find("managing perfectly well")
        .expect("next bid present");
    assert!(
        reaction < next,
        "reaction should precede the next bid: {text}"
    );
}

#[test]
fn poor_answers_miss_the_gate_and_skip_the_secret() {
    let runtime = runtime();
    let texts = play(
        &runtime,
        &["b1-d", "b2-a", "b3-c", "b4-d", "b5-b", "b6-b", "b7-d"],
    );
    assert_eq!(points(&runtime), 0);
    assert!(
        !texts.last().unwrap().contains("two cups every morning"),
        "the secret must not appear below the gate"
    );
    assert!(texts.last().unwrap().contains("Back to the bridge"));
}

#[test]
fn a_fumbled_secret_scores_nothing() {
    let runtime = runtime();
    play(
        &runtime,
        &["b1-b", "b2-b", "b3-a", "b4-b", "b5-c", "b6-a", "b7-a"],
    );
    let before = points(&runtime);
    play(&runtime, &["s1-a"]);
    assert_eq!(points(&runtime), before);
}

/// The server rebuilds the runtime from saved state on every turn, so stats
/// earned earlier in the act must survive a reload.
#[test]
fn points_survive_a_reload_between_turns() {
    let pack = load_named_pack("isla", Some("en")).expect("isla loads");
    let mut state = WorldState::new(&pack);
    let mut last_text = String::new();
    for choice in ["b1-b", "b2-b", "b3-a", "b4-b", "b5-c", "b6-a", "b7-a"] {
        let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime");
        last_text = runtime.run_turn(choice).expect("turn").text();
        state = runtime.export_state().expect("state exports");
    }
    assert_eq!(state.actor_stat("hal", "points"), 7);
    assert!(
        last_text.contains("two cups every morning"),
        "the gate must still be met after reloads: {last_text}"
    );
}

#[test]
fn looking_around_leaves_the_pending_choice_open() {
    let runtime = runtime();
    let room = runtime.run_turn("look").expect("look").text();
    assert!(room.contains("Rain streaks the window"), "{room}");
    let hal = runtime.run_turn("look at hal").expect("look at hal").text();
    assert!(hal.contains("seventy-one"), "{hal}");
    assert!(
        runtime
            .current_active_menu_info()
            .expect("menu info")
            .is_some(),
        "the menu stays open after a look"
    );
    assert_eq!(points(&runtime), 0, "looking earns and costs nothing");
}
