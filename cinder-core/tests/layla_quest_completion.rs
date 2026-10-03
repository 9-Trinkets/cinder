//! Integration tests for the Handler's quest-completion commentary.
//!
//! A beat's `completion_message` fires against the stage that was *finished*,
//! which is what makes it the quest's closing beat rather than any step along
//! the way. Two properties follow and are pinned here:
//!
//! 1. Quest-terminal stages comment, including terminal ones with no
//!    `next_stage_ids` — the case `on_advance_effects` structurally cannot
//!    cover, because those effects are read off the *next* stage.
//! 2. Intermediate steps of a quest stay silent, so the Handler comments once
//!    per quest rather than once per beat.

use cinder_core::content::loader::load_named_pack;
use cinder_core::content::types::{ContentPack, ItemStorageTarget, PackMessageVoice};
use cinder_core::engine::events::{TimestampedWorldEvent, WorldEvent};
use cinder_core::engine::narrative::{NarrativeLine, NarrativeLineKind};
use cinder_core::engine::reducer::apply_events;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::WorldState;

/// The four quest-closing Handler lines, by the phrase that identifies each.
const COMPLETION_MARKS: &[&str] = &[
    "read better quiet",
    "inside your reach",
    "Signed off on the retrieval",
    "Cage count reads zero",
];

fn handler_lines(lines: &[NarrativeLine]) -> Vec<&NarrativeLine> {
    lines
        .iter()
        .filter(|line| line.kind == NarrativeLineKind::Channel)
        .collect()
}

fn spoken_any_completion(lines: &[NarrativeLine]) -> bool {
    handler_lines(lines)
        .iter()
        .any(|line| COMPLETION_MARKS.iter().any(|m| line.text.contains(m)))
}

/// Run one world event through the reducer and hand back its narrative lines.
/// Quest beats advance off these signals, not off wall-clock time.
fn fire(state: &mut WorldState, pack: &ContentPack, event: WorldEvent) -> Vec<NarrativeLine> {
    let output = apply_events(state, pack, &[TimestampedWorldEvent::now(event)]);
    output.lines.0
}

#[test]
fn awakening_jamil_draws_a_handler_comment() {
    // `sq_awaken_father` is quest-terminal *and* has no `next_stage_ids`, so
    // its comment can only come from `completion_message`.
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "village_square".to_string();
    state.active_objective_stage_ids = vec!["sq_awaken_father".to_string()];
    state.add_item("zayd-lantern");

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");
    let outcome = runtime
        .run_turn("give battered miner's lantern to sakhra")
        .expect("give lantern to sakhra");

    let after = runtime.export_state().expect("state exported");
    assert!(after.is_actor_awakened("sakhra"), "the quest must complete");
    assert!(
        handler_lines(&outcome.lines)
            .iter()
            .any(|line| line.text.contains("read better quiet")),
        "Handler must comment on awakening Jamil: {:#?}",
        outcome.lines
    );
}

#[test]
fn returning_zayd_draws_a_handler_comment() {
    // `sq_return_zayd` closes the `save_zayd` quest even though it hands off to
    // a different quest afterwards, so the comment has to follow the quest_id
    // boundary rather than the chain's tail.
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "village_square".to_string();
    state.active_objective_stage_ids = vec!["sq_return_zayd".to_string()];
    state.story_vars.set_unchecked("zayd_rescued", "true");

    // Escorting Zayd home fires the `zayd_safe` story-var hook on arrival,
    // which is the signal `sq_return_zayd` advances on.
    let lines = fire(
        &mut state,
        &pack,
        WorldEvent::PlayerMoved {
            from_room_id: "village_square".to_string(),
            to_room_id: "village_south_1".to_string(),
        },
    );

    assert!(
        state.completed_stage_ids.contains("sq_return_zayd"),
        "the quest must complete"
    );
    assert!(
        handler_lines(&lines)
            .iter()
            .any(|line| line.text.contains("inside your reach")),
        "Handler must comment on returning Zayd: {lines:#?}"
    );
}

#[test]
fn an_intermediate_step_of_a_quest_stays_silent() {
    // `mq_find_scroll` is the first of two `teleport_scroll` stages. Completing
    // it is progress, not a solved quest, so the Handler must not weigh in.
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "village_square".to_string();
    state.active_objective_stage_ids = vec!["mq_find_scroll".to_string()];

    let lines = fire(
        &mut state,
        &pack,
        WorldEvent::ItemAcquired {
            item_id: "teleport-scroll".to_string(),
            storage: ItemStorageTarget::PlayerInventory,
        },
    );

    assert!(
        state.completed_stage_ids.contains("mq_find_scroll"),
        "the stage must complete"
    );
    assert!(
        !spoken_any_completion(&lines),
        "an intermediate beat must not draw a quest-completion comment: {lines:#?}"
    );
}

#[test]
fn quest_completion_comment_is_one_shot() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "village_square".to_string();
    state.active_objective_stage_ids = vec!["sq_return_zayd".to_string()];
    state.story_vars.set_unchecked("zayd_rescued", "true");

    let arrival = WorldEvent::PlayerMoved {
        from_room_id: "village_square".to_string(),
        to_room_id: "village_south_1".to_string(),
    };
    let first = fire(&mut state, &pack, arrival.clone());
    assert!(spoken_any_completion(&first), "arrival comments");

    // The beat left the active set, so a later arrival must not repeat it.
    let second = fire(&mut state, &pack, arrival);
    assert!(
        !spoken_any_completion(&second),
        "the comment must not repeat: {second:#?}"
    );
}

#[test]
fn quest_completion_messages_exist_and_are_handler_voiced() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    for key in [
        "handler.quest.teleport_scroll_complete",
        "handler.quest.free_prisoners_complete",
        "handler.quest.save_zayd_complete",
        "handler.quest.awaken_jamil_complete",
    ] {
        let message = pack
            .messages
            .get(key)
            .unwrap_or_else(|| panic!("{key} must exist"));
        assert_eq!(
            message.voice(),
            PackMessageVoice::Handler,
            "{key} must be handler-voiced so it renders as a Handler comm"
        );
    }
}

#[test]
fn only_quest_closing_beats_carry_a_completion_message() {
    // Guards the per-quest (not per-beat) contract in the data itself: a stage
    // may only comment when it is the last stage of its quest, i.e. when its
    // `next_stage_ids` either is empty or leaves the `quest_id`.
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let quest_of = |stage_id: &str| {
        pack.beats
            .stages
            .iter()
            .find(|s| s.id == stage_id)
            .and_then(|s| s.quest_id.clone())
    };

    for stage in &pack.beats.stages {
        let Some(key) = stage.completion_message.as_deref() else {
            continue;
        };
        assert!(
            pack.messages.contains_key(key),
            "{}: completion_message {key} is not defined",
            stage.id
        );
        let Some(quest_id) = stage.quest_id.as_deref() else {
            panic!("{}: a completion comment needs a quest_id", stage.id);
        };
        for next_id in &stage.next_stage_ids {
            let Some(next_quest) = quest_of(next_id) else {
                continue;
            };
            assert_ne!(
                next_quest, quest_id,
                "{}: {} is still the same quest, so it must not comment yet",
                stage.id, next_id
            );
        }
    }
}
