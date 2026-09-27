//! Integration tests for Floor 4 teleportation mechanics:
//! - Teleport scroll unlocks action and triggers Handler tutorial commentary
//! - Temporary chalk anchors (max 3, FIFO eviction)
//! - Single-use consumption of chalk anchors upon teleporting to them
//! - Step-on registration of permanent platform anchors
//! - Sigil tracing on platform activates it and unlocks Floor 5 descent
//! - Infinite fast-travel between permanent anchors

use cinder_core::content::loader::load_named_pack;
use cinder_core::content::types::{ItemStorageTarget, PanelDataSource};
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::WorldState;

#[test]
fn test_read_scroll_learns_teleport_and_handler_explains() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "command_bastion".to_string();
    state.acquire_player_item(&pack, "teleport-scroll");

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");

    let outcome = runtime
        .run_turn("use teleport scroll")
        .expect("read scroll");
    let text = outcome.text();

    assert!(text.contains("learned the Teleportation Sigil"));
    assert!(
        text.contains("links your chalk to anchor points") || text.contains("Good. That sigil")
    );

    let state = runtime.export_state().expect("export state");
    assert_eq!(state.story_vars.get("knows_teleport"), Some("true"));

    let options = runtime
        .panel_options(&PanelDataSource::TeleportAnchors)
        .expect("options");
    // No anchors active yet
    assert!(options.is_empty());
}

#[test]
fn test_chalk_anchors_fifo_eviction_and_mp_cost() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.story_vars.set_unchecked("knows_teleport", "true");
    state.acquire_player_item(&pack, "magic-chalk");
    state
        .initial_actor_stats
        .entry("player".to_string())
        .or_default()
        .insert("mp".to_string(), 20);
    state
        .actor_stats
        .entry("player".to_string())
        .or_default()
        .insert("mp".to_string(), 20);
    state.current_room_id = "village_square".to_string();

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");

    // Anchor 1: village_square
    let mp_before_1 = runtime
        .export_state()
        .unwrap()
        .actor_stat_u32("player", "mp");
    let out1 = runtime.run_turn("trace teleport sigil").expect("trace 1");
    assert!(out1.text().contains("anchoring this room") || out1.text().contains("draw"));
    let s1 = runtime.export_state().unwrap();
    assert_eq!(s1.chalk_anchors, vec!["village_square"]);
    assert_eq!(s1.actor_stat_u32("player", "mp"), mp_before_1 - 4);

    // Anchor 2: village_sw_corner
    runtime.run_turn("go west").expect("go west");
    let mp_before_2 = runtime
        .export_state()
        .unwrap()
        .actor_stat_u32("player", "mp");
    runtime.run_turn("trace teleport sigil").expect("trace 2");
    let s2 = runtime.export_state().unwrap();
    assert_eq!(
        s2.chalk_anchors,
        vec!["village_square", "village_sw_corner"]
    );
    assert_eq!(s2.actor_stat_u32("player", "mp"), mp_before_2 - 4);

    // Anchor 3: village_west_1
    runtime.run_turn("go northeast").expect("go ne");
    let mp_before_3 = runtime
        .export_state()
        .unwrap()
        .actor_stat_u32("player", "mp");
    runtime.run_turn("trace teleport sigil").expect("trace 3");
    let s3 = runtime.export_state().unwrap();
    assert_eq!(
        s3.chalk_anchors,
        vec!["village_square", "village_sw_corner", "village_west_1"]
    );
    assert_eq!(s3.actor_stat_u32("player", "mp"), mp_before_3 - 4);

    // Anchor 4: village_west_2 (exceeds max 3, evicts oldest: village_square)
    runtime.run_turn("go northeast").expect("go ne");
    let mp_before_4 = runtime
        .export_state()
        .unwrap()
        .actor_stat_u32("player", "mp");
    let out4 = runtime.run_turn("trace teleport sigil").expect("trace 4");
    assert!(out4.text().contains("fades away as you draw a new one"));
    assert!(out4.text().contains("Village Square"));

    let s4 = runtime.export_state().unwrap();
    assert_eq!(
        s4.chalk_anchors,
        vec!["village_sw_corner", "village_west_1", "village_west_2"]
    );
    assert_eq!(s4.actor_stat_u32("player", "mp"), mp_before_4 - 4);
    assert!(!s4.has_chalk_anchor("village_square"));
    assert!(!s4.has_item_in_storage(
        "teleport-sigil",
        ItemStorageTarget::CurrentRoom,
        "village_square"
    ));
}

#[test]
fn test_chalk_anchor_single_use_consumption() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.story_vars.set_unchecked("knows_teleport", "true");
    state.acquire_player_item(&pack, "magic-chalk");
    state
        .initial_actor_stats
        .entry("player".to_string())
        .or_default()
        .insert("mp".to_string(), 20);
    state
        .actor_stats
        .entry("player".to_string())
        .or_default()
        .insert("mp".to_string(), 20);
    state.current_room_id = "village_square".to_string();

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");

    // Place chalk anchor in village_square
    runtime
        .run_turn("trace teleport sigil")
        .expect("trace sigil");
    assert!(
        runtime
            .export_state()
            .unwrap()
            .has_chalk_anchor("village_square")
    );

    // Walk to village_sw_corner
    runtime.run_turn("go west").expect("go west");
    assert_eq!(runtime.current_room_id().unwrap(), "village_sw_corner");

    // Teleport back to village_square
    let tp_out = runtime
        .run_turn("teleport village_square")
        .expect("teleport");
    assert!(tp_out.text().contains("Bright light flares from the chalk"));
    assert_eq!(runtime.current_room_id().unwrap(), "village_square");

    // Chalk anchor is consumed
    let state = runtime.export_state().unwrap();
    assert!(!state.has_chalk_anchor("village_square"));
    assert!(!state.has_item_in_storage(
        "teleport-sigil",
        ItemStorageTarget::CurrentRoom,
        "village_square"
    ));
}

#[test]
fn test_platform_discovery_activation_and_floor5_gate() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.story_vars.set_unchecked("knows_teleport", "true");
    state.acquire_player_item(&pack, "magic-chalk");
    state
        .actor_stats
        .entry("player".to_string())
        .or_default()
        .insert("mp".to_string(), 20);
    // Access to teleport_platform is exclusively from command_bastion, gated by malik_defeated
    state.current_room_id = "command_bastion".to_string();

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");

    // Before defeating Malik, entering the platform is blocked
    let barred = runtime
        .run_turn("go northwest")
        .expect("try enter platform");
    assert_ne!(runtime.current_room_id().unwrap(), "teleport_platform");

    // Defeating Malik unlocks the exit to the platform
    let mut state = runtime.export_state().unwrap();
    state.story_vars.set_unchecked("malik_defeated", "true");
    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");

    // Step onto teleport_platform -> registers permanent anchors
    runtime.run_turn("go northwest").expect("enter platform");
    assert_eq!(runtime.current_room_id().unwrap(), "teleport_platform");

    let s1 = runtime.export_state().unwrap();
    assert_eq!(s1.story_vars.get("anchor_floor4_platform"), Some("true"));
    assert_eq!(s1.story_vars.get("anchor_floor5_gate"), Some("true"));
    assert_ne!(s1.story_vars.get("platform_activated"), Some("true"));

    // Before activation, descending to floor 5 is blocked
    let blocked = runtime.run_turn("go floor 5").expect("try descend");
    assert_ne!(runtime.current_room_id().unwrap(), "floor5_start");
    assert!(
        blocked.text().contains("route sheet")
            || blocked.text().contains("cannot go")
            || blocked.text().contains("can't go")
    );

    // Trace teleport sigil on the platform -> activates it
    let activate_out = runtime
        .run_turn("trace teleport sigil")
        .expect("trace platform");
    assert!(
        activate_out.text().contains("platform roars to life")
            || activate_out.text().contains("conduits")
    );

    let s2 = runtime.export_state().unwrap();
    assert_eq!(s2.story_vars.get("platform_activated"), Some("true"));
    // Sigil did not become a temporary chalk anchor or loose item
    assert!(!s2.has_chalk_anchor("teleport_platform"));
    assert!(!s2.has_item_in_storage(
        "teleport-sigil",
        ItemStorageTarget::CurrentRoom,
        "teleport_platform"
    ));

    // Descend to Floor 5 now succeeds
    runtime.run_turn("go floor 5").expect("descend floor 5");
    assert_eq!(runtime.current_room_id().unwrap(), "floor5_start");
}

#[test]
fn test_permanent_anchor_fast_travel_infinite_uses() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.story_vars.set_unchecked("knows_teleport", "true");
    state
        .story_vars
        .set_unchecked("anchor_floor4_platform", "true");
    state.story_vars.set_unchecked("anchor_floor5_gate", "true");
    state.current_room_id = "village_square".to_string();

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");

    // Teleport to Floor 4 platform
    let out1 = runtime
        .run_turn("teleport teleport_platform")
        .expect("teleport 1");
    assert!(out1.text().contains("brass platform glows with blue light"));
    assert_eq!(runtime.current_room_id().unwrap(), "teleport_platform");

    // Permanent anchor is NOT consumed
    let s1 = runtime.export_state().unwrap();
    assert_eq!(s1.story_vars.get("anchor_floor4_platform"), Some("true"));

    // Teleport to Floor 5 platform
    let _out2 = runtime
        .run_turn("teleport floor5_start")
        .expect("teleport 2");
    assert_eq!(runtime.current_room_id().unwrap(), "floor5_start");

    // Teleport back to Floor 4 platform
    runtime
        .run_turn("teleport teleport_platform")
        .expect("teleport 3");
    assert_eq!(runtime.current_room_id().unwrap(), "teleport_platform");
}

#[test]
fn test_teleport_rejections() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.story_vars.set_unchecked("knows_teleport", "true");
    state
        .story_vars
        .set_unchecked("anchor_floor4_platform", "true");
    state.current_room_id = "teleport_platform".to_string();

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");

    // Already standing at anchor
    let rej1 = runtime
        .run_turn("teleport teleport_platform")
        .expect("teleport self");
    assert!(rej1.text().contains("already standing at this anchor"));

    // Unknown anchor
    let rej2 = runtime
        .run_turn("teleport nonexistent_dungeon")
        .expect("teleport unknown");
    assert!(rej2.text().contains("Unknown anchor"));
}
