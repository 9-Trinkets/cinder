//! Integration tests for Floor 4 (The Commoners: Village, Mine, Guard Camp).

use cinder_core::content::loader::load_named_pack;
use cinder_core::engine::state::WorldState;

const EXPECTED_FLOOR4_ROOMS: &[&str] = &[
    "village_square",
    "village_north_gate",
    "elder_hut",
    "baker_hut",
    "mine_north_apex",
    "cart_tracks",
    "crystal_pit",
    "old_drain_pipe",
    "camp_gate",
    "command_tent",
    "prison_cage",
    "calcinator_core",
    "teleport_gate",
];

#[test]
fn floor4_rooms_and_features_load_and_validate() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");

    for room_id in EXPECTED_FLOOR4_ROOMS {
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
fn floor4_navigation_and_gates_resolve() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");

    // Floor 3 -> Floor 4 gate
    let oh = pack.room("oh").expect("oh exists");
    let to_village = oh
        .exits
        .iter()
        .find(|e| e.room_id == "village_square")
        .expect("oh has exit to village_square");
    assert_eq!(
        to_village.requires_story_var.as_str(),
        "elemental_released"
    );

    // Village square connects back to oh
    let village_square = pack.room("village_square").expect("village_square exists");
    assert!(
        village_square.exits.iter().any(|e| e.room_id == "oh"),
        "village_square connects up to oh"
    );

    // Secret infiltration: old_drain_pipe connects to command_tent
    let drain_pipe = pack.room("old_drain_pipe").expect("old_drain_pipe exists");
    assert!(
        drain_pipe.exits.iter().any(|e| e.room_id == "command_tent"),
        "drain pipe must connect into command_tent"
    );

    // Front gate: camp_gate connects to command_tent with camp_gate_open gate
    let camp_gate = pack.room("camp_gate").expect("camp_gate exists");
    let gate_exit = camp_gate
        .exits
        .iter()
        .find(|e| e.room_id == "command_tent")
        .expect("camp_gate connects to command_tent");
    assert_eq!(
        gate_exit.requires_story_var.as_str(),
        "camp_gate_open"
    );

    // Guard camp connections
    let command_tent = pack.room("command_tent").expect("command_tent exists");
    assert!(command_tent.exits.iter().any(|e| e.room_id == "prison_cage"));
    assert!(command_tent.exits.iter().any(|e| e.room_id == "teleport_gate"));
    assert!(command_tent.exits.iter().any(|e| e.room_id == "old_drain_pipe"));
}

#[test]
fn floor4_map_layout_registered() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let map = pack
        .maps
        .iter()
        .find(|m| m.id == "the-commoners")
        .expect("the-commoners map exists");

    assert_eq!(map.label, "The Village & Mines");
    assert_eq!(map.rooms.len(), 100, "Floor 4 map must have 100 rooms");

    // Floor 4 must be strictly larger than any previous floor (Floor 1 had 81 rooms)
    let max_previous_size = pack
        .maps
        .iter()
        .filter(|m| m.id != "the-commoners")
        .map(|m| m.rooms.len())
        .max()
        .unwrap_or(0);
    assert!(
        map.rooms.len() > max_previous_size,
        "Floor 4 room count ({}) must exceed previous floors ({})",
        map.rooms.len(),
        max_previous_size
    );

    for room_id in EXPECTED_FLOOR4_ROOMS {
        assert!(
            map.rooms.iter().any(|r| r.room_id == *room_id),
            "map should include room {room_id}"
        );
    }

    // Every room mapped in Floor 4 must load and have features and inspect text
    for map_room in &map.rooms {
        let room = pack
            .room(&map_room.room_id)
            .unwrap_or_else(|| panic!("mapped room {} must exist", map_room.room_id));
        assert!(!room.title.trim().is_empty());
        assert!(!room.summary.trim().is_empty());
        assert!(!room.inspect_text.trim().is_empty());
        assert!(!room.features.is_empty());
    }
}

#[test]
fn floor4_descent_triggers_handler_village_commentary() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    assert!(pack.messages.contains_key("handler.descend.the_village"));

    let mut state = WorldState::new(&pack);
    state.current_room_id = "oh".to_string();
    state.story_vars.set_unchecked("elemental_released", "true");

    let dialogue =
        std::sync::Arc::new(cinder_core::engine::dialogue::ScriptedDialogueGenerator::new());
    let runtime =
        cinder_core::engine::runtime::CinderRuntime::with_dialogue_generator(pack, state, dialogue)
            .expect("runtime creates");

    let outcome = runtime.run_turn("go down").expect("turn runs");
    assert!(
        outcome.text.contains("civilian life signs") || outcome.text.contains("people live down here"),
        "Handler commentary should remark on civilians: {}",
        outcome.text
    );
    assert!(outcome.text.contains("Handler:"));

    // Second movement into the village does not replay the descent line
    let _ = runtime.run_turn("go up").expect("turn runs");
    let outcome2 = runtime.run_turn("go down").expect("turn runs");
    assert!(
        !outcome2.text.contains("civilian life signs"),
        "Descent line must only play on first visit"
    );
}
