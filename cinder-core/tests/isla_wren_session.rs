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

/// The option that fits each menu's bid, and one that does not. Answers are
/// chosen by id, so the order they are listed in does not matter.
fn fit(menu: &str) -> &'static str {
    match menu {
        "meet" => "meet-reflect",
        "bins" => "bins-reflect",
        "snack-have" => "snack-have-reflect",
        "snack-none" => "snack-none-offer",
        "bag" => "bag-reflect",
        "stories" => "stories-read",
        "fetch-book" => "fetch-book-read",
        "bird-1" => "bird-1-reflect",
        "notebook-bird" => "notebook-bird-reflect",
        "bird-2" => "bird-2-follow",
        "quiet" => "quiet-reflect",
        "bird-3" => "bird-3-reflect",
        "cup" => "cup-reflect",
        "closing" => "closing-plain",
        "bear-1" => "bear-1-reflect",
        "notebook-bear" => "notebook-bear-reflect",
        "bear-2" => "bear-2-reflect",
        "fox-1" => "fox-1-reflect",
        "phone-1" => "phone-1-open",
        "phone-2" => "phone-2-next",
        "note" => "note-write",
        other => panic!("no fitting option known for {other}"),
    }
}

fn miss(menu: &str) -> String {
    format!("{menu}-probe")
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
        text.contains("Wren (to Isla): Today's Thursday, right? My dad brought me today."),
        "{text}"
    );
    assert_eq!(active_menu(&runtime).as_deref(), Some("meet"));
}

#[test]
fn a_fitting_answer_earns_a_point_and_a_miss_does_not() {
    let fit = runtime();
    greet(&fit);
    let text = turn(&fit, "meet-reflect");
    assert!(text.contains("way across town for a job"), "{text}");
    assert_eq!(points(&fit), 1);

    let miss = runtime();
    greet(&miss);
    let text = turn(&miss, "meet-probe");
    assert!(
        text.contains("Wren (to Isla): I don't know. Just because."),
        "{text}"
    );
    assert_eq!(points(&miss), 0);
}

#[test]
fn wren_follows_isla_to_the_playroom_and_the_bins_follow() {
    let runtime = runtime();
    greet(&runtime);
    let text = turn(&runtime, "meet-reflect");
    assert!(text.contains("She stays a step behind you"), "{text}");
    assert_eq!(active_menu(&runtime), None);

    let text = turn(&runtime, "go hall");
    assert!(text.contains("Wren walks beside you."), "{text}");
    let look = turn(&runtime, "look");
    assert!(look.contains("Beside you: Wren."), "{look}");
    assert!(!look.contains("Your party"), "{look}");
    let text = turn(&runtime, "go playroom");
    let heading_pos = text.find("== Playroom ==").expect("playroom heading");
    let scene_pos = text
        .find("Afternoon light slants across the road rug")
        .expect("playroom arrival scene");
    assert!(
        heading_pos < scene_pos,
        "room heading and summary should appear before arrival scene:\n{text}"
    );
    assert!(
        text.contains("Wren (to Isla): These ones live in this house"),
        "{text}"
    );
    assert_eq!(active_menu(&runtime).as_deref(), Some("bins"));
    let state = runtime.export_state().expect("state");
    assert_eq!(state.actor_room_id("wren", "waiting-room"), "playroom");
    assert!(!state.follows_player("wren"), "she stays at the bins");

    let text = turn(&runtime, "bins-reflect");
    assert!(text.contains("otherwise one of them gets lonely"), "{text}");
    assert_eq!(points(&runtime), 2);
    assert_eq!(active_menu(&runtime).as_deref(), Some("snack-none"));
}

#[test]
fn points_survive_a_runtime_reload() {
    let first = runtime();
    greet(&first);
    turn(&first, "meet-reflect");
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
    turn(runtime, "meet-reflect");
    turn(runtime, "go hall");
    turn(runtime, "go playroom");
}

/// Walks to the back steps, reads both messages, writes the note and reads it.
fn finish_after(runtime: &CinderRuntime) -> String {
    let mut text = String::new();
    for step in ["go hall", "go kitchen", "go back yard"] {
        text += &turn(runtime, step);
    }
    text += &answer(runtime, "phone-1", fit("phone-1"));
    text += &answer(runtime, "phone-2", fit("phone-2"));
    text += &answer(runtime, "note", fit("note"));
    text += &turn(runtime, "look at notes from today");
    text
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
        answer(&runtime, menu, fit(menu));
    }
    assert_eq!(points(&runtime), 15);
    let state = runtime.export_state().expect("state");
    assert!(
        state
            .active_objective_stage_ids
            .contains(&"stage-farewell-high".to_string()),
        "{:?}",
        state.active_objective_stage_ids
    );
    let after = finish_after(&runtime);
    assert!(after.contains("She hummed the whole way home"), "{after}");
    assert!(after.contains("She showed me the handle."), "{after}");
    assert_eq!(
        runtime.export_state().expect("state").phase,
        GamePhase::GameEnded
    );
}

#[test]
fn the_first_secret_is_the_bag_and_the_last_is_the_cup() {
    let runtime = runtime();
    start_session(&runtime, true, Some("bird book"));
    assert!(
        runtime.export_state().expect("state").has_item("snack"),
        "Isla should carry the snack into the playroom"
    );
    answer(&runtime, "bins", fit("bins"));
    let text = answer(&runtime, "snack-have", fit("snack-have"));
    assert!(
        !runtime.export_state().expect("state").has_item("snack"),
        "snack should be removed from Isla's tote bag after she gives it to Wren"
    );
    assert!(text.contains("packed Biscuit's toothbrush"), "{text}");
    let text = answer(&runtime, "bag", fit("bag"));
    assert!(
        text.contains("never has to leave anything behind"),
        "{text}"
    );
    for menu in [
        "stories",
        "bird-1",
        "notebook-bird",
        "bird-2",
        "quiet",
        "bird-3",
    ] {
        answer(&runtime, menu, fit(menu));
    }
    let text = turn(&runtime, fit("cup"));
    assert!(
        text.contains("It was Mom's favorite blue cup"),
        "the cup should be told: {text}"
    );
}

#[test]
fn without_a_snack_isla_has_to_fetch_one() {
    let runtime = runtime();
    start_session(&runtime, false, None);
    answer(&runtime, "bins", fit("bins"));
    let text = answer(&runtime, "snack-none", fit("snack-none"));
    assert!(text.contains("I'll wait right here with Biscuit"), "{text}");
    assert_eq!(points(&runtime), 3);
    assert_eq!(
        active_menu(&runtime),
        None,
        "Isla must now physically fetch the snack from the kitchen"
    );
    assert!(
        runtime
            .export_state()
            .expect("state")
            .active_objective_stage_ids
            .contains(&"stage-fetch-snack".to_string())
    );

    turn(&runtime, "go hall");
    turn(&runtime, "go kitchen");
    turn(&runtime, "take snack");
    assert!(runtime.export_state().expect("state").has_item("snack"));
    turn(&runtime, "go hall");
    let return_text = turn(&runtime, "go playroom");
    assert!(
        return_text.contains("Wren (to Isla): Thank you. Four squares is the right number."),
        "{return_text}"
    );
    assert!(
        !runtime.export_state().expect("state").has_item("snack"),
        "snack is removed from Isla's tote bag upon giving it to Wren"
    );
    assert_eq!(active_menu(&runtime).as_deref(), Some("bag"));
}

#[test]
fn poor_answers_close_the_deeper_secrets() {
    let runtime = runtime();
    start_session(&runtime, false, None);
    answer(&runtime, "bins", &miss("bins"));
    answer(&runtime, "snack-none", &miss("snack-none"));
    assert_eq!(
        active_menu(&runtime).as_deref(),
        Some("stories"),
        "no bag secret when the answers have not fit"
    );
}

#[test]
fn the_counting_book_is_the_shallow_book() {
    let runtime = runtime();
    start_session(&runtime, true, Some("bear book"));
    for menu in ["bins", "snack-have", "bag", "stories"] {
        answer(&runtime, menu, fit(menu));
    }
    assert_eq!(active_menu(&runtime).as_deref(), Some("bear-1"));
    for menu in ["bear-1", "notebook-bear", "bear-2", "closing"] {
        answer(&runtime, menu, fit(menu));
    }
    let state = runtime.export_state().expect("state");
    assert!(
        state
            .active_objective_stage_ids
            .iter()
            .any(|id| id.starts_with("stage-farewell")),
        "the session ends with a farewell"
    );
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
        answer(&runtime, menu, fit(menu));
    }
    let text = answer(&runtime, "fox-1", fit("fox-1"));
    assert!(text.contains("goes back to the bins"), "{text}");
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
        answer(&runtime, menu, fit(menu));
    }
    answer(&runtime, "quiet", &miss("quiet"));
    answer(&runtime, "bird-3", fit("bird-3"));
    assert_eq!(
        active_menu(&runtime).as_deref(),
        Some("closing"),
        "no cup when the quiet was not held"
    );
}

#[test]
fn choices_wait_while_isla_is_away_from_wren() {
    let runtime = runtime();
    start_session(&runtime, false, None);
    answer(&runtime, "bins", fit("bins"));
    assert!(runtime.menu_choice_options().unwrap().is_some());

    turn(&runtime, "go hall");
    assert!(
        runtime.current_active_menu_info().unwrap().is_none(),
        "no choices out in the hall"
    );
    assert!(runtime.menu_choice_options().unwrap().is_none());
    // A number typed in the hall answers nothing.
    let text = turn(&runtime, "snack-none-offer");
    assert!(!text.contains("step into the kitchen"), "{text}");
    assert_eq!(active_menu(&runtime).as_deref(), Some("snack-none"));

    turn(&runtime, "go playroom");
    assert!(runtime.current_active_menu_info().unwrap().is_some());
    let text = turn(&runtime, "snack-none-offer");
    assert!(text.contains("step into the kitchen"), "{text}");
}

#[test]
fn a_quieter_session_gets_a_quieter_message_and_note() {
    let runtime = runtime();
    start_session(&runtime, false, Some("bear book"));
    answer(&runtime, "bins", &miss("bins"));
    answer(&runtime, "snack-none", &miss("snack-none"));
    answer(&runtime, "stories", fit("stories"));
    for menu in ["bear-1", "bear-2", "closing"] {
        answer(&runtime, menu, &miss(menu));
    }
    let after = finish_after(&runtime);
    assert!(after.contains("pretty quiet in the car"), "{after}");
    assert!(after.contains("no hurry about the book"), "{after}");
    assert_eq!(
        runtime.export_state().expect("state").phase,
        GamePhase::GameEnded
    );
}

#[test]
fn wren_leaves_the_house_after_the_farewell_and_glorias_waiting_topic_closes() {
    let runtime = runtime();
    start_session(&runtime, true, Some("fox book"));
    for menu in ["bins", "snack-have", "bag", "stories", "fox-1"] {
        answer(&runtime, menu, fit(menu));
    }
    let closing_text = answer(&runtime, "closing", "closing-again");
    assert!(
        closing_text.contains("same place next Thursday"),
        "{closing_text}"
    );
    let state = runtime.export_state().expect("state");
    assert_eq!(state.actor_room_id("wren", "waiting-room"), "");
    turn(&runtime, "go hall");
    turn(&runtime, "go waiting room");
    let topics = runtime
        .panel_options(&cinder_core::content::types::PanelDataSource::ActorTopics)
        .expect("topics");
    assert!(topics.is_empty(), "Gloria's pre-session topic is retired");
}

#[test]
fn the_messages_come_from_mom_and_dr_bell_and_say_who_they_are_to() {
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
        answer(&runtime, menu, fit(menu));
    }
    for step in ["go hall", "go kitchen", "go back yard"] {
        turn(&runtime, step);
    }
    let first = answer(&runtime, "phone-1", fit("phone-1"));
    assert!(first.contains("Mom (to Isla): She hummed"), "{first}");
    let second = answer(&runtime, "phone-2", fit("phone-2"));
    assert!(
        second.contains("Dr. Bell (to Isla): How did it go?"),
        "{second}"
    );
}

#[test]
fn the_session_cannot_go_on_until_isla_has_fetched_a_book() {
    let runtime = runtime();
    start_session(&runtime, true, None);
    for menu in ["bins", "snack-have"] {
        answer(&runtime, menu, fit(menu));
    }
    if active_menu(&runtime).as_deref() == Some("bag") {
        answer(&runtime, "bag", fit("bag"));
    }
    let text = answer(&runtime, "stories", fit("stories"));
    assert!(text.contains("no book"), "{text}");
    assert_eq!(active_menu(&runtime).as_deref(), Some("fetch-book"));

    // Opening the tote again without a book changes nothing.
    let again = answer(&runtime, "fetch-book", fit("fetch-book"));
    assert!(again.contains("no book"), "{again}");
    assert_eq!(active_menu(&runtime).as_deref(), Some("fetch-book"));

    // Fetching one from the book room lets the story begin.
    for step in ["go hall", "go upstairs", "go upstairs", "go book room"] {
        turn(&runtime, step);
    }
    turn(&runtime, "take bear book");
    for step in [
        "go landing",
        "go downstairs",
        "go downstairs",
        "go playroom",
    ] {
        turn(&runtime, step);
    }
    answer(&runtime, "fetch-book", fit("fetch-book"));
    assert_eq!(active_menu(&runtime).as_deref(), Some("bear-1"));
}

#[test]
fn the_to_do_for_a_missing_book_says_where_the_books_are() {
    let pack = load_named_pack("isla", Some("en")).expect("isla loads");
    let stage = pack
        .beats
        .stages
        .iter()
        .find(|stage| stage.id == "stage-fetch-book")
        .expect("fetch book stage");
    assert!(
        stage.summary.contains("book room, at the top of the house"),
        "{}",
        stage.summary
    );
}
