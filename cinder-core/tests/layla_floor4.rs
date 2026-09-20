//! Integration tests for Floor 4 (The Commoners: 9-Room Village Triangle).

use cinder_core::content::loader::load_named_pack;
use cinder_core::engine::state::WorldState;

const EXPECTED_FLOOR4_ROOMS: &[&str] = &[
    "village_square",
    "village_north_gate",
    "village_sw_corner",
    "village_se_corner",
    "village_west_1",
    "village_west_2",
    "village_east_1",
    "village_east_2",
    "village_south_1",
];

#[test]
fn floor4_rooms_and_features_load_and_validate() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");

    assert_eq!(EXPECTED_FLOOR4_ROOMS.len(), 9);
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

    // Village square connects along southern baseline of the triangle loop
    assert!(village_square.exits.iter().any(|e| e.room_id == "village_sw_corner"));
    assert!(village_square.exits.iter().any(|e| e.room_id == "village_south_1"));

    // Fortress apex is locked: no entrance to actual fortress
    let north_gate = pack.room("village_north_gate").expect("village_north_gate exists");
    assert!(
        !north_gate.exits.iter().any(|e| e.room_id == "camp_gate" || e.room_id == "command_tent"),
        "fortress entrance must be inaccessible for now"
    );
    assert!(north_gate.summary.contains("bulkhead"));
}

#[test]
fn floor4_map_layout_registered() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let map = pack
        .maps
        .iter()
        .find(|m| m.id == "the-commoners")
        .expect("the-commoners map exists");

    assert_eq!(map.label, "The Worker Village");
    assert_eq!(map.rooms.len(), 9, "Floor 4 map must have exactly 9 rooms");

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

#[test]
fn floor4_actors_and_interactions_validate() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");

    // Check civilians exist and are placed properly
    let rashid = pack.actor("elder_rashid").expect("elder_rashid exists");
    assert_eq!(rashid.room_id, "village_south_1");
    assert!(!rashid.attackable);

    let yasmin = pack.actor("yasmin").expect("yasmin exists");
    assert_eq!(yasmin.room_id, "village_east_2");
    assert!(!yasmin.attackable);

    let tariq = pack.actor("tariq").expect("tariq exists");
    assert_eq!(tariq.room_id, "village_west_2");
    assert!(!tariq.attackable);

    // Two sentries standing at the northern tip
    let sentries: Vec<_> = pack
        .actors
        .iter()
        .filter(|a| a.room_id == "village_north_gate")
        .collect();
    assert_eq!(sentries.len(), 2, "There must be two sentries at village_north_gate");
    for sentry in &sentries {
        assert!(sentry.attackable);
        assert!(!sentry.initial_hostile, "Sentries should not attack on sight");
    }

    // Deferred actors are offstage
    for deferred_id in &["zayd", "captain_malik", "priest_harun", "sakhra", "garrison_warden"] {
        let actor = pack.actor(deferred_id).expect("deferred actor still defined");
        assert!(actor.room_id.is_empty(), "actor {deferred_id} should be offstage");
    }
}

#[test]
fn floor4_cardinal_and_diagonal_navigation_resolves_cleanly() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");

    // village_north_gate exit resolution
    let vng = pack.room("village_north_gate").expect("village_north_gate exists");
    for exit in &vng.exits {
        if exit.room_id == "village_west_2" {
            assert!(exit.aliases.iter().any(|a| a == "southwest" || a == "sw"));
            assert!(!exit.aliases.iter().any(|a| a == "south" || a == "s"));
        }
        if exit.room_id == "village_east_1" {
            assert!(exit.aliases.iter().any(|a| a == "southeast" || a == "se"));
            assert!(!exit.aliases.iter().any(|a| a == "south" || a == "s"));
        }
    }

    // Verify runtime movement full perimeter walk around the 9-room triangle
    let mut state = WorldState::new(&pack);
    state.current_room_id = "village_square".to_string();
    let dialogue =
        std::sync::Arc::new(cinder_core::engine::dialogue::ScriptedDialogueGenerator::new());
    let runtime =
        cinder_core::engine::runtime::CinderRuntime::with_dialogue_generator(pack.clone(), state, dialogue)
            .expect("runtime creates");

    // Walk clockwise around the 9-room loop:
    // 1. village_square -> west -> village_sw_corner
    let _ = runtime.run_turn("west").expect("turn runs");
    assert_eq!(runtime.current_room_id().unwrap(), "village_sw_corner");

    // 2. village_sw_corner -> northeast -> village_west_1
    let _ = runtime.run_turn("northeast").expect("turn runs");
    assert_eq!(runtime.current_room_id().unwrap(), "village_west_1");

    // 3. village_west_1 -> northeast -> village_west_2 (Tariq)
    let _ = runtime.run_turn("northeast").expect("turn runs");
    assert_eq!(runtime.current_room_id().unwrap(), "village_west_2");

    // 4. village_west_2 -> northeast -> village_north_gate (North Apex)
    let _ = runtime.run_turn("northeast").expect("turn runs");
    assert_eq!(runtime.current_room_id().unwrap(), "village_north_gate");

    // 5. village_north_gate -> southeast -> village_east_1
    let _ = runtime.run_turn("southeast").expect("turn runs");
    assert_eq!(runtime.current_room_id().unwrap(), "village_east_1");

    // 6. village_east_1 -> southeast -> village_east_2 (Yasmin)
    let _ = runtime.run_turn("southeast").expect("turn runs");
    assert_eq!(runtime.current_room_id().unwrap(), "village_east_2");

    // 7. village_east_2 -> southeast -> village_se_corner
    let _ = runtime.run_turn("southeast").expect("turn runs");
    assert_eq!(runtime.current_room_id().unwrap(), "village_se_corner");

    // 8. village_se_corner -> west -> village_south_1 (Elder Rashid)
    let _ = runtime.run_turn("west").expect("turn runs");
    assert_eq!(runtime.current_room_id().unwrap(), "village_south_1");

    // 9. village_south_1 -> west -> village_square (Loop completed)
    let _ = runtime.run_turn("west").expect("turn runs");
    assert_eq!(runtime.current_room_id().unwrap(), "village_square");
}

#[test]
fn floor4_mineral_and_chalk_lore_integrity() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    for room_id in EXPECTED_FLOOR4_ROOMS {
        let room = pack.room(room_id).expect("room exists");
        assert!(!room.summary.to_lowercase().contains("chalk crystals"));
        assert!(!room.inspect_text.to_lowercase().contains("chalk crystals"));
        for f in &room.features {
            assert!(!f.label.to_lowercase().contains("chalk crystals"));
            assert!(!f.inspect_text.to_lowercase().contains("chalk crystals"));
        }
    }
}
