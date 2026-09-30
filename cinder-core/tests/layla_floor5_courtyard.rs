//! Integration tests for Floor 5 (The Frost Citadel: 7-Room Hexagonal Courtyard).

use cinder_core::content::loader::load_named_pack;
use cinder_core::content::types::ItemStorageTarget;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::{ActorStance, WorldState};

const EXPECTED_COURTYARD_ROOMS: &[&str] = &[
    "courtyard_center",
    "courtyard_north",
    "courtyard_northeast",
    "courtyard_southeast",
    "courtyard_south",
    "courtyard_southwest",
    "courtyard_northwest",
];

#[test]
fn floor5_courtyard_rooms_load_and_validate() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");

    assert_eq!(EXPECTED_COURTYARD_ROOMS.len(), 7);
    for room_id in EXPECTED_COURTYARD_ROOMS {
        let room = pack
            .room(room_id)
            .unwrap_or_else(|| panic!("missing room {room_id}"));
        assert!(!room.title.trim().is_empty(), "room {room_id} title empty");
        assert!(
            !room.summary.trim().is_empty(),
            "room {room_id} summary empty"
        );
        assert!(
            !room.inspect_text.trim().is_empty(),
            "room {room_id} inspect_text empty"
        );
        assert!(
            !room.features.is_empty(),
            "room {room_id} should have features"
        );
        for feature in &room.features {
            assert!(
                !feature.inspect_text.trim().is_empty(),
                "feature {} in room {room_id} has empty inspect text",
                feature.id
            );
        }
    }
}

#[test]
fn floor5_courtyard_hexagonal_navigation_resolves() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");

    // Center has no physical connection to Floor 4; connects only to the 6 perimeter points
    let center = pack.room("courtyard_center").expect("center exists");
    assert!(
        !center
            .exits
            .iter()
            .any(|e| e.room_id == "teleport_platform"),
        "floor 4 and floor 5 must not be physically connected by an exit"
    );
    for perimeter_id in &EXPECTED_COURTYARD_ROOMS[1..] {
        assert!(
            center.exits.iter().any(|e| e.room_id == *perimeter_id),
            "center connects directly to perimeter room {perimeter_id}"
        );
    }

    // Each perimeter point connects back to center
    for perimeter_id in &EXPECTED_COURTYARD_ROOMS[1..] {
        let room = pack.room(perimeter_id).unwrap();
        assert!(
            room.exits.iter().any(|e| e.room_id == "courtyard_center"),
            "perimeter room {perimeter_id} connects back to courtyard_center"
        );
    }

    // Perimeter ring connections (clockwise and counter-clockwise)
    let ring_pairs = [
        ("courtyard_north", "courtyard_northeast"),
        ("courtyard_northeast", "courtyard_southeast"),
        ("courtyard_southeast", "courtyard_south"),
        ("courtyard_south", "courtyard_southwest"),
        ("courtyard_southwest", "courtyard_northwest"),
        ("courtyard_northwest", "courtyard_north"),
    ];

    for (room_a_id, room_b_id) in ring_pairs {
        let room_a = pack.room(room_a_id).unwrap();
        let room_b = pack.room(room_b_id).unwrap();
        assert!(
            room_a.exits.iter().any(|e| e.room_id == room_b_id),
            "{room_a_id} connects to {room_b_id}"
        );
        assert!(
            room_b.exits.iter().any(|e| e.room_id == room_a_id),
            "{room_b_id} connects to {room_a_id}"
        );
    }
}

#[test]
fn floor5_sentry_captain_defeat_drops_courtyard_key() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "courtyard_center".to_string();
    state
        .actor_stats
        .entry("player".to_string())
        .or_default()
        .insert("strength".to_string(), 20);

    // Sentry captain carries the cage key
    assert_eq!(
        state.actor_item_count("citadel_sentry_captain", "courtyard-cage-key"),
        1
    );

    // Set captain to 1 HP so a single strike defeats him
    state
        .actor_stats
        .entry("citadel_sentry_captain".to_string())
        .or_default()
        .insert("hp".to_string(), 1);

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");

    // Attack captain to trigger defeat sequence and drop inventory
    let attack_out = runtime
        .run_turn("attack sentry captain")
        .expect("attack captain");
    assert!(
        attack_out.text().contains("strikes")
            || attack_out.text().contains("Sentry Captain")
            || attack_out.text().contains("defeated")
    );

    let state = runtime.export_state().unwrap();
    assert!(
        state.has_item_in_storage(
            "courtyard-cage-key",
            ItemStorageTarget::CurrentRoom,
            "courtyard_center"
        ),
        "courtyard-cage-key should drop into courtyard_center upon captain defeat"
    );

    // Player takes the key
    let take_out = runtime
        .run_turn("take courtyard cage key")
        .expect("take key");
    assert!(
        take_out.text().contains("Picked up") || take_out.text().contains("courtyard cage key")
    );
    let final_state = runtime.export_state().unwrap();
    assert!(final_state.has_item("courtyard-cage-key"));
}

#[test]
fn floor5_unlock_cages_frees_astrid_and_einar() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "courtyard_center".to_string();
    state.acquire_player_item(&pack, "courtyard-cage-key");

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");

    // Astrid and Einar start un-allied
    let s0 = runtime.export_state().unwrap();
    assert_ne!(s0.stance("commander_astrid"), ActorStance::Allied);
    assert_ne!(s0.stance("einar"), ActorStance::Allied);
    assert_ne!(s0.story_vars.get("courtyard_cages_opened"), Some("true"));

    // Unlock cages command
    let unlock_out = runtime.run_turn("unlock cages").expect("unlock cages");
    assert!(
        unlock_out.text().contains("Commander Astrid")
            || unlock_out.text().contains("iron gates swing open")
            || unlock_out.text().contains("weeping in relief")
    );

    let s1 = runtime.export_state().unwrap();
    assert_eq!(s1.story_vars.get("courtyard_cages_opened"), Some("true"));
    assert!(!s1.has_item("courtyard-cage-key"));
    assert_eq!(s1.stance("commander_astrid"), ActorStance::Allied);
    assert_eq!(s1.stance("einar"), ActorStance::Allied);
}

#[test]
fn floor5_sensory_enhancer_use_sets_story_var_and_consumes_item() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "courtyard_center".to_string();
    state.acquire_player_item(&pack, "sensory-enhancer");

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");

    let s0 = runtime.export_state().unwrap();
    assert!(s0.has_item("sensory-enhancer"));
    assert_ne!(s0.story_vars.get("has_sensory_enhancer"), Some("true"));

    // Swallow the sensory enhancer
    let out = runtime
        .run_turn("use sensory enhancer")
        .expect("use enhancer");
    let text = out.text();
    assert!(
        text.contains("translucent blue capsule")
            || text.contains("senses sharpen")
            || text.contains("electric chill")
    );

    let s1 = runtime.export_state().unwrap();
    assert!(!s1.has_item("sensory-enhancer"));
    assert_eq!(s1.story_vars.get("has_sensory_enhancer"), Some("true"));
}

#[test]
fn floor5_citadel_map_definition_and_reveal_condition() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");

    let map = pack
        .map_for_room("courtyard_center")
        .expect("courtyard_center has map");
    assert_eq!(map.id, "the-frost-citadel");
    assert_eq!(map.label, "The Frost Citadel");
    assert_eq!(map.rooms.len(), 7);

    // Each courtyard room is present in the map
    for room_id in EXPECTED_COURTYARD_ROOMS {
        assert!(
            map.rooms.iter().any(|r| r.room_id == *room_id),
            "map should include room {room_id}"
        );
    }

    // Map reveal condition is gated by has_sensory_enhancer
    assert!(
        map.reveal_conditions.iter().any(|c| matches!(
            c,
            cinder_core::content::types::MapRevealCondition::StoryVarTruthy { key } if key == "has_sensory_enhancer"
        )),
        "map should be revealed by has_sensory_enhancer"
    );
}

#[test]
fn floor5_to_floor4_only_via_teleport() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "courtyard_center".to_string();
    state.story_vars.set_unchecked("knows_teleport", "true");
    state
        .story_vars
        .set_unchecked("anchor_floor4_platform", "true");
    state.story_vars.set_unchecked("anchor_floor5_gate", "true");

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");

    // Physical movement up/ascend is rejected because there is no physical connection
    for dir in ["ascend", "up", "go up", "go floor 4"] {
        let outcome = runtime.run_turn(dir).expect("turn runs");
        assert_eq!(
            runtime.current_room_id().unwrap(),
            "courtyard_center",
            "physical command '{dir}' should not move player out of courtyard"
        );
        let text = outcome.text();
        assert!(
            text.contains("cannot go that way")
                || text.contains("can't go that way")
                || text.contains("route sheet")
                || text.contains("approved playbook")
                || text.contains("unknown"),
            "expected cannot go rejection for '{dir}': {text}"
        );
    }

    // Returning to Floor 4 requires the teleport command
    let tp_out = runtime
        .run_turn("teleport teleport_platform")
        .expect("teleport runs");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "teleport_platform",
        "teleporting to teleport_platform moves player to floor 4"
    );
    assert!(
        tp_out
            .text()
            .contains("brass platform glows with blue light")
    );

    // Teleporting back to Floor 5 courtyard also works
    let tp_back = runtime
        .run_turn("teleport floor 5")
        .expect("teleport back runs");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "courtyard_center",
        "teleporting to floor 5 moves player to courtyard_center"
    );
    assert!(
        tp_back
            .text()
            .contains("brass platform glows with blue light")
    );
}
