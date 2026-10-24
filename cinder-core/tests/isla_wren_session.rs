//! The opening of the session: greeting Wren, following her, the bins.

use cinder_core::content::loader::load_named_pack;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::{GamePhase, WorldState};

fn runtime() -> CinderRuntime {
    let pack = load_named_pack("isla", Some("en")).expect("isla loads");
    let state = WorldState::new(&pack);
    CinderRuntime::from_state(pack, state, false).expect("runtime creates")
}

fn turn(runtime: &CinderRuntime, input: &str) -> String {
    runtime
        .run_turn(input)
        .unwrap_or_else(|e| panic!("turn '{input}' failed: {e}"))
        .text()
}

fn points(runtime: &CinderRuntime) -> i32 {
    runtime
        .export_state()
        .expect("state")
        .combat
        .actor_stats
        .get("wren")
        .and_then(|stats| stats.get("points"))
        .copied()
        .unwrap_or(0)
}

fn active_menu(runtime: &CinderRuntime) -> Option<String> {
    runtime.export_state().expect("state").active_menu_id
}

fn greet(runtime: &CinderRuntime) -> String {
    turn(runtime, "go inside");
    turn(runtime, "say wren:greet")
}

#[test]
fn greeting_wren_opens_the_first_bid() {
    let runtime = runtime();
    let text = greet(&runtime);
    assert!(
        text.contains("Wren (to Isla): It's Thursday. Dad brought me."),
        "{text}"
    );
    assert_eq!(active_menu(&runtime).as_deref(), Some("meet"));
}

#[test]
fn a_fitting_answer_earns_a_point_and_a_miss_does_not() {
    let fit = runtime();
    greet(&fit);
    let text = turn(&fit, "1");
    assert!(text.contains("He had a job. A far one."), "{text}");
    assert_eq!(points(&fit), 1);

    let miss = runtime();
    greet(&miss);
    let text = turn(&miss, "3");
    assert!(text.contains("Wren (to Isla): Nothing."), "{text}");
    assert_eq!(points(&miss), 0);
}

#[test]
fn wren_follows_isla_to_the_playroom_and_the_bins_follow() {
    let runtime = runtime();
    greet(&runtime);
    let text = turn(&runtime, "1");
    assert!(text.contains("She stays a step behind you"), "{text}");
    assert_eq!(active_menu(&runtime), None);

    let text = turn(&runtime, "go hall");
    assert!(text.contains("Wren walks beside you."), "{text}");
    let look = turn(&runtime, "look");
    assert!(look.contains("Beside you: Wren."), "{look}");
    assert!(!look.contains("Your party"), "{look}");
    let text = turn(&runtime, "go playroom");
    assert!(
        text.contains("Wren (to Isla): These ones live here."),
        "{text}"
    );
    assert_eq!(active_menu(&runtime).as_deref(), Some("bins"));
    let state = runtime.export_state().expect("state");
    assert_eq!(state.actor_room_id("wren", "waiting-room"), "playroom");
    assert!(!state.follows_player("wren"), "she stays at the bins");

    let text = turn(&runtime, "1");
    assert!(text.contains("And those ones."), "{text}");
    assert_eq!(points(&runtime), 2);
    assert_eq!(active_menu(&runtime).as_deref(), Some("snack-none"));
}

#[test]
fn points_survive_a_runtime_reload() {
    let first = runtime();
    greet(&first);
    turn(&first, "1");
    let state = first.export_state().expect("state");
    let pack = load_named_pack("isla", Some("en")).expect("isla loads");
    let reloaded = CinderRuntime::from_state(pack, state, false).expect("reload");
    assert_eq!(points(&reloaded), 1);
}

/// Speech names who is talking to whom, as `Name (to Other): text`. Apart from
/// that label, authored lines have no parentheses.
#[test]
fn spoken_lines_name_who_is_talking_to_whom() {
    let pack = load_named_pack("isla", Some("en")).expect("isla loads");
    let mut lines: Vec<String> = Vec::new();
    for actor in &pack.actors {
        for topic in &actor.topics {
            lines.extend(topic.narrative_lines.clone());
        }
    }
    for menu in &pack.menus {
        for option in &menu.options {
            lines.extend(option.narrative_lines.clone());
        }
    }
    for stage in &pack.beats.stages {
        lines.extend(stage.narrative_lines.clone());
    }
    for line in lines {
        let speech = line
            .split_once(": ")
            .filter(|(label, _)| {
                label
                    .chars()
                    .all(|c| c.is_alphabetic() || " .()".contains(c))
            })
            .filter(|(label, _)| *label != "Book")
            .filter(|(label, _)| !label.contains(' ') || label.contains("(to "));
        if let Some((label, body)) = speech {
            assert!(
                label.contains("(to "),
                "speech does not say who it is to: {line}"
            );
            assert!(!body.contains('(') && !body.contains(')'), "{line}");
        } else {
            assert!(!line.contains('(') && !line.contains(')'), "{line}");
        }
    }
}

// ---------------------------------------------------------------- the whole session

fn take_from(runtime: &CinderRuntime, route: &[&str], item: &str) {
    for step in route {
        turn(runtime, step);
    }
    turn(runtime, &format!("take {item}"));
}

/// Prepares as the player may: optionally the snack and a book, then greets
/// Wren and walks her to the playroom, answering the first two bids with `1`.
fn start_session(runtime: &CinderRuntime, snack: bool, book: Option<&str>) {
    turn(runtime, "go inside");
    turn(runtime, "go hall");
    if snack {
        take_from(runtime, &["go kitchen"], "snack");
        turn(runtime, "go hall");
    }
    if let Some(book) = book {
        take_from(
            runtime,
            &["go upstairs", "go upstairs", "go book room"],
            book,
        );
        for step in ["go landing", "go downstairs", "go downstairs"] {
            turn(runtime, step);
        }
    }
    turn(runtime, "go waiting room");
    turn(runtime, "say wren:greet");
    turn(runtime, "1");
    turn(runtime, "go hall");
    turn(runtime, "go playroom");
}

fn answer(runtime: &CinderRuntime, menu: &str, choice: &str) -> String {
    assert_eq!(
        active_menu(runtime).as_deref(),
        Some(menu),
        "expected the {menu} menu to be open"
    );
    turn(runtime, choice)
}

#[test]
fn the_best_session_with_the_right_book_reaches_every_secret_and_the_best_drawing() {
    let runtime = runtime();
    start_session(&runtime, true, Some("bird book"));
    for menu in [
        "bins",
        "snack-have",
        "bag",
        "stories",
        "bird-1",
        "notebook-bird",
        "bird-2",
        "quiet",
        "bird-3",
        "cup",
        "closing",
    ] {
        answer(&runtime, menu, "1");
    }
    assert_eq!(points(&runtime), 15);
    let state = runtime.export_state().expect("state");
    assert_eq!(state.phase, GamePhase::GameEnded);
    assert!(
        state
            .active_objective_stage_ids
            .contains(&"stage-farewell-high".to_string()),
        "{:?}",
        state.active_objective_stage_ids
    );
}

#[test]
fn the_first_secret_is_the_bag_and_the_last_is_the_cup() {
    let runtime = runtime();
    start_session(&runtime, true, Some("bird book"));
    answer(&runtime, "bins", "1");
    let text = answer(&runtime, "snack-have", "1");
    assert!(text.contains("Rabbit's got his toothbrush"), "{text}");
    let text = answer(&runtime, "bag", "1");
    assert!(text.contains("He never has to stay anywhere"), "{text}");
    for menu in [
        "stories",
        "bird-1",
        "notebook-bird",
        "bird-2",
        "quiet",
        "bird-3",
    ] {
        answer(&runtime, menu, "1");
    }
    let text = turn(&runtime, "1");
    assert!(
        text.contains("It was Mom's. I was making tea"),
        "the cup should be told: {text}"
    );
}

#[test]
fn without_a_snack_isla_has_to_fetch_one() {
    let runtime = runtime();
    start_session(&runtime, false, None);
    answer(&runtime, "bins", "1");
    let text = answer(&runtime, "snack-none", "2");
    assert!(text.contains("The kitchen is warm"), "{text}");
    assert_eq!(points(&runtime), 3);
}

#[test]
fn poor_answers_close_the_deeper_secrets() {
    let runtime = runtime();
    start_session(&runtime, false, None);
    answer(&runtime, "bins", "4");
    answer(&runtime, "snack-none", "4");
    assert_eq!(
        active_menu(&runtime).as_deref(),
        Some("stories"),
        "no bag secret when the answers have not fit"
    );
}

#[test]
fn the_default_book_is_the_shallow_counting_book() {
    let runtime = runtime();
    start_session(&runtime, true, None);
    for menu in ["bins", "snack-have", "bag", "stories"] {
        answer(&runtime, menu, "1");
    }
    assert_eq!(active_menu(&runtime).as_deref(), Some("bear-1"));
    for menu in ["bear-1", "notebook-bear", "bear-2", "closing"] {
        answer(&runtime, menu, "1");
    }
    let state = runtime.export_state().expect("state");
    assert_eq!(state.phase, GamePhase::GameEnded, "the session ends");
    assert!(
        !state.completed_stage_ids.contains("stage-quiet"),
        "the counting book never reaches the quiet secret"
    );
}

#[test]
fn the_book_about_two_houses_makes_wren_retreat() {
    let runtime = runtime();
    start_session(&runtime, true, Some("fox book"));
    for menu in ["bins", "snack-have", "bag", "stories"] {
        answer(&runtime, menu, "1");
    }
    let text = answer(&runtime, "fox-1", "1");
    assert!(text.contains("goes to the bins"), "{text}");
    assert_eq!(active_menu(&runtime).as_deref(), Some("closing"));
}

#[test]
fn the_cup_needs_the_quiet_secret_to_have_been_held() {
    let runtime = runtime();
    start_session(&runtime, true, Some("bird book"));
    for menu in [
        "bins",
        "snack-have",
        "bag",
        "stories",
        "bird-1",
        "notebook-bird",
        "bird-2",
    ] {
        answer(&runtime, menu, "1");
    }
    answer(&runtime, "quiet", "3");
    answer(&runtime, "bird-3", "1");
    assert_eq!(
        active_menu(&runtime).as_deref(),
        Some("closing"),
        "no cup when the quiet was not held"
    );
}
