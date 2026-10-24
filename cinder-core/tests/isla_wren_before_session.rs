//! The first phase of the Wren act: Isla on the porch, her notes, her phone.

use cinder_core::content::loader::load_named_pack;
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
fn the_list_starts_with_only_reading_the_notes() {
    let runtime = runtime();
    assert_eq!(
        quest_status(&runtime),
        vec![("Read your notes".to_string(), "active".to_string())]
    );
}

#[test]
fn reading_a_note_prints_it_and_moves_the_list_on_to_greeting_wren() {
    let runtime = runtime();
    let text = runtime
        .run_turn("look at notes from last week")
        .expect("look")
        .text();
    assert!(text.contains("Did not name the move"), "{text}");

    assert_eq!(
        quest_status(&runtime),
        vec![
            ("Read your notes".to_string(), "completed".to_string()),
            ("Greet Wren".to_string(), "active".to_string()),
        ]
    );
}

#[test]
fn greeting_wren_without_reading_the_notes_skips_straight_to_the_session() {
    let runtime = runtime();
    runtime.run_turn("go inside").expect("go inside");
    runtime.run_turn("say wren:greet").expect("greet");
    let mut quests: Vec<_> = runtime
        .current_objective_summaries()
        .expect("objectives")
        .into_iter()
        .filter_map(|o| o.quest_id)
        .collect();
    quests.sort();
    assert_eq!(quests, vec!["session"]);
}

#[test]
fn the_snack_and_the_book_are_not_on_the_list_until_wren_needs_them() {
    let runtime = runtime();
    runtime.run_turn("go inside").expect("go inside");
    runtime.run_turn("say wren:greet").expect("greet");
    let titles: Vec<_> = quest_status(&runtime).into_iter().map(|q| q.0).collect();
    assert!(
        !titles
            .iter()
            .any(|t| t == "Choose a book" || t == "Bring a snack")
    );
}

fn quest_status(runtime: &CinderRuntime) -> Vec<(String, String)> {
    runtime
        .quest_summaries()
        .unwrap()
        .into_iter()
        .map(|q| (q.title, q.status))
        .collect()
}

#[test]
fn the_talk_panel_offers_what_to_say_to_the_people_in_the_room() {
    let runtime = runtime();
    runtime.run_turn("go inside").expect("go inside");
    let options = runtime
        .panel_options(&cinder_core::content::types::PanelDataSource::ActorTopics)
        .expect("topics");
    let titles: Vec<_> = options.iter().map(|o| o.title.as_str()).collect();
    assert_eq!(titles, vec!["Greet Wren", "Ask Gloria about Wren"]);
}

#[test]
fn saying_a_topic_plays_it_once_and_it_is_no_longer_offered() {
    let runtime = runtime();
    runtime.run_turn("go inside").expect("go inside");
    let text = runtime.run_turn("say wren:greet").expect("say").text();
    assert!(
        text.contains("Wren (to Isla): Today's Thursday, right? My dad brought me today."),
        "{text}"
    );

    let options = runtime
        .panel_options(&cinder_core::content::types::PanelDataSource::ActorTopics)
        .expect("topics");
    assert_eq!(options.len(), 1);
    assert_eq!(options[0].title, "Ask Gloria about Wren");

    let typed = runtime
        .run_turn("say ask gloria about wren")
        .expect("typed")
        .text();
    assert!(typed.contains("snail doesn't seem to mind"), "{typed}");
}

#[test]
fn taking_a_second_book_returns_the_first_to_the_shelf() {
    let runtime = runtime();
    for command in [
        "go inside",
        "go hall",
        "go upstairs",
        "go upstairs",
        "go book room",
    ] {
        runtime.run_turn(command).expect(command);
    }
    runtime
        .run_turn("take two houses for fox")
        .expect("take fox");
    assert_eq!(runtime.inventory_items().unwrap().get("book-fox"), Some(&1));

    let swap = runtime
        .run_turn("take the bird who carried her nest")
        .expect("take bird")
        .text();
    assert!(
        swap.contains("You put Two Houses for Fox back on the shelf"),
        "{swap}"
    );
    let carried = runtime.inventory_items().unwrap();
    assert_eq!(carried.get("book-bird"), Some(&1));
    assert_eq!(carried.get("book-fox"), None);
}

#[test]
fn topics_are_not_offered_where_nobody_is_listening() {
    let runtime = runtime();
    let options = runtime
        .panel_options(&cinder_core::content::types::PanelDataSource::ActorTopics)
        .expect("topics");
    assert!(options.is_empty(), "porch has no one to talk to");
}

#[test]
fn the_book_room_and_kitchen_start_stocked_for_taking() {
    let runtime = runtime();
    for command in ["go inside", "go hall", "go kitchen"] {
        runtime.run_turn(command).expect(command);
    }
    let text = runtime.run_turn("take snack").expect("take").text();
    assert!(text.to_lowercase().contains("clementine"), "{text}");
    assert_eq!(runtime.inventory_items().unwrap().get("snack"), Some(&1));
}
