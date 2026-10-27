//! Floor 6 (the High Sanctuary): the rooms load, their exits are two-way, and
//! each secret room opens only when the player operates the object that hides it.

use cinder_core::content::loader::load_named_pack;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::WorldState;

const SURFACE: &[&str] = &[
    "sanctuary_terrace",
    "sanctuary_nave",
    "sanctuary_cistern_tower",
    "sanctuary_knights_hall",
    "sanctuary_censer_tower",
    "sanctuary_chapel",
    "sanctuary_bell_tower",
    "sanctuary_scriptorium",
    "sanctuary_lamp_tower",
    "sanctuary_garden",
    "sanctuary_cell_1",
    "sanctuary_cell_2",
    "sanctuary_cell_3",
    "sanctuary_cell_4",
    "sanctuary_cell_5",
    "sanctuary_cell_6",
    "sanctuary_cell_7",
    "sanctuary_ledge",
    "sanctuary_cliff_garden",
];
const VAULTS: &[&str] = &[
    "sanctuary_spiral_stair",
    "sanctuary_sanctum",
    "sanctuary_control_hall",
];

fn runtime_in(room: &str) -> CinderRuntime {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = room.to_string();
    state.clock.turn_number = 1;
    CinderRuntime::from_state(pack, state, false).expect("runtime creates")
}

fn go(runtime: &CinderRuntime, command: &str) -> String {
    runtime.run_turn(command).expect("turn runs");
    runtime.current_room_id().expect("room")
}

#[test]
fn every_floor6_room_loads_with_text_and_features() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    for id in SURFACE.iter().chain(VAULTS) {
        let room = pack.room(id).unwrap_or_else(|| panic!("missing room {id}"));
        assert!(!room.title.trim().is_empty(), "{id} title");
        assert!(!room.summary.trim().is_empty(), "{id} summary");
        assert!(!room.inspect_text.trim().is_empty(), "{id} inspect_text");
        assert!(!room.features.is_empty(), "{id} needs a feature");
    }
}

#[test]
fn every_exit_has_a_way_back() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    for id in SURFACE.iter().chain(VAULTS) {
        for exit in &pack.room(id).unwrap().exits {
            let target = pack
                .room(&exit.room_id)
                .unwrap_or_else(|| panic!("{id} leads to missing {}", exit.room_id));
            assert!(
                target.exits.iter().any(|back| back.room_id == *id),
                "{} has no exit back to {id}",
                exit.room_id
            );
        }
    }
}

#[test]
fn the_maps_hold_exactly_the_floor6_rooms() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    for (map_id, rooms) in [("the-high-sanctuary", SURFACE), ("the-vaults", VAULTS)] {
        let map = pack
            .maps
            .iter()
            .find(|map| map.id == map_id)
            .unwrap_or_else(|| panic!("missing map {map_id}"));
        let mut listed: Vec<&str> = map.rooms.iter().map(|r| r.room_id.as_str()).collect();
        let mut expected: Vec<&str> = rooms.to_vec();
        listed.sort_unstable();
        expected.sort_unstable();
        assert_eq!(listed, expected, "{map_id} rooms");
    }
}

#[test]
fn the_arch_stays_shut_until_a_gap_is_traced() {
    let runtime = runtime_in("sanctuary_terrace");
    let look = runtime.run_turn("look").expect("look");
    assert!(
        look.text().contains("chalk") && look.text().contains("Do menu"),
        "the terrace must say how to get through: {}",
        look.text()
    );
    assert_eq!(go(&runtime, "go north"), "sanctuary_terrace");
    assert_eq!(go(&runtime, "trace gap"), "sanctuary_terrace");
    assert_eq!(go(&runtime, "go north"), "sanctuary_nave");
    assert_eq!(go(&runtime, "go south"), "sanctuary_terrace");
}

#[test]
fn the_ring_and_the_garden_wheel_connect() {
    let runtime = runtime_in("sanctuary_nave");
    // Around the square, clockwise from the Nave.
    for (command, room) in [
        ("go east", "sanctuary_cistern_tower"),
        ("go north", "sanctuary_knights_hall"),
        ("go north", "sanctuary_censer_tower"),
        ("go west", "sanctuary_chapel"),
        ("go west", "sanctuary_bell_tower"),
        ("go south", "sanctuary_scriptorium"),
        ("go south", "sanctuary_lamp_tower"),
        ("go east", "sanctuary_nave"),
    ] {
        assert_eq!(go(&runtime, command), room, "{command}");
    }
    // Into the garden, round the cells and back out.
    assert_eq!(go(&runtime, "go north"), "sanctuary_garden");
    assert_eq!(go(&runtime, "go cell 3"), "sanctuary_cell_3");
    assert_eq!(go(&runtime, "go next cell"), "sanctuary_cell_4");
    assert_eq!(go(&runtime, "go out"), "sanctuary_garden");
}

#[test]
fn cell_seven_hides_the_ledge_the_boulder_hides_the_garden_and_the_door_hides_the_stair() {
    let runtime = runtime_in("sanctuary_cell_7");
    assert_eq!(go(&runtime, "go window"), "sanctuary_cell_7");
    go(&runtime, "slide lattice");
    assert_eq!(go(&runtime, "go window"), "sanctuary_ledge");

    assert_eq!(go(&runtime, "go ahead"), "sanctuary_ledge");
    go(&runtime, "squeeze past");
    assert_eq!(go(&runtime, "go ahead"), "sanctuary_cliff_garden");

    // Only Orva opens the door, for someone who knows Einar's words, and she
    // hands over the spirit capsule while she does it.
    assert_eq!(go(&runtime, "go down"), "sanctuary_cliff_garden");
    go(&runtime, "unlock door");
    assert_eq!(go(&runtime, "go down"), "sanctuary_cliff_garden");
    assert!(!runtime.export_state().unwrap().has_item("spirit-pill"));
    go(&runtime, "say the words");
    assert!(runtime.export_state().unwrap().has_item("spirit-pill"));
    assert_eq!(go(&runtime, "go down"), "sanctuary_spiral_stair");
    assert_eq!(go(&runtime, "go down"), "sanctuary_sanctum");
}

#[test]
fn the_spirit_capsule_lets_layla_see_the_souls_in_the_sanctum() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "sanctuary_sanctum".to_string();
    state.acquire_player_item(&pack, "spirit-pill");
    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime creates");

    let before = runtime.run_turn("look at ledgers").expect("look");
    assert!(!before.text().contains("Siraj"), "{}", before.text());

    runtime.run_turn("use spirit capsule").expect("swallow it");
    assert_eq!(
        runtime
            .export_state()
            .unwrap()
            .story_vars
            .get("spiritual_sight"),
        Some("true")
    );
    let room = runtime.run_turn("look").expect("look");
    assert!(
        room.text().contains("Each spirit is a glow"),
        "{}",
        room.text()
    );
    let after = runtime.run_turn("look at ledgers").expect("look");
    for name in ["Siraj", "Nur", "Orin", "Mari", "Ferid", "Hana"] {
        assert!(
            after.text().contains(name),
            "{name} missing: {}",
            after.text()
        );
    }
}

#[test]
fn orva_waits_in_the_cliff_garden_and_einar_gives_the_words() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let orva = pack.actor("orva").expect("orva exists");
    assert_eq!(orva.room_id, "sanctuary_cliff_garden");
    assert!(!orva.attackable);
    let beats = serde_json::to_string(&pack.beats).expect("beats serialise");
    assert!(
        beats.contains("The arrow always finds home"),
        "Einar's briefing must give Layla the words"
    );
}

#[test]
fn the_sanctum_bar_opens_the_chapel_stair_from_both_sides() {
    let runtime = runtime_in("sanctuary_sanctum");
    assert_eq!(go(&runtime, "go north"), "sanctuary_sanctum");
    go(&runtime, "lift bar");
    assert_eq!(go(&runtime, "go north"), "sanctuary_chapel");
    assert_eq!(go(&runtime, "go down"), "sanctuary_sanctum");
    assert_eq!(go(&runtime, "go east"), "sanctuary_control_hall");
}

#[test]
fn the_chapel_door_stays_sealed_from_above() {
    let runtime = runtime_in("sanctuary_chapel");
    assert_eq!(go(&runtime, "go down"), "sanctuary_chapel");
}

#[test]
fn the_clandestine_token_lands_layla_alone_on_the_terrace() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "courtyard_center".to_string();
    state.acquire_player_item(&pack, "clandestine-sanctuary-token");
    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime creates");
    runtime
        .run_turn("use clandestine-sanctuary-token")
        .expect("token is used");
    assert_eq!(runtime.current_room_id().unwrap(), "sanctuary_terrace");
    let state = runtime.export_state().unwrap();
    assert_eq!(state.story_vars.get("entered_floor6"), Some("true"));
}
