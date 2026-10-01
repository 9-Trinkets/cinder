use cinder_core::content::loader::load_named_pack;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::WorldState;

#[test]
fn test_scroll_reading_and_usage_via_generic_item_system() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");

    // 1. Drain scroll via `read scroll`
    let mut state = WorldState::new(&pack);
    state.acquire_player_item(&pack, "drain-scroll");
    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime");

    let outcome = runtime.run_turn("read scroll").expect("read scroll");
    let text = outcome.text();
    assert!(
        text.contains("You know the drain sigil") || text.contains("drain sigil"),
        "Should narrate learning drain sigil: {text}"
    );
    assert!(
        !text.contains("feel mended"),
        "Should NOT narrate potion healing flavor text: {text}"
    );

    let state = runtime.export_state().unwrap();
    assert_eq!(state.story_vars.get("knows_drain"), Some("true"));
    assert!(
        !state.has_item("drain-scroll"),
        "drain-scroll must be consumed"
    );

    // 2. Spawn scroll via `use ember scroll`
    let mut state = WorldState::new(&pack);
    state.acquire_player_item(&pack, "spawn-scroll");
    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime");

    let outcome = runtime
        .run_turn("use ember scroll")
        .expect("use ember scroll");
    let text = outcome.text();
    assert!(
        text.contains("You know the shape") || text.contains("spawn"),
        "Should narrate learning spawn sigil: {text}"
    );
    assert!(
        !text.contains("feel mended"),
        "Should NOT narrate potion healing flavor text: {text}"
    );

    let state = runtime.export_state().unwrap();
    assert_eq!(state.story_vars.get("knows_spawn"), Some("true"));
    assert!(
        !state.has_item("spawn-scroll"),
        "spawn-scroll must be consumed"
    );
}

#[test]
fn test_key_usage_and_symmetric_phrasing() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");

    // 1. In village_north_gate, unlock with fortress-gate-key using `use key on gate`
    let mut state = WorldState::new(&pack);
    state.current_room_id = "village_north_gate".to_string();
    state.acquire_player_item(&pack, "fortress-gate-key");
    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime");

    let outcome = runtime
        .run_turn("use key on gate")
        .expect("use key on gate");
    let text = outcome.text();
    assert!(
        text.contains("hydraulic deadbolts slide back")
            || text.contains("releasing the sealed fortress gate"),
        "Should unlock gate: {text}"
    );
    let state = runtime.export_state().unwrap();
    assert_eq!(state.story_vars.get("fortress_gate_open"), Some("true"));
    assert!(!state.has_item("fortress-gate-key"));

    // 2. In steam_prison_cage, unlock with `unlock cage with key`
    let mut state = WorldState::new(&pack);
    state.current_room_id = "steam_prison_cage".to_string();
    state.acquire_player_item(&pack, "iron-cage-key");
    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime");

    let outcome = runtime
        .run_turn("unlock cage with key")
        .expect("unlock cage with key");
    let text = outcome.text();
    assert!(
        text.contains("barred gate swings open") || text.contains("slide the heavy iron key"),
        "Should unlock cage: {text}"
    );
    let state = runtime.export_state().unwrap();
    assert_eq!(state.story_vars.get("zayd_rescued"), Some("true"));
    assert!(!state.has_item("iron-cage-key"));

    // 3. In command_bastion, unlock with `use commander safe key`
    let mut state = WorldState::new(&pack);
    state.current_room_id = "command_bastion".to_string();
    state.acquire_player_item(&pack, "commander-safe-key");
    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime");

    let outcome = runtime
        .run_turn("use commander safe key")
        .expect("use commander safe key");
    let text = outcome.text();
    assert!(
        text.contains("Teleportation Scroll") || text.contains("steel compartment"),
        "Should unlock safe: {text}"
    );
    let state = runtime.export_state().unwrap();
    assert_eq!(state.story_vars.get("safe_unlocked"), Some("true"));
    assert!(!state.has_item("commander-safe-key"));
    assert!(state.has_item("teleport-scroll"));

    // 4. In courtyard_center, unlock with `use courtyard-cage-key`
    let mut state = WorldState::new(&pack);
    state.current_room_id = "courtyard_center".to_string();
    state.acquire_player_item(&pack, "courtyard-cage-key");
    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime");

    let outcome = runtime
        .run_turn("use courtyard-cage-key")
        .expect("use courtyard-cage-key");
    let text = outcome.text();
    assert!(
        text.contains("Commander Astrid") || text.contains("Einar"),
        "Should unlock courtyard cages: {text}"
    );
    let state = runtime.export_state().unwrap();
    assert_eq!(state.story_vars.get("courtyard_cages_opened"), Some("true"));
    assert!(!state.has_item("courtyard-cage-key"));

    // 5. Using key in the wrong room gives clear contextual rejection
    let mut state = WorldState::new(&pack);
    state.current_room_id = "citadel_corridor_south".to_string();
    state.acquire_player_item(&pack, "iron-cage-key");
    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime");

    let outcome = runtime.run_turn("use iron cage key").expect("turn runs");
    let text = outcome.text();
    assert!(
        text.contains("Suspended Steam Cage") || text.contains("can't run"),
        "Should reject when not in allowed room: {text}"
    );
    let state = runtime.export_state().unwrap();
    assert!(
        state.has_item("iron-cage-key"),
        "Key must not be consumed on rejection"
    );

    // 6. Using a standalone key with no lock via generic plan_use_command
    let mut pack_with_key = pack.clone();
    pack_with_key
        .items
        .push(cinder_core::content::types::ItemDefinition {
            id: "rusty-key".to_string(),
            label: "rusty key".to_string(),
            kind: cinder_core::content::types::ItemKind::Key,
            ..Default::default()
        });
    let mut state = WorldState::new(&pack_with_key);
    state.acquire_player_item(&pack_with_key, "rusty-key");
    let runtime = CinderRuntime::from_state(pack_with_key, state, false).expect("runtime");

    let outcome = runtime.run_turn("use rusty key").expect("turn runs");
    let text = outcome.text();
    assert!(
        text.contains("There is nothing to unlock here with the rusty key"),
        "Should reject with contextual key message: {text}"
    );
    let state = runtime.export_state().unwrap();
    assert!(
        state.has_item("rusty-key"),
        "Key must not be consumed on rejection"
    );
}
