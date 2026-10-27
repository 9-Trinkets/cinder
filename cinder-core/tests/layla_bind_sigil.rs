//! The bind sigil: traced with Zayd's lantern at hand, it wakes every allied,
//! living, dormant actor in the room, and lets the lantern release the fallen.

use cinder_core::content::loader::load_named_pack;
use cinder_core::content::types::ContentPack;
use cinder_core::engine::dialogue::ScriptedDialogueGenerator;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::{ActorStance, WorldState};
use std::sync::Arc;

const HALL: &str = "sanctuary_control_hall";

fn runtime_with(setup: impl FnOnce(&ContentPack, &mut WorldState)) -> CinderRuntime {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = HALL.to_string();
    state.clock.turn_number = 1;
    state.story_vars.set_unchecked("knows_bind", "true");
    state
        .combat
        .actor_stats
        .entry("player".to_string())
        .or_default()
        .insert("mp".to_string(), 60);
    setup(&pack, &mut state);
    CinderRuntime::with_dialogue_generator(pack, state, Arc::new(ScriptedDialogueGenerator::new()))
        .expect("runtime creates")
}

fn ally(pack: &ContentPack, s: &mut WorldState, id: &str, room: &str) {
    s.set_actor_stance(pack, id, ActorStance::Allied, true);
    s.actors
        .actor_room_overrides
        .insert(id.to_string(), room.to_string());
}

fn kill(s: &mut WorldState, id: &str) {
    s.combat
        .actor_stats
        .get_mut(id)
        .expect("stats")
        .insert("hp".to_string(), 0);
}

#[test]
fn the_sigil_needs_the_scroll_and_the_lantern_in_the_pack_or_in_hand() {
    // No lantern: nothing is traced.
    let bare = runtime_with(|_, _| {});
    let out = bare.run_turn("trace bind sigil").expect("trace");
    assert!(!out.text().contains("interlocking knot"), "{}", out.text());

    // Lantern in the pack.
    let packed = runtime_with(|_, s| s.add_item("zayd-lantern"));
    let out = packed.run_turn("trace bind sigil").expect("trace");
    assert!(out.text().contains("interlocking knot"), "{}", out.text());

    // Lantern equipped: it has left the pack but is still carried.
    let held = runtime_with(|_, s| {
        s.inventory
            .equipment
            .insert("off-hand".to_string(), "zayd-lantern".to_string());
    });
    let out = held.run_turn("trace bind sigil").expect("trace");
    assert!(out.text().contains("interlocking knot"), "{}", out.text());

    // Lantern but no scroll read.
    let unread = runtime_with(|_, s| {
        s.add_item("zayd-lantern");
        s.story_vars.set_unchecked("knows_bind", "false");
    });
    let out = unread.run_turn("trace bind sigil").expect("trace");
    assert!(!out.text().contains("interlocking knot"), "{}", out.text());
}

fn crowd(pack: &ContentPack, s: &mut WorldState) {
    s.add_item("zayd-lantern");
    s.story_vars.set_unchecked("courtyard_cages_opened", "true");
    // Present and allied: a golem, Sakhra, the queen and a pawn.
    for id in ["golem-dark-nw", "sakhra", "elf-queen-4", "elf-pawn-1"] {
        ally(pack, s, id, HALL);
    }
    // A golem who stayed behind on Floor 5, and one who fell.
    ally(pack, s, "golem-dark-sw", "courtyard_center");
    ally(pack, s, "golem-pale-se", HALL);
    kill(s, "golem-pale-se");
    // A piece that is present but not an ally.
    s.actors
        .actor_room_overrides
        .insert("elf-pawn-2".to_string(), HALL.to_string());
}

#[test]
fn casting_wakes_everyone_present_and_no_one_else() {
    let runtime = runtime_with(crowd);
    let out = runtime.run_turn("trace bind sigil").expect("trace");
    let text = out.text();
    assert!(text.contains("warm amber"), "{text}");
    assert!(
        text.contains("I remember the rings"),
        "Orin's fragment: {text}"
    );

    let state = runtime.export_state().unwrap();
    for id in ["golem-dark-nw", "sakhra", "elf-queen-4", "elf-pawn-1"] {
        assert!(state.is_actor_awakened(id), "{id} should have woken");
    }
    assert!(
        !state.is_actor_awakened("golem-dark-sw"),
        "someone in another room stays asleep"
    );
    assert!(
        !state.is_actor_awakened("elf-pawn-2"),
        "someone who is not an ally stays asleep"
    );
    assert!(
        !state.is_actor_awakened("golem-pale-se"),
        "the fallen do not wake"
    );
}

#[test]
fn the_lantern_releases_a_fallen_golem_once() {
    let runtime = runtime_with(crowd);
    let first = runtime.run_turn("trace bind sigil").expect("trace");
    assert!(first.text().contains("Hana, who wove"), "{}", first.text());

    let second = runtime.run_turn("trace bind sigil").expect("trace");
    assert!(
        !second.text().contains("Hana, who wove"),
        "the release plays once: {}",
        second.text()
    );
}

#[test]
fn with_no_one_to_wake_the_sigil_says_so() {
    let runtime = runtime_with(|_, s| s.add_item("zayd-lantern"));
    let out = runtime.run_turn("trace bind sigil").expect("trace");
    assert!(
        out.text().contains("no one here it can reach"),
        "{}",
        out.text()
    );
}

// --- Siraj and Nur ----------------------------------------------------------------

const REUNION: &str = "rusted hinge learning to move";

fn watch_for_the_reunion(s: &mut WorldState) {
    s.active_objective_stage_ids
        .push("reunion_siraj_nur".to_string());
}

#[test]
fn siraj_and_nur_are_reunited_when_the_lantern_wakes_both() {
    let runtime = runtime_with(|pack, s| {
        s.add_item("zayd-lantern");
        ally(pack, s, "sakhra", HALL);
        ally(pack, s, "elf-queen-4", HALL);
        watch_for_the_reunion(s);
    });
    let first = runtime.run_turn("trace bind sigil").expect("trace");
    assert!(first.text().contains(REUNION), "{}", first.text());
    assert!(
        first.text().contains("close around her"),
        "{}",
        first.text()
    );

    let again = runtime.run_turn("trace bind sigil").expect("trace");
    assert!(!again.text().contains(REUNION), "once only");
}

#[test]
fn a_lone_awakening_waits_until_the_other_wakes() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = HALL.to_string();
    state.clock.turn_number = 1;
    state.story_vars.set_unchecked("knows_bind", "true");
    state.add_item("zayd-lantern");
    state
        .combat
        .actor_stats
        .entry("player".to_string())
        .or_default()
        .insert("mp".to_string(), 60);
    ally(&pack, &mut state, "sakhra", HALL);
    ally(&pack, &mut state, "elf-queen-4", "courtyard_center");
    watch_for_the_reunion(&mut state);
    let runtime = CinderRuntime::with_dialogue_generator(
        pack.clone(),
        state,
        Arc::new(ScriptedDialogueGenerator::new()),
    )
    .expect("runtime creates");

    let first = runtime.run_turn("trace bind sigil").expect("trace");
    assert!(!first.text().contains(REUNION), "{}", first.text());
    let mid = runtime.export_state().unwrap();
    assert!(mid.is_actor_awakened("sakhra"));
    assert!(!mid.is_actor_awakened("elf-queen-4"));

    // The queen joins them, and the next cast wakes her.
    let mut state = mid;
    ally(&pack, &mut state, "elf-queen-4", HALL);
    let runtime = CinderRuntime::with_dialogue_generator(
        pack,
        state,
        Arc::new(ScriptedDialogueGenerator::new()),
    )
    .expect("runtime creates");
    let second = runtime.run_turn("trace bind sigil").expect("trace");
    assert!(second.text().contains(REUNION), "{}", second.text());
}

#[test]
fn entering_floor_six_switches_the_reunion_on() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "courtyard_center".to_string();
    state.clock.turn_number = 1;
    state.acquire_player_item(&pack, "clandestine-sanctuary-token");
    state.active_objective_stage_ids = vec!["mq_infiltrate_sanctuary".to_string()];
    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime creates");

    runtime
        .run_turn("use clandestine-sanctuary-token")
        .expect("token");
    let state = runtime.export_state().unwrap();
    assert!(
        state
            .active_objective_stage_ids
            .contains(&"reunion_siraj_nur".to_string()),
        "{:?}",
        state.active_objective_stage_ids
    );
}
