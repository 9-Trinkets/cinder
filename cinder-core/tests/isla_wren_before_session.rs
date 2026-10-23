//! The first phase of the Wren act: Isla on the porch, her notes, her phone.

use cinder_core::content::loader::load_named_pack;
use cinder_core::engine::narrative::NarrativeLineKind;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::WorldState;

const NOTES: [&str; 5] = [
    "note-intake",
    "note-referral",
    "note-session-one",
    "note-session-two",
    "note-schedule",
];

fn runtime() -> CinderRuntime {
    let pack = load_named_pack("isla", Some("en")).expect("isla loads");
    let state = WorldState::new(&pack);
    CinderRuntime::from_state(pack, state, false).expect("runtime creates")
}

#[test]
fn isla_starts_carrying_all_five_notes() {
    let carried = runtime().inventory_items().expect("inventory");
    for note in NOTES {
        assert_eq!(carried.get(note), Some(&1), "{note} not carried");
    }
}

#[test]
fn wren_is_waiting_in_the_waiting_room() {
    let pack = load_named_pack("isla", Some("en")).expect("isla loads");
    let wren = pack.actor("wren").expect("wren exists");
    assert_eq!(wren.room_id, "waiting-room");
}

#[test]
fn dr_bell_texts_as_a_message_on_the_phone() {
    let lines = runtime().drain_scripted_sequences().expect("opening");
    let text = lines
        .0
        .iter()
        .find(|line| line.kind == NarrativeLineKind::Channel)
        .expect("a channel line");
    assert!(text.text.starts_with("Dr. Bell:"), "{}", text.text);
    assert!(text.text.contains("especially last week's"));
}

#[test]
fn reading_a_note_prints_it_and_finishes_the_quest_step() {
    let runtime = runtime();
    let before = runtime.quest_summaries().expect("quests");
    assert_eq!(before.len(), 1);
    assert_eq!(before[0].title, "Get ready for Wren");
    assert_eq!(before[0].status, "active");

    let text = runtime
        .run_turn("look at notes from last week")
        .expect("look")
        .text();
    assert!(text.contains("Did not name the move"), "{text}");

    let after = runtime.quest_summaries().expect("quests");
    assert!(
        after.iter().all(|q| q.status != "active"),
        "quest still active: {after:?}"
    );
}
