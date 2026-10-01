//! Integration tests for the Handler's reward beats.
//!
//! Each beat is a one-shot interjection the Handler grudgingly attaches to a
//! thing Layla did well, framed as a console upgrade he hands over rather than
//! praise. Kills read as competence, not virtue.

use cinder_core::content::loader::load_named_pack;
use cinder_core::engine::narrative::{NarrativeLine, NarrativeLineKind};
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::{ActorStance, WorldState};

fn handler_spoke(lines: &[NarrativeLine], needle: &str) -> bool {
    lines
        .iter()
        .any(|line| line.kind == NarrativeLineKind::Channel && line.text.contains(needle))
}

fn strong_player(state: &mut WorldState) {
    let stats = state.actor_stats.entry("player".to_string()).or_default();
    stats.insert("strength".to_string(), 100);
    // `surround_rule` is intelligence-based resistance.
    stats.insert("intelligence".to_string(), 100);
}

#[test]
fn first_mob_kill_grants_vitals_readout_and_handler_line() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "r2c2".to_string(); // goblin-1
    strong_player(&mut state);

    // Vitals stay hidden until Layla proves she can handle herself.
    assert!(
        !pack.vitals_sidebar_shown(&state),
        "vitals must stay hidden until the first kill"
    );

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");
    let outcome = runtime
        .run_turn("attack goblin")
        .expect("attack goblin succeeds");

    let after = runtime.export_state().expect("state exported");
    assert_eq!(after.story_vars.get("first_mob_defeated"), Some("true"));
    assert!(
        pack.vitals_sidebar_shown(&after),
        "the first kill must reveal the vitals readout"
    );
    assert!(
        handler_spoke(&outcome.lines, "First one down"),
        "Handler must comment on the first kill"
    );
}

#[test]
fn first_mob_kill_beat_is_one_shot() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "r2c2".to_string();
    strong_player(&mut state);

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");
    let first = runtime
        .run_turn("attack goblin")
        .expect("attack goblin succeeds");
    assert!(handler_spoke(&first.lines, "First one down"));

    // Second kill, same room, must not repeat the beat.
    let second = runtime
        .run_turn("attack goblin")
        .expect("second attack succeeds");
    assert!(
        !handler_spoke(&second.lines, "First one down"),
        "the first-kill beat must not repeat"
    );
}

#[test]
fn clean_shaman_run_earns_handler_acknowledgement() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "r5c5".to_string();
    strong_player(&mut state);

    // No floor-one mob was attacked, so the clean-run condition holds.
    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");
    let outcome = runtime
        .run_turn("attack goblin shaman")
        .expect("attack shaman succeeds");

    let after = runtime.export_state().expect("state exported");
    assert!(after.has_item("shaman-ring"), "clean run grants the ring");
    assert!(
        handler_spoke(&outcome.lines, "Clean"),
        "Handler must acknowledge the clean shaman run"
    );
}

#[test]
fn messy_shaman_run_earns_no_acknowledgement() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "r5c5".to_string();
    strong_player(&mut state);
    state
        .story_vars
        .set_unchecked("layla_attacked_floor_one_mob", "true");

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");
    let outcome = runtime
        .run_turn("attack goblin shaman")
        .expect("attack shaman succeeds");

    let after = runtime.export_state().expect("state exported");
    assert!(!after.has_item("shaman-ring"));
    assert!(
        !handler_spoke(&outcome.lines, "Clean"),
        "a messy run must not earn the Handler's approval"
    );
}

/// Ring the room containing `target` so the final sigil placement converts it.
/// Every neighbour of `room_id` needs a charm sigil; placing the trigger item
/// in `room_id` itself is what closes the circle and fires the hook.
fn ring_room(
    pack: &cinder_core::content::types::ContentPack,
    state: &mut WorldState,
    room_id: &str,
) -> Vec<NarrativeLine> {
    use cinder_core::content::types::ItemStorageTarget as Storage;
    use cinder_core::engine::events::{TimestampedWorldEvent, WorldEvent};
    use cinder_core::engine::reducer::apply_events;

    let player_id = pack.settings.combat.player_actor_id.clone();
    state.actor_level.insert(player_id, 12);

    for neighbour in pack.adjacent_room_ids(room_id) {
        state.add_item_to_storage("charm-sigil", Storage::CurrentRoom, &neighbour);
    }

    // Placing the final sigil in the target's own room closes the circle.
    let output = apply_events(
        state,
        pack,
        &[TimestampedWorldEvent::now(WorldEvent::ItemAcquired {
            item_id: "charm-sigil".to_string(),
            storage: Storage::CurrentRoom,
        })],
    );
    let _ = room_id;
    output.lines.0
}

#[test]
fn first_mob_charm_earns_handler_line() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "r2c2".to_string(); // goblin-1
    strong_player(&mut state);

    let lines = ring_room(&pack, &mut state, "r2c2");

    assert_eq!(
        state.relationship("goblin-1").stance,
        ActorStance::Allied,
        "the ring must convert goblin-1"
    );
    assert_eq!(state.story_vars.get("first_mob_charmed"), Some("true"));
    assert!(
        handler_spoke(&lines, "closed a ring around it"),
        "Handler must comment on the first charm"
    );
}

#[test]
fn charm_beat_ignores_the_shaman() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "r5c5".to_string();
    strong_player(&mut state);

    let lines = ring_room(&pack, &mut state, "r5c5");

    assert_eq!(
        state.story_vars.get("first_mob_charmed"),
        None,
        "the shaman is a boss and must not spend the first-charm beat"
    );
    assert!(
        !handler_spoke(&lines, "closed a ring around it"),
        "the shaman must not spend the first-charm beat"
    );
}

#[test]
fn sensory_enhancer_narrates_handler_upgrade() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "r5c5".to_string();
    state
        .player_inventory
        .insert("sensory-enhancer".to_string(), 1);

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");
    let outcome = runtime
        .run_turn("use sensory-enhancer")
        .expect("using the enhancer succeeds");

    let after = runtime.export_state().expect("state exported");
    assert_eq!(after.story_vars.get("has_sensory_enhancer"), Some("true"));
    assert!(
        handler_spoke(&outcome.lines, "perception capsule"),
        "Handler must frame the enhancer as a console upgrade"
    );
}

#[test]
fn handler_reward_lines_are_defined() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    for key in [
        "handler.first_mob_killed",
        "handler.first_mob_charmed",
        "handler.clean_shaman_reward",
        "handler.sensory_enhancer_unlocked",
    ] {
        assert!(pack.messages.contains_key(key), "{key} must exist");
    }
}
