//! Layla's gates read the one condition language. These tests pin the behavior
//! the packs had before the gates were rewritten, so nothing about the story
//! changes: exits stay hidden until their story variable is set, a room reads
//! differently once its boss has fallen, and a one-time unlock cannot be repeated.

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
fn a_one_time_unlock_cannot_be_repeated() {
    let pack = pack();
    let action = pack
        .actions
        .iter()
        .find(|action| action.id == "unlock_fortress_gate")
        .expect("the gate unlock");
    assert_eq!(
        action.available.when,
        vec![Condition::Not(Box::new(Condition::StoryVar(
            "fortress_gate_open".to_string()
        )))]
    );
    let mut state = WorldState::new(&pack);
    state.current_room_id = "village_north_gate".to_string();
    state.add_item("fortress-gate-key");
    assert!(action_is_available(
        &pack,
        &state,
        action,
        "village_north_gate"
    ));

    state.story_vars.set_unchecked("fortress_gate_open", "true");
    assert!(
        !action_is_available(&pack, &state, action, "village_north_gate"),
        "the gate is already open"
    );
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
