//! The opening of the session: greeting Wren, following her, the bins.

use cinder_core::content::loader::load_named_pack;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::WorldState;

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
fn wren_leads_the_way_to_the_playroom_and_the_bins_follow() {
    let runtime = runtime();
    greet(&runtime);
    let text = turn(&runtime, "1");
    assert!(text.contains("She knows the way."), "{text}");
    assert_eq!(active_menu(&runtime), None);

    turn(&runtime, "go hall");
    let text = turn(&runtime, "go playroom");
    assert!(
        text.contains("Wren (to Isla): These ones live here."),
        "{text}"
    );
    assert_eq!(active_menu(&runtime).as_deref(), Some("bins"));

    let text = turn(&runtime, "1");
    assert!(text.contains("And those ones."), "{text}");
    assert_eq!(points(&runtime), 2);
    assert_eq!(active_menu(&runtime), None);
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
