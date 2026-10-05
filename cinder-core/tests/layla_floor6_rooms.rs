//! Integration tests for Floor 6 (The High Sanctuary: Outer Wall & 4 Quadrants).

use cinder_core::content::loader::load_named_pack;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::WorldState;

const EXPECTED_FLOOR6_ROOMS: &[&str] = &[
    // Central nexus & entrance
    "sanctuary_grand_terrace",
    "sanctuary_crossroads",
    // Zone 1: Grand Basilica (NW)
    "basilica_portal",
    "basilica_nave",
    "basilica_high_altar",
    "basilica_vestry",
    "basilica_clergy_cells",
    // Zone 2: Scriptorium & Archive (NE)
    "scriptorium_portal",
    "scriptorium_cloister",
    "scriptorium_library",
    "scriptorium_archive_vault",
    "scriptorium_secret_archive",
    // Zone 3: Herbarium & Dispensary (SW)
    "herbarium_portal",
    "herbarium_garden",
    "herbarium_apothecary",
    "herbarium_still_room",
    "herbarium_water_clock",
    // Zone 4: Mortuary Crypts & Platform (SE)
    "mortuary_portal",
    "mortuary_descent",
    "mortuary_crypt",
    "mortuary_embalming_hall",
    "mortuary_reanimation_sanctum",
    "sanctuary_main_platform",
    // Outer Wall Square (8 rooms)
    "sanctuary_wall_nw_tower",
    "sanctuary_wall_north",
    "sanctuary_wall_ne_tower",
    "sanctuary_wall_east",
    "sanctuary_wall_se_tower",
    "sanctuary_wall_south",
    "sanctuary_wall_sw_tower",
    "sanctuary_wall_west",
];

#[test]
fn floor6_rooms_load_and_validate() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");

    assert_eq!(EXPECTED_FLOOR6_ROOMS.len(), 31);
    for room_id in EXPECTED_FLOOR6_ROOMS {
        let room = pack
            .room(room_id)
            .unwrap_or_else(|| panic!("missing Floor 6 room {room_id}"));
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
fn floor6_all_exits_are_bidirectional() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");

    // Floor 5 exit to Floor 6
    let gate = pack
        .room("citadel_sanctum_gate")
        .expect("citadel_sanctum_gate exists");
    assert!(
        gate.exits
            .iter()
            .any(|e| e.room_id == "sanctuary_grand_terrace"),
        "citadel_sanctum_gate must have exit to sanctuary_grand_terrace"
    );

    for room_id in EXPECTED_FLOOR6_ROOMS {
        let room = pack.room(room_id).unwrap();
        for exit in &room.exits {
            let target = pack
                .room(&exit.room_id)
                .unwrap_or_else(|| panic!("exit in {room_id} points to missing room {}", exit.room_id));
            assert!(
                target.exits.iter().any(|e| e.room_id == *room_id),
                "room {} points to {}, but {} has no return exit back to {}",
                room_id,
                exit.room_id,
                exit.room_id,
                room_id
            );
        }
    }
}

#[test]
fn floor6_outer_wall_ring_traversal() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");

    // 8 perimeter rooms in clockwise order
    let wall_ring = [
        "sanctuary_wall_nw_tower",
        "sanctuary_wall_north",
        "sanctuary_wall_ne_tower",
        "sanctuary_wall_east",
        "sanctuary_wall_se_tower",
        "sanctuary_wall_south",
        "sanctuary_wall_sw_tower",
        "sanctuary_wall_west",
    ];

    for i in 0..wall_ring.len() {
        let current = wall_ring[i];
        let next = wall_ring[(i + 1) % wall_ring.len()];
        let room = pack.room(current).unwrap();
        assert!(
            room.exits.iter().any(|e| e.room_id == next),
            "outer wall room {current} must connect clockwise to {next}"
        );
    }

    // Corner towers connect down to interior zones
    let corner_connections = [
        ("sanctuary_wall_nw_tower", "basilica_nave"),
        ("sanctuary_wall_ne_tower", "scriptorium_cloister"),
        ("sanctuary_wall_sw_tower", "herbarium_garden"),
        ("sanctuary_wall_se_tower", "mortuary_crypt"),
    ];

    for (tower, interior) in corner_connections {
        let t_room = pack.room(tower).unwrap();
        let i_room = pack.room(interior).unwrap();
        assert!(
            t_room.exits.iter().any(|e| e.room_id == interior),
            "{tower} must connect down to {interior}"
        );
        assert!(
            i_room.exits.iter().any(|e| e.room_id == tower),
            "{interior} must connect up to {tower}"
        );
    }
}

#[test]
fn floor6_central_crossroads_connects_to_all_four_zones() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let crossroads = pack
        .room("sanctuary_crossroads")
        .expect("sanctuary_crossroads exists");

    let expected_portals = [
        "basilica_portal",
        "scriptorium_portal",
        "herbarium_portal",
        "mortuary_portal",
        "sanctuary_grand_terrace",
    ];

    for portal in expected_portals {
        assert!(
            crossroads.exits.iter().any(|e| e.room_id == portal),
            "sanctuary_crossroads must connect to {portal}"
        );
        let p_room = pack.room(portal).unwrap();
        assert!(
            p_room.exits.iter().any(|e| e.room_id == "sanctuary_crossroads"),
            "{portal} must connect back to sanctuary_crossroads"
        );
    }
}

#[test]
fn floor6_layla_can_walk_from_floor5_into_sanctuary_and_explore() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "citadel_sanctum_gate".to_string();

    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime creates");

    // Walk south from Floor 5 into Floor 6 Gate Terrace
    runtime.run_turn("go south").expect("turn runs");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "sanctuary_grand_terrace",
        "walking south from Floor 5 gate brings Layla to sanctuary_grand_terrace"
    );

    // Walk north across terrace into the Grand Plaza
    runtime.run_turn("go north").expect("turn runs");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "sanctuary_crossroads",
        "walking north enters sanctuary_crossroads"
    );

    // Walk northwest to the Basilica Portal
    runtime.run_turn("go northwest").expect("turn runs");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "basilica_portal",
        "walking northwest enters basilica_portal"
    );

    // Walk north into the Nave
    runtime.run_turn("go north").expect("turn runs");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "basilica_nave",
        "walking north enters basilica_nave"
    );

    // Climb spiral stairs to the Northwest Bell Tower (Outer Wall)
    runtime.run_turn("go up").expect("turn runs");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "sanctuary_wall_nw_tower",
        "walking up enters sanctuary_wall_nw_tower"
    );

    // Walk south along the West Rampart
    runtime.run_turn("go south").expect("turn runs");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "sanctuary_wall_west",
        "walking south enters sanctuary_wall_west"
    );

    // Continue south to the Southwest Garden Bastion
    runtime.run_turn("go south").expect("turn runs");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "sanctuary_wall_sw_tower",
        "walking south enters sanctuary_wall_sw_tower"
    );

    // Climb down into the Herbarium Solarium Garden
    runtime.run_turn("go down").expect("turn runs");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "herbarium_garden",
        "walking down enters herbarium_garden"
    );

    // Walk north out through the Arch of Remedies
    runtime.run_turn("go north").expect("turn runs");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "herbarium_portal",
        "walking north enters herbarium_portal"
    );

    // Return to the Grand Plaza
    runtime.run_turn("go northeast").expect("turn runs");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "sanctuary_crossroads",
        "walking northeast returns to sanctuary_crossroads"
    );
}

#[test]
fn floor6_map_is_defined_in_maps_json() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let map = pack
        .map_for_room("sanctuary_crossroads")
        .expect("sanctuary_crossroads has map");

    assert_eq!(map.id, "the-high-sanctuary");
    assert_eq!(map.label, "The High Sanctuary");
    for room_id in EXPECTED_FLOOR6_ROOMS {
        assert!(
            map.rooms.iter().any(|r| r.room_id == *room_id),
            "map must contain room {room_id}"
        );
    }
}
