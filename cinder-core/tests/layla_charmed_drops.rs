//! Tests that charmed mobs have their important drop items in inventory and
//! that the player can take them via party commands.

use cinder_core::content::loader::load_named_pack;
use cinder_core::content::types::ItemStorageTarget;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::{ActorStance, WorldState};

#[test]
fn charmed_guard_has_gate_key_in_inventory_and_can_be_taken() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);

    // Initial state: guard has fortress-gate-key in initial_inventory
    assert!(
        state.actor_has_item("garrison_guard", "fortress-gate-key"),
        "guard must start with fortress-gate-key in inventory"
    );

    // Move player to village_north_gate where garrison_guard is stationed
    state.current_room_id = "village_north_gate".to_string();

    // Charm the guard (convert to ally)
    state.set_actor_stance(&pack, "garrison_guard", ActorStance::Allied, true);
    assert_eq!(state.stance("garrison_guard"), ActorStance::Allied);
    assert!(state.follows_player("garrison_guard"));

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");

    // Layla takes the fortress-gate-key from the charmed guard
    let take_outcome = runtime
        .run_turn("take fortress-gate-key from guard")
        .expect("take key from guard succeeds");
    assert!(
        !take_outcome.lines.is_empty(),
        "taking key produces narration lines"
    );

    let state_after_take = runtime.export_state().expect("state exported");
    assert!(
        state_after_take.has_item("fortress-gate-key"),
        "Layla must now have the fortress-gate-key"
    );
    assert!(
        !state_after_take.actor_has_item("garrison_guard", "fortress-gate-key"),
        "Guard must no longer have the fortress-gate-key"
    );

    // Layla uses the key to unlock the gate
    runtime
        .run_turn("unlock gate")
        .expect("unlock gate succeeds");

    let final_state = runtime.export_state().expect("state exported");
    assert_eq!(
        final_state.story_vars.get("fortress_gate_open"),
        Some("true"),
        "Gate must be unlocked"
    );
}

#[test]
fn charmed_guard_does_not_drop_duplicate_key_on_subsequent_defeat() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "village_north_gate".to_string();

    // Charm the guard and take key
    state.set_actor_stance(&pack, "garrison_guard", ActorStance::Allied, true);
    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");
    runtime
        .run_turn("take fortress-gate-key from guard")
        .expect("take key succeeds");

    // Reduce guard HP to 1, make attackable, and turn hostile to simulate combat defeat
    let mut s = runtime.export_state().expect("state exported");
    s.actor_stats
        .entry("garrison_guard".to_string())
        .or_default()
        .insert("hp".to_string(), 1);
    s.actor_stats
        .entry("player".to_string())
        .or_default()
        .insert("strength".to_string(), 50);
    s.set_actor_stance(&pack, "garrison_guard", ActorStance::Hostile, false);

    let runtime2 = CinderRuntime::from_state(pack.clone(), s, false).expect("runtime creates");
    runtime2
        .run_turn("attack garrison guard")
        .expect("attack guard succeeds");

    let final_state = runtime2.export_state().expect("state exported");
    assert!(
        final_state.actor_is_defeated("garrison_guard", "hp"),
        "guard must be defeated"
    );
    // Key must NOT have dropped into room because it was already taken
    assert!(
        !final_state.has_item_in_storage(
            "fortress-gate-key",
            ItemStorageTarget::CurrentRoom,
            "village_north_gate"
        ),
        "Defeated guard must not drop duplicate key after it was taken"
    );
}

#[test]
fn unallied_guard_drops_key_normally_when_defeated() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "village_north_gate".to_string();
    state
        .actor_stats
        .entry("garrison_guard".to_string())
        .or_default()
        .insert("hp".to_string(), 1);
    state
        .actor_stats
        .entry("player".to_string())
        .or_default()
        .insert("strength".to_string(), 50);

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");
    runtime
        .run_turn("attack garrison guard")
        .expect("attack guard succeeds");

    let final_state = runtime.export_state().expect("state exported");
    assert!(
        final_state.actor_is_defeated("garrison_guard", "hp"),
        "guard must be defeated"
    );
    // Key must drop into room when guard is killed without being charmed
    assert!(
        final_state.has_item_in_storage(
            "fortress-gate-key",
            ItemStorageTarget::CurrentRoom,
            "village_north_gate"
        ),
        "Unallied guard must drop fortress-gate-key on defeat"
    );
}
