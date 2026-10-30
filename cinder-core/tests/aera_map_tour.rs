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
    assert_eq!(pack.rooms.len(), 10, "expected 10 rooms in Da'an villa");

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
fn player_can_tour_entire_daan_villa() {
    let runtime = aera_runtime();

    // Start in front entry
    assert_eq!(runtime.current_room_id().unwrap(), "entrance");

    // Inspect camphor shoe bench
    let look_bench = runtime.run_turn("look bench").expect("look bench");
    assert!(
        look_bench
            .lines
            .iter()
            .any(|line| line.text.contains("fragrant slab of camphor wood"))
    );

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

#[test]
fn player_can_operate_doors_and_doors_gate_movement() {
    let runtime = aera_runtime();

    // Move from entrance to lounge
    runtime.run_turn("go lounge").expect("go lounge");
    assert_eq!(runtime.current_room_id().unwrap(), "lounge");

    // Door starts open
    let look_open = runtime
        .run_turn("look confessional-door")
        .expect("look door");
    assert!(
        look_open
            .lines
            .iter()
            .any(|l| l.text.contains("stands ajar"))
    );

    // Close the confessional door
    let close_res = runtime
        .run_turn("do close confessional door")
        .expect("close door");
    assert!(
        close_res
            .lines
            .iter()
            .any(|l| l.text.contains("magnetic latch seals"))
    );

    // Inspecting now shows it sealed
    let look_closed = runtime
        .run_turn("look confessional-door")
        .expect("look door closed");
    assert!(
        look_closed
            .lines
            .iter()
            .any(|l| l.text.contains("amber 'ON AIR' indicator glows"))
    );

    // Attempting to go to confessional fails while closed
    runtime
        .run_turn("go confessional")
        .expect("go confessional");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "lounge",
        "movement should be blocked by closed door"
    );

    // Open the door
    runtime
        .run_turn("do open confessional door")
        .expect("open door");

    // Now movement succeeds
    runtime
        .run_turn("go confessional")
        .expect("go confessional");
    assert_eq!(runtime.current_room_id().unwrap(), "confessional");

    // Inside confessional, close the door from within
    runtime
        .run_turn("do close confessional door")
        .expect("close from inside");
    let look_inside = runtime
        .run_turn("look confessional-door")
        .expect("look door inside");
    assert!(
        look_inside
            .lines
            .iter()
            .any(|l| l.text.contains("sealed shut from the inside"))
    );

    // Leaving is blocked while closed
    runtime.run_turn("go lounge").expect("go lounge blocked");
    assert_eq!(runtime.current_room_id().unwrap(), "confessional");

    // Open the door from inside
    runtime
        .run_turn("do open confessional door")
        .expect("open from inside");
    runtime.run_turn("go lounge").expect("go lounge");
    assert_eq!(runtime.current_room_id().unwrap(), "lounge");

    // Test sliding door in 2F studio / girls-room
    runtime.run_turn("go studio").expect("go studio");
    assert_eq!(runtime.current_room_id().unwrap(), "studio");

    // Close sliding door
    runtime
        .run_turn("do slide bedroom door shut")
        .expect("slide shut");
    runtime
        .run_turn("go girls-room")
        .expect("go girls-room blocked");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "studio",
        "movement should be blocked by closed sliding door"
    );

    // Reopen and enter
    runtime
        .run_turn("do slide bedroom door open")
        .expect("slide open");
    runtime.run_turn("go girls-room").expect("go girls-room");
    assert_eq!(runtime.current_room_id().unwrap(), "girls-room");

    // Return to studio and head up to rooftop
    runtime.run_turn("go studio").expect("go studio");
    runtime.run_turn("go rooftop").expect("go rooftop");
    assert_eq!(runtime.current_room_id().unwrap(), "rooftop");

    // Test guys-door on 3F
    runtime
        .run_turn("shut terrace door")
        .expect("shut terrace door directly");
    runtime
        .run_turn("go guys-room")
        .expect("go guys-room blocked");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "rooftop",
        "movement should be blocked by closed terrace door"
    );

    // Reopen terrace door directly
    runtime
        .run_turn("open terrace door")
        .expect("open terrace door directly");
    runtime.run_turn("go guys-room").expect("go guys-room");
    assert_eq!(runtime.current_room_id().unwrap(), "guys-room");

    // Head back down to lounge
    runtime.run_turn("go rooftop").expect("go rooftop");
    runtime.run_turn("go studio").expect("go studio");
    runtime.run_turn("go lounge").expect("go lounge");
    assert_eq!(runtime.current_room_id().unwrap(), "lounge");

    // Test courtyard accordion doors
    runtime
        .run_turn("close accordion doors")
        .expect("close accordion doors");
    runtime
        .run_turn("go courtyard")
        .expect("go courtyard blocked");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "lounge",
        "movement should be blocked by closed accordion doors"
    );

    // Reopen accordion doors
    runtime
        .run_turn("open accordion doors")
        .expect("open accordion doors");
    runtime.run_turn("go courtyard").expect("go courtyard");
    assert_eq!(runtime.current_room_id().unwrap(), "courtyard");
}
