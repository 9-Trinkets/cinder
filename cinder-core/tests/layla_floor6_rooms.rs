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
    // Inner Sanctuary Core (4 rooms)
    "sanctuary_crossroads",
    "sanctuary_inquisition_archive",
    "herbarium_garden",
    "mortuary_reanimation_sanctum",
];

#[test]
fn floor6_rooms_load_and_validate() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");

    assert_eq!(EXPECTED_FLOOR6_ROOMS.len(), 9);
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
        ("sanctuary_east_promenade", "sanctuary_inquisition_archive"),
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
fn floor6_inner_ring_traversal() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");

    // 4 inner ring rooms in clockwise order:
    // NW (Crossroads) -> NE (Archive) -> SE (Reanimation Sanctum) -> SW (Solarium Garden)
    let inner_ring = [
        "sanctuary_crossroads",
        "sanctuary_inquisition_archive",
        "mortuary_reanimation_sanctum",
        "herbarium_garden",
    ];

    for i in 0..inner_ring.len() {
        let current = inner_ring[i];
        let next = inner_ring[(i + 1) % inner_ring.len()];
        let room = pack.room(current).unwrap();
        assert!(
            room.exits.iter().any(|e| e.room_id == next),
            "inner ring room {current} must connect clockwise to {next}"
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

    // Step in through grand arch into the Crossroads Atrium
    runtime.run_turn("go in").expect("turn runs");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "sanctuary_crossroads",
        "entering in brings Layla to sanctuary_crossroads"
    );

    // Companions still remain on Floor 5 after moving within Floor 6
    let s2 = runtime.export_state().unwrap();
    assert_eq!(
        s2.actor_room_id("commander_astrid", "courtyard_center"),
        "courtyard_center"
    );

    // Walk south into the Solarium Garden
    runtime.run_turn("go south").expect("turn runs");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "herbarium_garden",
        "walking south enters herbarium_garden"
    );

    // Dislodge loose brick and slip through vent out to the Cliff Balcony
    runtime.run_turn("pry brick").expect("turn runs");
    runtime.run_turn("go vent").expect("turn runs");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "herbarium_cliff_balcony",
        "crawling through vent enters herbarium_cliff_balcony"
    );

    // Unbolt door and descend secret spiral stair to the Mortuary Reanimation Sanctum
    runtime.run_turn("unbolt door").expect("turn runs");
    runtime.run_turn("go down").expect("turn runs");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "mortuary_reanimation_sanctum",
        "taking secret stair enters mortuary_reanimation_sanctum"
    );

    // Step out onto the Mountain Relay Platform
    runtime.run_turn("go out").expect("turn runs");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "sanctuary_main_platform",
        "walking out enters sanctuary_main_platform"
    );

    // Walk west along the perimeter to the West Colonnade
    runtime.run_turn("go west").expect("turn runs");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "sanctuary_west_colonnade",
        "walking west enters sanctuary_west_colonnade"
    );

    // Complete the loop north back to Grand Terrace
    runtime.run_turn("go north").expect("turn runs");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "sanctuary_grand_terrace",
        "walking north returns to sanctuary_grand_terrace"
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
