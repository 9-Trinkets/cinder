//! Integration test verifying full map navigation and room inspection across the Da'an villa.

use cinder_core::content::loader::load_named_pack;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::WorldState;

fn aera_runtime() -> CinderRuntime {
    let pack = load_named_pack("aera", Some("en")).expect("aera loads");
    let state = WorldState::new(&pack);
    CinderRuntime::from_state(pack, state, false).expect("runtime creates")
}

#[test]
fn every_aera_room_loads_with_complete_flavor_and_features() {
    let pack = load_named_pack("aera", Some("en")).expect("aera loads");
    assert_eq!(
        pack.rooms.len(),
        12,
        "expected 12 rooms in Da'an villa + video village"
    );

    for room in &pack.rooms {
        assert!(
            !room.title.trim().is_empty(),
            "room '{}' missing title",
            room.id
        );
        assert!(
            !room.summary.trim().is_empty(),
            "room '{}' missing summary",
            room.id
        );
        assert!(
            !room.inspect_text.trim().is_empty(),
            "room '{}' missing inspect_text",
            room.id
        );
        assert!(
            !room.features.is_empty(),
            "room '{}' should have at least one feature",
            room.id
        );
        assert!(
            !room.exits.is_empty(),
            "room '{}' must have exits to be reachable",
            room.id
        );

        for feature in &room.features {
            assert!(
                !feature.label.trim().is_empty(),
                "feature '{}' in room '{}' missing label",
                feature.id,
                room.id
            );
            assert!(
                !feature.inspect_text.trim().is_empty(),
                "feature '{}' in room '{}' missing inspect_text",
                feature.id,
                room.id
            );
        }
    }
}

#[test]
fn player_can_tour_entire_daan_villa_from_control_room() {
    let runtime = aera_runtime();

    // Start in control room
    assert_eq!(runtime.current_room_id().unwrap(), "control-room");

    // Walk into the alley
    runtime.run_turn("go alley").expect("go alley");
    assert_eq!(runtime.current_room_id().unwrap(), "alley");

    // Inspect bougainvillea feature
    let look_flowers = runtime
        .run_turn("look bougainvillea")
        .expect("look bougainvillea");
    assert!(
        look_flowers
            .lines
            .iter()
            .any(|line| line.text.contains("magenta bougainvillea"))
    );

    // Enter the villa genkan
    runtime.run_turn("go entrance").expect("go entrance");
    assert_eq!(runtime.current_room_id().unwrap(), "entrance");

    // Head into the open kitchen
    runtime.run_turn("go kitchen").expect("go kitchen");
    assert_eq!(runtime.current_room_id().unwrap(), "kitchen");

    // Step into the dining room
    runtime.run_turn("go dining-room").expect("go dining-room");
    assert_eq!(runtime.current_room_id().unwrap(), "dining-room");

    // Step into the interior courtyard
    runtime.run_turn("go courtyard").expect("go courtyard");
    assert_eq!(runtime.current_room_id().unwrap(), "courtyard");

    // Step into the sunken living lounge
    runtime.run_turn("go lounge").expect("go lounge");
    assert_eq!(runtime.current_room_id().unwrap(), "lounge");

    // Inspect modular sofa
    let look_sofa = runtime.run_turn("look sofa").expect("look sofa");
    assert!(
        look_sofa
            .lines
            .iter()
            .any(|line| line.text.contains("textured slate linen"))
    );

    // Go down to the soundproof Diary Room (Basement)
    runtime
        .run_turn("go confessional")
        .expect("go confessional");
    assert_eq!(runtime.current_room_id().unwrap(), "confessional");

    // Return to the lounge
    runtime.run_turn("go lounge").expect("go lounge");
    assert_eq!(runtime.current_room_id().unwrap(), "lounge");

    // Climb stairs to 2F Design Studio
    runtime.run_turn("go studio").expect("go studio");
    assert_eq!(runtime.current_room_id().unwrap(), "studio");

    // Check the Girls' Dormitory on 2F
    runtime.run_turn("go girls-room").expect("go girls-room");
    assert_eq!(runtime.current_room_id().unwrap(), "girls-room");

    // Back to studio
    runtime.run_turn("go studio").expect("go studio");
    assert_eq!(runtime.current_room_id().unwrap(), "studio");

    // Climb stairs to 3F Rooftop Terrace
    runtime.run_turn("go rooftop").expect("go rooftop");
    assert_eq!(runtime.current_room_id().unwrap(), "rooftop");

    // Check the Guys' Dormitory on 3F
    runtime.run_turn("go guys-room").expect("go guys-room");
    assert_eq!(runtime.current_room_id().unwrap(), "guys-room");

    // Return to rooftop
    runtime.run_turn("go rooftop").expect("go rooftop");
    assert_eq!(runtime.current_room_id().unwrap(), "rooftop");
}
