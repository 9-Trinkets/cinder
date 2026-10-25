//! Integration tests for Floor 6 (The High Sanctuary: Concentric Square Rings).

use cinder_core::content::loader::load_named_pack;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::WorldState;

const EXPECTED_FLOOR6_ROOMS: &[&str] = &[
    // Outer Ring (4 rooms) + Cliff Balcony Annex (1 room)
    "sanctuary_grand_terrace",
    "sanctuary_east_promenade",
    "sanctuary_main_platform",
    "sanctuary_west_colonnade",
    "herbarium_cliff_balcony",
    // Inner Sanctuary Core (3 rooms)
    "sanctuary_crossroads",
    "herbarium_garden",
    "mortuary_reanimation_sanctum",
];

#[test]
fn floor6_rooms_load_and_validate() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");

    assert_eq!(EXPECTED_FLOOR6_ROOMS.len(), 8);
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

    for room_id in EXPECTED_FLOOR6_ROOMS {
        let room = pack.room(room_id).unwrap();
        for exit in &room.exits {
            let target = pack.room(&exit.room_id).unwrap_or_else(|| {
                panic!("exit in {room_id} points to missing room {}", exit.room_id)
            });
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
fn floor6_outer_ring_traversal() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");

    // 4 outer ring rooms in clockwise order:
    // North (Grand Terrace) -> East (East Promenade) -> South (Main Platform) -> West (West Colonnade)
    let outer_ring = [
        "sanctuary_grand_terrace",
        "sanctuary_east_promenade",
        "sanctuary_main_platform",
        "sanctuary_west_colonnade",
    ];

    for i in 0..outer_ring.len() {
        let current = outer_ring[i];
        let next = outer_ring[(i + 1) % outer_ring.len()];
        let room = pack.room(current).unwrap();
        assert!(
            room.exits.iter().any(|e| e.room_id == next),
            "outer ring room {current} must connect clockwise to {next}"
        );
    }

    // Outer ring connects inwards to the inner sanctuary core
    let inner_connections = [
        ("sanctuary_grand_terrace", "sanctuary_crossroads"),
        ("sanctuary_west_colonnade", "herbarium_garden"),
        ("sanctuary_main_platform", "mortuary_reanimation_sanctum"),
    ];

    for (outer, inner) in inner_connections {
        let o_room = pack.room(outer).unwrap();
        let i_room = pack.room(inner).unwrap();
        assert!(
            o_room.exits.iter().any(|e| e.room_id == inner),
            "{outer} must connect inwards to {inner}"
        );
        assert!(
            i_room.exits.iter().any(|e| e.room_id == outer),
            "{inner} must connect outwards to {outer}"
        );
    }
}

#[test]
fn floor6_inner_crossroads_connections() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let crossroads = pack
        .room("sanctuary_crossroads")
        .expect("sanctuary_crossroads exists");

    let expected_connections = [
        "sanctuary_grand_terrace",
        "herbarium_garden",
        "mortuary_reanimation_sanctum",
    ];

    for room_id in expected_connections {
        assert!(
            crossroads.exits.iter().any(|e| e.room_id == room_id),
            "sanctuary_crossroads must connect to {room_id}"
        );
        let c_room = pack.room(room_id).unwrap();
        assert!(
            c_room
                .exits
                .iter()
                .any(|e| e.room_id == "sanctuary_crossroads"),
            "{room_id} must connect back to sanctuary_crossroads"
        );
    }
}

#[test]
fn floor6_layla_teleports_alone_into_sanctuary_and_explores() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "courtyard_center".to_string();

    // Allies in Floor 5 courtyard follow the player
    state.set_follows_player("commander_astrid", true);
    state.set_follows_player("einar", true);
    state.actors.actor_room_overrides.insert(
        "commander_astrid".to_string(),
        "courtyard_center".to_string(),
    );
    state
        .actors
        .actor_room_overrides
        .insert("einar".to_string(), "courtyard_center".to_string());

    // Give Layla the clandestine token
    state.add_item("clandestine-sanctuary-token");

    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime creates");

    // Consume the token to solo teleport into Floor 6 Gate Terrace
    let outcome = runtime
        .run_turn("use clandestine-sanctuary-token")
        .expect("turn runs");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "sanctuary_grand_terrace",
        "consuming clandestine token brings Layla to sanctuary_grand_terrace"
    );
    assert!(
        outcome.text().contains("High Sanctuary"),
        "Handler arrival line for High Sanctuary should be narrated, got: {}",
        outcome.text()
    );

    // Verify party members were left behind in Floor 5 courtyard (Party Disconnection)
    let s = runtime.export_state().unwrap();
    assert_eq!(
        s.actor_room_id("commander_astrid", "courtyard_center"),
        "courtyard_center",
        "Astrid must stay on Floor 5"
    );
    assert_eq!(
        s.actor_room_id("einar", "courtyard_center"),
        "courtyard_center",
        "Einar must stay on Floor 5"
    );

    // Layla cannot order off-floor party members
    let order_outcome = runtime.run_turn("order astrid guard").expect("turn runs");
    assert!(
        order_outcome
            .text()
            .contains("That companion is not close enough to receive your order"),
        "cannot issue party orders to off-floor companions, got: {}",
        order_outcome.text()
    );

    // Walk south across terrace into the Crossroads
    runtime.run_turn("go south").expect("turn runs");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "sanctuary_crossroads",
        "walking south enters sanctuary_crossroads"
    );

    // Companions still remain on Floor 5 after moving within Floor 6
    let s2 = runtime.export_state().unwrap();
    assert_eq!(
        s2.actor_room_id("commander_astrid", "courtyard_center"),
        "courtyard_center"
    );

    // Walk west into the Solarium Garden
    runtime.run_turn("go west").expect("turn runs");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "herbarium_garden",
        "walking west enters herbarium_garden"
    );

    // Slip through vent out to the Cliff Balcony
    runtime.run_turn("go out").expect("turn runs");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "herbarium_cliff_balcony",
        "crawling through vent enters herbarium_cliff_balcony"
    );

    // Descend secret spiral stair to the Mortuary Reanimation Sanctum
    runtime.run_turn("go down").expect("turn runs");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "mortuary_reanimation_sanctum",
        "taking secret stair enters mortuary_reanimation_sanctum"
    );

    // Step south out to the Mountain Relay Platform
    runtime.run_turn("go south").expect("turn runs");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "sanctuary_main_platform",
        "walking south enters sanctuary_main_platform"
    );

    // Walk northwest along the West Colonnade
    runtime.run_turn("go northwest").expect("turn runs");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "sanctuary_west_colonnade",
        "walking northwest enters sanctuary_west_colonnade"
    );

    // Complete the loop northeast back to Grand Terrace
    runtime.run_turn("go northeast").expect("turn runs");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "sanctuary_grand_terrace",
        "walking northeast returns to sanctuary_grand_terrace"
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
