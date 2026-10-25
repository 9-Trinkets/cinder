//! Layla's gates read the one condition language. These tests pin the behavior
//! the packs had before the gates were rewritten, so nothing about the story
//! changes: exits stay hidden until their story variable is set, a room reads
//! differently once its boss has fallen, and keys open their gates once.

use cinder_core::content::loader::load_named_pack;
use cinder_core::content::types::Condition;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::WorldState;
use cinder_core::engine::turn_policies::action_is_available;

fn pack() -> cinder_core::content::types::ContentPack {
    load_named_pack("layla", Some("en")).expect("layla loads")
}

#[test]
fn a_gated_exit_is_hidden_until_its_story_variable_is_set() {
    let pack = pack();
    let mut state = WorldState::new(&pack);
    let exit = pack
        .room("oh")
        .expect("oh")
        .exits
        .iter()
        .find(|exit| exit.room_id == "village_square")
        .expect("exit to the village");
    assert_eq!(
        exit.when,
        vec![Condition::StoryVar("elemental_released".to_string())]
    );
    assert!(!pack.exit_is_open(&state, "oh", exit));
    assert!(
        pack.resolve_exit_for(&state, "oh", "village square")
            .is_none(),
        "a hidden exit cannot be walked through"
    );

    state.story_vars.set_unchecked("elemental_released", "true");
    assert!(pack.exit_is_open(&state, "oh", exit));
    assert!(
        pack.resolve_exit_for(&state, "oh", "village square")
            .is_some()
    );

    state
        .story_vars
        .set_unchecked("elemental_released", "false");
    assert!(
        !pack.exit_is_open(&state, "oh", exit),
        "a variable set to false does not open the exit"
    );
}

#[test]
fn an_exit_with_no_conditions_is_always_open() {
    let pack = pack();
    let state = WorldState::new(&pack);
    let open = pack
        .rooms
        .iter()
        .flat_map(|room| room.exits.iter().map(move |exit| (room, exit)))
        .find(|(_, exit)| exit.when.is_empty())
        .expect("an ungated exit exists");
    assert!(pack.exit_is_open(&state, &open.0.id, open.1));
}

#[test]
fn a_room_reads_differently_once_its_boss_is_defeated() {
    let pack = pack();
    let room = pack.room("r5c5").expect("r5c5");
    assert_eq!(
        room.descriptions[0].when,
        vec![Condition::Defeated("goblin-shaman".to_string())]
    );
    let before = look_in(&pack, "r5c5", |_| {});
    let after = look_in(&pack, "r5c5", |state| {
        state
            .combat
            .actor_stats
            .entry("goblin-shaman".to_string())
            .or_default()
            .insert(pack.settings.combat.health_stat_id.clone(), 0);
    });
    assert_ne!(before, after, "the room text changes when the shaman falls");
    assert!(
        after.contains(&room.descriptions[0].inspect_text)
            || after.contains(&room.descriptions[0].summary),
        "the override's text is the one shown: {after}"
    );
}

fn look_in(
    pack: &cinder_core::content::types::ContentPack,
    room: &str,
    change: impl FnOnce(&mut WorldState),
) -> String {
    let mut state = WorldState::new(pack);
    state.current_room_id = room.to_string();
    change(&mut state);
    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime");
    runtime.run_turn("look").expect("look").text()
}

#[test]
fn erasing_needs_a_trace_mark_in_the_room() {
    let pack = pack();
    let erase = pack
        .actions
        .iter()
        .find(|action| action.id == "erase")
        .expect("erase");
    assert_eq!(erase.available.when, vec![Condition::TraceMarkHere]);
    let mut state = WorldState::new(&pack);
    assert!(!action_is_available(
        &pack,
        &state,
        erase,
        &state.current_room_id.clone()
    ));
    let room = state.current_room_id.clone();
    state.add_item_to_storage(
        "drain-sigil",
        cinder_core::content::types::ItemStorageTarget::CurrentRoom,
        &room,
    );
    assert!(action_is_available(&pack, &state, erase, &room));
}

#[test]
fn the_teleport_skill_waits_for_the_knowledge_that_unlocks_it() {
    let pack = pack();
    let teleport = pack
        .actions
        .iter()
        .find(|action| action.id == "teleport")
        .expect("teleport");
    assert_eq!(
        teleport.available.when,
        vec![Condition::StoryVar("knows_teleport".to_string())]
    );
}

fn runtime_at(room: &str, items: &[&str], locale: &str) -> CinderRuntime {
    let pack = load_named_pack("layla", Some(locale)).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = room.to_string();
    for item in items {
        state.add_item(item);
    }
    CinderRuntime::from_state(pack, state, false).expect("runtime")
}

#[test]
fn a_key_opens_its_gate_once_and_is_used_up() {
    let runtime = runtime_at("village_north_gate", &["fortress-gate-key"], "en");
    let first = runtime.run_turn("unlock gate").expect("unlock").text();
    assert!(first.contains("hydraulic deadbolts slide back"), "{first}");
    assert!(first.contains("You use"), "{first}");
    let state = runtime.export_state().expect("state");
    assert_eq!(state.story_vars.get("fortress_gate_open"), Some("true"));
    assert!(!state.has_item("fortress-gate-key"));
    let again = runtime.run_turn("unlock gate").expect("unlock").text();
    assert!(!again.contains("deadbolts"), "{again}");
}

#[test]
fn the_typed_phrases_of_the_old_actions_still_work() {
    for phrase in ["use key", "use the gate key", "unlock the bulkhead"] {
        let runtime = runtime_at("village_north_gate", &["fortress-gate-key"], "en");
        let text = runtime.run_turn(phrase).expect("turn").text();
        assert!(text.contains("deadbolts"), "{phrase}: {text}");
    }
    let runtime = runtime_at("steam_prison_cage", &["iron-cage-key"], "en");
    let text = runtime.run_turn("free zayd").expect("turn").text();
    assert!(text.contains("Zayd scrambles out"), "{text}");
}

#[test]
fn the_same_phrase_picks_the_unlock_in_the_room_you_are_in() {
    let runtime = runtime_at("command_bastion", &["commander-safe-key"], "en");
    let text = runtime.run_turn("use key").expect("turn").text();
    assert!(text.contains("dual tumblers"), "{text}");
}

#[test]
fn an_unlock_without_its_key_does_nothing() {
    let runtime = runtime_at("village_north_gate", &[], "en");
    let text = runtime.run_turn("unlock gate").expect("turn").text();
    assert!(!text.contains("deadbolts"), "{text}");
    assert_ne!(
        runtime
            .export_state()
            .expect("state")
            .story_vars
            .get("fortress_gate_open"),
        Some("true")
    );
}

#[test]
fn an_unlock_in_the_wrong_room_names_the_right_one() {
    let runtime = runtime_at("village_north_gate", &["iron-cage-key"], "en");
    let text = runtime.run_turn("unlock cage").expect("turn").text();
    assert!(text.contains("You must be in"), "{text}");
}

#[test]
fn the_valve_and_the_gate_key_open_the_same_way_once() {
    let runtime = runtime_at("village_west_1", &[], "en");
    let first = runtime.run_turn("turn valve").expect("turn").text();
    assert!(first.contains("valve screeches open"), "{first}");
    let again = runtime.run_turn("turn valve").expect("turn").text();
    assert!(!again.contains("screeches"), "{again}");
}

#[test]
fn the_traditional_chinese_phrases_work() {
    let runtime = runtime_at("steam_prison_cage", &["iron-cage-key"], "zh-TW");
    let text = runtime.run_turn("打開籠子").expect("turn").text();
    assert!(text.contains("宰德"), "{text}");
    let state = runtime.export_state().expect("state");
    assert_eq!(state.story_vars.get("zayd_rescued"), Some("true"));
}
