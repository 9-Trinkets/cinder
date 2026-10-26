//! The second act: Leo, from the end of Wren's act to the end of the game.

use cinder_core::content::loader::load_named_pack;
use cinder_core::content::types::{MovieStyle, PanelDataSource};
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::{GamePhase, WorldState};

fn turn(runtime: &CinderRuntime, input: &str) -> String {
    runtime
        .run_turn(input)
        .unwrap_or_else(|e| panic!("turn '{input}' failed: {e}"))
        .text()
}

fn active_menu(runtime: &CinderRuntime) -> Option<String> {
    runtime.export_state().expect("state").active_menu_id
}

fn answer(runtime: &CinderRuntime, menu: &str, choice: &str) -> String {
    assert_eq!(
        active_menu(runtime).as_deref(),
        Some(menu),
        "expected the {menu} menu to be open"
    );
    turn(runtime, choice)
}

fn points(runtime: &CinderRuntime) -> i32 {
    runtime
        .export_state()
        .expect("state")
        .combat
        .actor_stats
        .get("leo")
        .and_then(|stats| stats.get("points"))
        .copied()
        .unwrap_or(0)
}

fn active_stages(runtime: &CinderRuntime) -> Vec<String> {
    runtime
        .export_state()
        .expect("state")
        .active_objective_stage_ids
}

/// The option that fits each menu's bid. Answers are chosen by id, so the order
/// they are listed in does not matter.
fn fit(menu: &str) -> &'static str {
    match menu {
        "leo-meet" => "leo-meet-play",
        "leo-clinic" => "leo-clinic-play",
        "leo-upright" => "leo-upright-reflect",
        "leo-snack-have" => "leo-snack-have-play",
        "leo-snack-none" => "leo-snack-none-play",
        "leo-signs" => "leo-signs-reflect",
        "leo-stories" => "leo-stories-read",
        "leo-owl-1" => "leo-owl-1-reflect",
        "leo-owl-2" => "leo-owl-2-follow",
        "leo-resting" => "leo-resting-reflect",
        "leo-owl-3" => "leo-owl-3-plain",
        "leo-light" => "leo-light-reflect",
        "leo-rabbit-1" => "leo-rabbit-1-play",
        "leo-rabbit-2" => "leo-rabbit-2-play",
        "leo-clouds-1" => "leo-clouds-1-close",
        "leo-closing" => "leo-closing-plain",
        "phone-1" => "phone-1-open",
        "phone-2" => "phone-2-next",
        "note" => "note-write",
        other => panic!("no fitting option known for {other}"),
    }
}

/// An option that does not fit the bid, and scores nothing.
fn miss(menu: &str) -> &'static str {
    match menu {
        "leo-meet" => "leo-meet-probe",
        "leo-clinic" => "leo-clinic-probe",
        "leo-upright" => "leo-upright-probe",
        "leo-snack-have" => "leo-snack-have-probe",
        "leo-snack-none" => "leo-snack-none-probe",
        "leo-signs" => "leo-signs-probe",
        "leo-owl-1" => "leo-owl-1-probe",
        "leo-owl-2" => "leo-owl-2-push",
        "leo-resting" => "leo-resting-probe",
        "leo-owl-3" => "leo-owl-3-probe",
        "leo-light" => "leo-light-probe",
        "leo-rabbit-1" => "leo-rabbit-1-probe",
        "leo-rabbit-2" => "leo-rabbit-2-probe",
        "leo-closing" => "leo-closing-probe",
        other => panic!("no missing option known for {other}"),
    }
}

/// Wren's act by its quickest road, then the world carries on into Leo's. The
/// second value is what the continuation says, since the server shows it right
/// after the card.
fn episode_two() -> (CinderRuntime, String) {
    let pack = load_named_pack("isla", Some("en")).expect("isla loads");
    let state = WorldState::new(&pack);
    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime creates");
    for step in [
        "go inside",
        "go hall",
        "go upstairs",
        "go upstairs",
        "go book room",
    ] {
        turn(&runtime, step);
    }
    turn(&runtime, "take fox book");
    for step in [
        "go landing",
        "go downstairs",
        "go downstairs",
        "go waiting room",
        "say wren:greet",
        "meet-probe",
        "go hall",
        "go playroom",
        "bins-probe",
        "snack-none-probe",
        "stories-read",
        "fox-1-probe",
        "closing-probe",
        "go hall",
        "go kitchen",
        "go back yard",
        "phone-1-open",
        "phone-2-next",
        "note-write",
        "look at notes from today",
    ] {
        turn(&runtime, step);
    }
    assert_eq!(
        runtime.export_state().expect("state").phase,
        GamePhase::ActEnded
    );
    let lines = runtime.continue_after_act().expect("the act continues");
    let text = lines
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    (runtime, text)
}

/// Prepares as the player may, then greets Leo and walks him to the playroom.
fn start_session(runtime: &CinderRuntime, basket: bool, snack: bool, book: Option<&str>) {
    turn(runtime, "go inside");
    turn(runtime, "go hall");
    if snack {
        turn(runtime, "go kitchen");
        turn(runtime, "take pretzels");
        turn(runtime, "go hall");
    }
    if basket || book.is_some() {
        turn(runtime, "go upstairs");
    }
    if basket {
        turn(runtime, "go quiet room");
        turn(runtime, "take basket of stuffed animals");
        turn(runtime, "go landing");
    }
    if let Some(book) = book {
        turn(runtime, "go upstairs");
        turn(runtime, "go book room");
        turn(runtime, &format!("take {book}"));
        turn(runtime, "go landing");
        turn(runtime, "go downstairs");
    }
    if basket || book.is_some() {
        turn(runtime, "go downstairs");
    }
    turn(runtime, "go waiting room");
    turn(runtime, "say leo:greet");
    turn(runtime, fit("leo-meet"));
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
    text += &turn(runtime, "look at leo today");
    text
}

/// Every menu of the owl book's road, in order, from the first bid after the
/// walk to the playroom to the goodbye.
const OWL_ROAD: [&str; 11] = [
    "leo-clinic",
    "leo-upright",
    "leo-snack-have",
    "leo-signs",
    "leo-stories",
    "leo-owl-1",
    "leo-owl-2",
    "leo-resting",
    "leo-owl-3",
    "leo-light",
    "leo-closing",
];

fn play_owl_to_the_end(runtime: &CinderRuntime) {
    for menu in OWL_ROAD {
        answer(runtime, menu, fit(menu));
    }
}

// -------------------------------------------------------------- the opening

#[test]
fn episode_two_opens_on_the_porch_with_leos_notes() {
    let (runtime, text) = episode_two();
    assert!(text.contains("early February"), "{text}");
    assert!(text.contains("puffy red coat"), "{text}");

    let state = runtime.export_state().expect("state");
    assert_eq!(state.phase, GamePhase::Active);
    assert_eq!(state.current_room_id, "porch");
    assert_eq!(state.actor_room_id("leo", ""), "waiting-room");
    assert_eq!(state.actor_room_id("wren", "waiting-room"), "");

    let carried = runtime.inventory_items().expect("inventory");
    for page in [
        "leo-intake",
        "leo-referral",
        "leo-session-one",
        "leo-schedule",
        "journal-index-two",
        "note-intake",
        "note-three-low",
    ] {
        assert_eq!(carried.get(page), Some(&1), "{page} is not in the journal");
    }
    assert_eq!(
        carried.get("journal-index"),
        None,
        "the new index replaces the old one"
    );
    assert_eq!(carried.get("book-fox"), None, "Wren's book is put away");
}

#[test]
fn the_notes_hold_the_clue_about_sleep_and_the_one_about_keeping_watch() {
    let (runtime, _) = episode_two();
    let schedule = turn(&runtime, "look at leo schedule");
    assert!(
        schedule.contains("Be careful with \"sleep\" and \"tired.\""),
        "{schedule}"
    );
    let week_one = turn(&runtime, "look at leo week one");
    assert!(week_one.contains("night watchman at the zoo"), "{week_one}");
    let intake = turn(&runtime, "look at leo intake");
    assert!(intake.contains("very old and very tired"), "{intake}");
    let referral = turn(&runtime, "look at leo referral");
    assert!(referral.contains("tell him the number"), "{referral}");
}

#[test]
fn the_house_is_restocked_for_leo() {
    let (runtime, _) = episode_two();
    for step in ["go inside", "go hall", "go kitchen"] {
        turn(&runtime, step);
    }
    let shelf = turn(&runtime, "look at snack shelf");
    assert!(shelf.contains("pretzel sticks and oat cookies"), "{shelf}");
    for step in ["go hall", "go upstairs", "go quiet room"] {
        turn(&runtime, step);
    }
    let animals = turn(&runtime, "look at stuffed animals");
    assert!(animals.contains("basket of stuffed animals"), "{animals}");
    for step in ["go landing", "go upstairs", "go book room"] {
        turn(&runtime, step);
    }
    let shelves = turn(&runtime, "look at shelves");
    for title in [
        "The Little Owl Who Kept Watch",
        "Doctor Rabbit's Busy Clinic",
        "The Long Sleep in the Clouds",
    ] {
        assert!(shelves.contains(title), "{shelves}");
    }
    assert!(
        !shelves.contains("Bear Counts to Ten") && !shelves.contains("Two Houses for Fox"),
        "Wren's books are gone: {shelves}"
    );
}

#[test]
fn the_list_starts_with_reading_the_notes_and_greeting_skips_ahead() {
    let (runtime, _) = episode_two();
    let quests = |runtime: &CinderRuntime| {
        runtime
            .quest_summaries()
            .unwrap()
            .into_iter()
            .filter(|q| q.status == "active")
            .map(|q| q.title)
            .collect::<Vec<_>>()
    };
    assert_eq!(quests(&runtime), vec!["Read your notes"]);
    turn(&runtime, "look at leo schedule");
    assert_eq!(quests(&runtime), vec!["Greet Leo"]);
    turn(&runtime, "go inside");
    turn(&runtime, "say leo:greet");
    assert_eq!(quests(&runtime), vec!["The session"]);
}

#[test]
fn glorias_topic_about_leo_closes_once_he_leaves_the_waiting_room() {
    let (runtime, _) = episode_two();
    turn(&runtime, "go inside");
    let topics = |runtime: &CinderRuntime| {
        runtime
            .panel_options(&PanelDataSource::ActorInteractions)
            .expect("topics")
            .into_iter()
            .map(|o| o.title)
            .collect::<Vec<_>>()
    };
    assert_eq!(topics(&runtime), vec!["Greet Leo", "Ask Gloria about Leo"]);
    let text = turn(&runtime, "say ask gloria about leo");
    assert!(text.contains("asleep or stuck"), "{text}");
    turn(&runtime, "say leo:greet");
    turn(&runtime, fit("leo-meet"));
    turn(&runtime, "go hall");
    turn(&runtime, "go playroom");
    turn(&runtime, "go hall");
    turn(&runtime, "go waiting room");
    assert!(topics(&runtime).is_empty());
}

// ------------------------------------------------------------- the first bids

#[test]
fn greeting_leo_opens_the_first_bid() {
    let (runtime, _) = episode_two();
    turn(&runtime, "go inside");
    let text = turn(&runtime, "say leo:greet");
    assert!(
        text.contains(
            "Leo (to Isla): Okay. The clinic is open today. Did you get the patients READY?"
        ),
        "{text}"
    );
    assert_eq!(active_menu(&runtime).as_deref(), Some("leo-meet"));
}

#[test]
fn a_fitting_answer_earns_a_point_and_a_miss_does_not() {
    let (fit_runtime, _) = episode_two();
    turn(&fit_runtime, "go inside");
    turn(&fit_runtime, "say leo:greet");
    let text = turn(&fit_runtime, fit("leo-meet"));
    assert!(text.contains("You're the nurse. Follow me."), "{text}");
    assert_eq!(points(&fit_runtime), 1);

    let (miss_runtime, _) = episode_two();
    turn(&miss_runtime, "go inside");
    turn(&miss_runtime, "say leo:greet");
    let text = turn(&miss_runtime, miss("leo-meet"));
    assert!(
        text.contains("Nurses don't ask that. Nurses say, Yes, Doctor."),
        "{text}"
    );
    assert_eq!(points(&miss_runtime), 0);
}

#[test]
fn leo_follows_isla_to_the_playroom_and_the_clinic_opens() {
    let (runtime, _) = episode_two();
    turn(&runtime, "go inside");
    turn(&runtime, "say leo:greet");
    turn(&runtime, fit("leo-meet"));
    let text = turn(&runtime, "go hall");
    assert!(text.contains("Leo walks beside you."), "{text}");
    let arrival = turn(&runtime, "go playroom");
    assert!(arrival.contains("== Playroom =="), "{arrival}");
    assert!(
        arrival.contains("Leo (to Isla): You're the chart nurse. Write down everything I say."),
        "{arrival}"
    );
    let state = runtime.export_state().expect("state");
    assert_eq!(state.actor_room_id("leo", "waiting-room"), "playroom");
    assert!(!state.follows_player("leo"), "he stays at the clinic");
}

#[test]
fn with_the_basket_the_patients_are_named_and_the_first_secret_follows() {
    let (runtime, _) = episode_two();
    start_session(&runtime, true, true, Some("owl book"));
    assert_eq!(active_menu(&runtime).as_deref(), Some("leo-clinic"));
    let text = answer(&runtime, "leo-clinic", fit("leo-clinic"));
    assert!(
        text.contains("Twenty-four beats. Write that down."),
        "{text}"
    );
    assert!(
        text.contains("Nobody lies down in my hospital. Lying down makes you worse."),
        "{text}"
    );
    assert_eq!(active_menu(&runtime).as_deref(), Some("leo-upright"));
    assert!(
        !runtime
            .inventory_items()
            .unwrap()
            .contains_key("patient-basket"),
        "the patients are out of the tote and in the clinic"
    );
    let text = answer(&runtime, "leo-upright", fit("leo-upright"));
    assert!(text.contains("first rule"), "{text}");
    assert_eq!(points(&runtime), 4);
}

#[test]
fn without_the_basket_leo_sends_isla_upstairs_for_the_patients() {
    let (runtime, _) = episode_two();
    start_session(&runtime, false, true, Some("owl book"));
    let state = runtime.export_state().expect("state");
    assert!(
        state
            .active_objective_stage_ids
            .contains(&"stage-leo-fetch-basket".to_string()),
        "{:?}",
        state.active_objective_stage_ids
    );
    assert_eq!(active_menu(&runtime), None);

    for step in ["go hall", "go upstairs", "go quiet room"] {
        turn(&runtime, step);
    }
    turn(&runtime, "take basket of stuffed animals");
    for step in ["go landing", "go downstairs"] {
        turn(&runtime, step);
    }
    let text = turn(&runtime, "go playroom");
    assert!(
        text.contains("About time, Nurse. First patients."),
        "{text}"
    );
    assert_eq!(active_menu(&runtime).as_deref(), Some("leo-clinic"));
}

#[test]
fn without_pretzels_isla_has_to_fetch_them() {
    let (runtime, _) = episode_two();
    start_session(&runtime, true, false, Some("owl book"));
    answer(&runtime, "leo-clinic", fit("leo-clinic"));
    answer(&runtime, "leo-upright", fit("leo-upright"));
    let text = answer(&runtime, "leo-snack-none", fit("leo-snack-none"));
    assert!(text.contains("Kitchen shelf."), "{text}");
    assert_eq!(active_menu(&runtime), None);
    assert!(active_stages(&runtime).contains(&"stage-leo-fetch-snack".to_string()));

    for step in ["go hall", "go kitchen"] {
        turn(&runtime, step);
    }
    turn(&runtime, "take pretzels");
    turn(&runtime, "go hall");
    let text = turn(&runtime, "go playroom");
    assert!(
        text.contains("Leo (to Isla): Ninety-eight. He's still warm."),
        "{text}"
    );
    assert!(
        !runtime.inventory_items().unwrap().contains_key("leo-snack"),
        "the snack is used up"
    );
    assert_eq!(active_menu(&runtime).as_deref(), Some("leo-signs"));
}

#[test]
fn a_miss_at_the_snack_skips_the_trip_to_the_kitchen() {
    let (runtime, _) = episode_two();
    start_session(&runtime, true, false, Some("owl book"));
    answer(&runtime, "leo-clinic", fit("leo-clinic"));
    answer(&runtime, "leo-upright", fit("leo-upright"));
    answer(&runtime, "leo-snack-none", miss("leo-snack-none"));
    assert!(
        !active_stages(&runtime).contains(&"stage-leo-fetch-snack".to_string()),
        "Leo has moved on"
    );
    assert_eq!(active_menu(&runtime).as_deref(), Some("leo-signs"));
}

#[test]
fn without_a_book_the_next_to_do_is_to_choose_one_and_bring_it_back() {
    let (runtime, _) = episode_two();
    start_session(&runtime, true, true, None);
    for menu in ["leo-clinic", "leo-upright", "leo-snack-have", "leo-signs"] {
        answer(&runtime, menu, fit(menu));
    }
    let step = |runtime: &CinderRuntime| {
        runtime
            .current_objective_summaries()
            .unwrap()
            .into_iter()
            .map(|o| o.summary)
            .collect::<Vec<_>>()
    };
    // The story cannot start yet, and the list says why.
    assert_eq!(active_menu(&runtime), None);
    assert_eq!(
        step(&runtime),
        vec!["Choose a picture book from the book room, at the top of the house."]
    );

    for command in ["go hall", "go upstairs", "go upstairs", "go book room"] {
        turn(&runtime, command);
    }
    turn(&runtime, "take rabbit book");
    assert_eq!(
        step(&runtime),
        vec!["Take the book back to Leo in the playroom."]
    );
    for command in [
        "go landing",
        "go downstairs",
        "go downstairs",
        "go playroom",
    ] {
        turn(&runtime, command);
    }
    assert_eq!(active_menu(&runtime).as_deref(), Some("leo-stories"));
    answer(&runtime, "leo-stories", fit("leo-stories"));
    assert_eq!(active_menu(&runtime).as_deref(), Some("leo-rabbit-1"));
}

#[test]
fn an_empty_tote_at_the_story_still_sends_isla_for_a_book() {
    let (runtime, _) = episode_two();
    start_session(&runtime, true, true, Some("owl book"));
    for menu in ["leo-clinic", "leo-upright", "leo-snack-have", "leo-signs"] {
        answer(&runtime, menu, fit(menu));
    }
    // The book is gone from her tote by the time the story starts.
    let mut state = runtime.export_state().expect("state");
    assert!(state.remove_item("book-owl"));
    let pack = load_named_pack("isla", Some("en")).expect("isla loads");
    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime");
    let text = answer(&runtime, "leo-stories", fit("leo-stories"));
    assert!(text.contains("Nurse, go pick a book."), "{text}");
    assert!(active_stages(&runtime).contains(&"stage-leo-pick-book".to_string()));
}

// ------------------------------------------------------------ the whole session

#[test]
fn the_best_session_with_the_owl_book_reaches_every_secret_and_the_high_ending() {
    let (runtime, _) = episode_two();
    start_session(&runtime, true, true, Some("owl book"));
    play_owl_to_the_end(&runtime);
    assert_eq!(points(&runtime), 15);
    assert!(
        active_stages(&runtime).contains(&"stage-leo-farewell-high".to_string()),
        "{:?}",
        active_stages(&runtime)
    );
    assert!(
        runtime
            .inventory_items()
            .unwrap()
            .contains_key("leo-chart-high"),
        "the chart is folded into the journal"
    );
    let after = finish_after(&runtime);
    assert!(after.contains("I closed my eyes on the couch"), "{after}");
    assert!(after.contains("red squeeze light Pop gave him"), "{after}");
    assert_eq!(
        runtime.export_state().expect("state").phase,
        GamePhase::GameEnded
    );
    let card = runtime
        .consume_pending_projector_sequence()
        .expect("movie lookup")
        .expect("the episode ends with a journal card");
    assert_eq!(card.style, MovieStyle::Card);
    assert_eq!(card.frames[0].heading, "Tuesday, February 4");
    assert!(
        card.frames[0]
            .text
            .contains("red squeeze light Pop gave him"),
        "the card quotes the note Isla wrote: {}",
        card.frames[0].text
    );
}

#[test]
fn each_secret_is_heard_in_the_order_the_bible_gives() {
    let (runtime, _) = episode_two();
    start_session(&runtime, true, true, Some("owl book"));
    let mut heard = Vec::new();
    for menu in OWL_ROAD {
        let text = answer(&runtime, menu, fit(menu));
        for (secret, words) in [
            ("upright", "Nobody lies down in my hospital"),
            ("signs", "three white hairs"),
            ("resting", "I'm just resting my eyes"),
            ("light", "Miller's Hardware"),
        ] {
            if text.contains(words) {
                heard.push(secret);
            }
        }
    }
    assert_eq!(heard, vec!["upright", "signs", "resting", "light"]);
}

#[test]
fn poor_answers_close_the_deeper_secrets() {
    let (runtime, _) = episode_two();
    start_session(&runtime, true, true, Some("owl book"));
    answer(&runtime, "leo-clinic", miss("leo-clinic"));
    answer(&runtime, "leo-upright", miss("leo-upright"));
    answer(&runtime, "leo-snack-have", miss("leo-snack-have"));
    assert_eq!(
        active_menu(&runtime).as_deref(),
        Some("leo-stories"),
        "no warning signs when the answers have not fit"
    );
    answer(&runtime, "leo-stories", fit("leo-stories"));
    answer(&runtime, "leo-owl-1", miss("leo-owl-1"));
    answer(&runtime, "leo-owl-2", miss("leo-owl-2"));
    assert_eq!(
        active_menu(&runtime).as_deref(),
        Some("leo-owl-3"),
        "no resting secret on so few points"
    );
}

#[test]
fn the_light_needs_the_resting_secret_to_have_been_held() {
    let (runtime, _) = episode_two();
    start_session(&runtime, true, true, Some("owl book"));
    for menu in [
        "leo-clinic",
        "leo-upright",
        "leo-snack-have",
        "leo-signs",
        "leo-stories",
        "leo-owl-1",
        "leo-owl-2",
    ] {
        answer(&runtime, menu, fit(menu));
    }
    answer(&runtime, "leo-resting", miss("leo-resting"));
    answer(&runtime, "leo-owl-3", fit("leo-owl-3"));
    assert_eq!(
        active_menu(&runtime).as_deref(),
        Some("leo-closing"),
        "no light when the resting secret was not held"
    );
}

#[test]
fn the_rabbit_book_is_the_shallow_book() {
    let (runtime, _) = episode_two();
    start_session(&runtime, true, true, Some("rabbit book"));
    for menu in [
        "leo-clinic",
        "leo-upright",
        "leo-snack-have",
        "leo-signs",
        "leo-stories",
    ] {
        answer(&runtime, menu, fit(menu));
    }
    assert_eq!(active_menu(&runtime).as_deref(), Some("leo-rabbit-1"));
    for menu in ["leo-rabbit-1", "leo-rabbit-2", "leo-closing"] {
        answer(&runtime, menu, fit(menu));
    }
    assert!(
        active_stages(&runtime)
            .iter()
            .any(|id| id.starts_with("stage-leo-farewell")),
        "the session ends with a farewell"
    );
    let state = runtime.export_state().expect("state");
    assert!(!state.completed_stage_ids.contains("stage-leo-resting"));
    assert!(!state.completed_stage_ids.contains("stage-leo-light"));
}

#[test]
fn the_book_about_a_long_sleep_makes_leo_shut_it() {
    let (runtime, _) = episode_two();
    start_session(&runtime, true, true, Some("clouds book"));
    for menu in [
        "leo-clinic",
        "leo-upright",
        "leo-snack-have",
        "leo-signs",
        "leo-stories",
    ] {
        answer(&runtime, menu, fit(menu));
    }
    let text = answer(&runtime, "leo-clouds-1", fit("leo-clouds-1"));
    assert!(text.contains("face down on the rug"), "{text}");
    assert_eq!(active_menu(&runtime).as_deref(), Some("leo-closing"));

    let (again, _) = episode_two();
    start_session(&again, true, true, Some("clouds book"));
    for menu in ["leo-clinic", "leo-upright", "leo-snack-have", "leo-signs"] {
        answer(&again, menu, fit(menu));
    }
    let text = answer(&again, "leo-stories", fit("leo-stories"));
    assert!(
        text.contains("Leo (to Isla): That book is wrong. Sleeping isn't the same thing."),
        "{text}"
    );
    assert_eq!(active_menu(&again).as_deref(), Some("leo-clouds-1"));
}

// -------------------------------------------------------------------- the end

#[test]
fn a_quieter_session_gets_a_quieter_message_and_note() {
    let (runtime, _) = episode_two();
    start_session(&runtime, false, false, Some("rabbit book"));
    // Leo will not start without the patients.
    for step in ["go hall", "go upstairs", "go quiet room"] {
        turn(&runtime, step);
    }
    turn(&runtime, "take basket of stuffed animals");
    for step in ["go landing", "go downstairs", "go playroom"] {
        turn(&runtime, step);
    }
    answer(&runtime, "leo-clinic", miss("leo-clinic"));
    answer(&runtime, "leo-upright", miss("leo-upright"));
    answer(&runtime, "leo-snack-none", miss("leo-snack-none"));
    answer(&runtime, "leo-stories", fit("leo-stories"));
    for menu in ["leo-rabbit-1", "leo-rabbit-2", "leo-closing"] {
        answer(&runtime, menu, miss(menu));
    }
    assert!(active_stages(&runtime).contains(&"stage-leo-farewell-low".to_string()));
    let after = finish_after(&runtime);
    assert!(after.contains("resting my eyes before dinner"), "{after}");
    assert!(after.contains("let him set the rules"), "{after}");
    let card = runtime
        .consume_pending_projector_sequence()
        .expect("movie lookup")
        .expect("a card closes this episode too");
    assert!(
        card.frames[0].text.contains("let him set the rules"),
        "the quiet card shows the quiet note: {}",
        card.frames[0].text
    );
    assert_eq!(
        runtime.export_state().expect("state").phase,
        GamePhase::GameEnded
    );
}

#[test]
fn the_warning_signs_alone_give_the_middle_ending() {
    let (runtime, _) = episode_two();
    start_session(&runtime, true, true, Some("owl book"));
    for menu in [
        "leo-clinic",
        "leo-upright",
        "leo-snack-have",
        "leo-signs",
        "leo-stories",
        "leo-owl-1",
        "leo-owl-2",
    ] {
        answer(&runtime, menu, fit(menu));
    }
    answer(&runtime, "leo-resting", miss("leo-resting"));
    answer(&runtime, "leo-owl-3", miss("leo-owl-3"));
    answer(&runtime, "leo-closing", fit("leo-closing"));
    assert!(
        active_stages(&runtime).contains(&"stage-leo-farewell-mid".to_string()),
        "{:?}",
        active_stages(&runtime)
    );
    let after = finish_after(&runtime);
    assert!(after.contains("regular tired or doctor tired"), "{after}");
    assert!(
        after.contains("three white hairs above his mom's ear"),
        "{after}"
    );
}

#[test]
fn the_farewell_shows_what_leo_left_behind_for_each_ending() {
    let pack = load_named_pack("isla", Some("en")).expect("isla loads");
    for (tier, words) in [
        ("low", "24 BEATS across the top in red crayon"),
        ("mid", "REGULAR TIRED"),
        ("high", "laid the one-eyed frog flat on its back"),
    ] {
        let stage = pack
            .beats
            .stages
            .iter()
            .find(|stage| stage.id == format!("stage-leo-farewell-{tier}"))
            .expect("farewell stage");
        assert!(
            stage
                .narrative_lines
                .iter()
                .any(|line| line.contains(words)),
            "{tier}: {:?}",
            stage.narrative_lines
        );
        assert!(
            stage
                .narrative_lines
                .iter()
                .any(|line| line.starts_with("Claire (to Leo):")),
            "Claire collects him at the door"
        );
    }
}

#[test]
fn the_messages_come_from_claire_and_dr_bell_and_say_who_they_are_to() {
    let (runtime, _) = episode_two();
    start_session(&runtime, true, true, Some("owl book"));
    play_owl_to_the_end(&runtime);
    for step in ["go hall", "go kitchen", "go back yard"] {
        turn(&runtime, step);
    }
    let first = answer(&runtime, "phone-1", fit("phone-1"));
    assert!(
        first.contains("Claire (to Isla): I closed my eyes"),
        "{first}"
    );
    let second = answer(&runtime, "phone-2", fit("phone-2"));
    assert!(
        second.contains("Dr. Bell (to Isla): How did it go?"),
        "{second}"
    );
}

#[test]
fn leo_leaves_the_house_after_the_farewell() {
    let (runtime, _) = episode_two();
    start_session(&runtime, true, true, Some("owl book"));
    play_owl_to_the_end(&runtime);
    let state = runtime.export_state().expect("state");
    assert_eq!(state.actor_room_id("leo", "playroom"), "");
}

#[test]
fn the_closing_lines_come_before_the_journal_card_which_shows_the_note() {
    let pack = load_named_pack("isla", Some("en")).expect("isla loads");
    for tier in ["low", "mid", "high"] {
        let end = pack
            .beats
            .stages
            .iter()
            .find(|stage| stage.id == format!("stage-leo-end-{tier}"))
            .expect("end stage");
        assert!(end.end_game, "Leo's is the last episode");
        assert_eq!(end.movie_id, format!("leo-end-{tier}"));
        assert!(end.lines_before_movie);
        let card = pack
            .movies
            .iter()
            .find(|movie| movie.id == end.movie_id)
            .expect("card");
        assert_eq!(card.frames.len(), 1);
        assert_eq!(card.frames[0].heading, "Tuesday, February 4");
        assert_eq!(card.frames[0].text_item, format!("note-leo-{tier}"));
    }
}

#[test]
fn leos_pages_are_in_the_journal_and_none_share_a_name_with_wrens() {
    let pack = load_named_pack("isla", Some("en")).expect("isla loads");
    let pages: Vec<_> = pack
        .items
        .iter()
        .filter(|item| item.group == "journal")
        .map(|item| item.id.as_str())
        .collect();
    for page in [
        "journal-index-two",
        "leo-intake",
        "leo-referral",
        "leo-session-one",
        "leo-schedule",
        "note-leo-low",
        "note-leo-mid",
        "note-leo-high",
        "leo-chart-low",
        "leo-chart-mid",
        "leo-chart-high",
    ] {
        assert!(pages.contains(&page), "{page} is not in the journal");
    }
    // A name that two items answer to would make "look at" ambiguous once both
    // children's pages are in the same journal.
    let mut seen = std::collections::BTreeMap::new();
    for item in pack.items.iter().filter(|item| item.group == "journal") {
        let mut names: Vec<String> = item.aliases.iter().map(|a| a.to_lowercase()).collect();
        names.push(item.label.to_lowercase());
        for name in names {
            if let Some(other) = seen.insert(name.clone(), item.id.clone()) {
                // The three endings of one child deliberately share a name, and
                // the second index replaces the first; only one is ever in the tote.
                let same_set = |a: &str, b: &str| {
                    [
                        "note-three-",
                        "wren-drawing-",
                        "note-leo-",
                        "leo-chart-",
                        "journal-index",
                    ]
                    .iter()
                    .any(|prefix| a.starts_with(prefix) && b.starts_with(prefix))
                };
                assert!(
                    same_set(&other, &item.id) || other == item.id,
                    "'{name}' names both {other} and {}",
                    item.id
                );
            }
        }
    }
}
