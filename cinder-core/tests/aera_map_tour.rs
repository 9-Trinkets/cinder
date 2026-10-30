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

#[test]
fn floor_transitions_have_clear_indicators_in_titles_exits_and_panels() {
    let pack = load_named_pack("aera", Some("en")).expect("aera loads");

    // 1. Every room has a first-class floor attribute, while titles stay clean without parentheticals
    for room in &pack.rooms {
        let floor = pack.room_floor(&room.id);
        assert!(
            matches!(floor, Some("1F" | "2F" | "3F" | "B1")),
            "room '{}' should have a valid floor attribute, got {:?}",
            room.id,
            floor
        );
        let has_unclean_parenthetical = room.title.contains("(1F)")
            || room.title.contains("(2F)")
            || room.title.contains("(3F)")
            || room.title.contains("(B1)");
        assert!(
            !has_unclean_parenthetical,
            "room '{}' title '{}' should not contain floor parenthetical",
            room.id, room.title
        );
    }

    // 2. Inter-floor exits must declare menu_label and label indicating destination floor
    let lounge = pack.room("lounge").unwrap();
    let to_studio = lounge.exits.iter().find(|e| e.room_id == "studio").unwrap();
    assert!(to_studio.label.contains("2F"));
    assert!(to_studio.menu_label.as_ref().unwrap().contains("2F"));

    let to_confessional = lounge
        .exits
        .iter()
        .find(|e| e.room_id == "confessional")
        .unwrap();
    assert!(to_confessional.label.contains("B1"));
    assert!(to_confessional.menu_label.as_ref().unwrap().contains("B1"));

    let studio = pack.room("studio").unwrap();
    let to_lounge = studio.exits.iter().find(|e| e.room_id == "lounge").unwrap();
    assert!(to_lounge.label.contains("1F"));
    assert!(to_lounge.menu_label.as_ref().unwrap().contains("1F"));

    let to_rooftop = studio
        .exits
        .iter()
        .find(|e| e.room_id == "rooftop")
        .unwrap();
    assert!(to_rooftop.label.contains("3F"));
    assert!(to_rooftop.menu_label.as_ref().unwrap().contains("3F"));

    let rooftop = pack.room("rooftop").unwrap();
    let roof_to_studio = rooftop
        .exits
        .iter()
        .find(|e| e.room_id == "studio")
        .unwrap();
    assert!(roof_to_studio.label.contains("2F"));
    assert!(roof_to_studio.menu_label.as_ref().unwrap().contains("2F"));

    let confessional = pack.room("confessional").unwrap();
    let conf_to_lounge = confessional
        .exits
        .iter()
        .find(|e| e.room_id == "lounge")
        .unwrap();
    assert!(conf_to_lounge.label.contains("1F"));
    assert!(conf_to_lounge.menu_label.as_ref().unwrap().contains("1F"));

    // 3. Runtime inspection and movement using intuitive directional/stair commands
    let runtime = aera_runtime();

    // From entrance, go to lounge
    runtime.run_turn("go lounge").expect("go lounge");
    assert_eq!(runtime.current_room_id().unwrap(), "lounge");

    // Inspecting room reveals the exits including floor details
    let look_lounge = runtime.run_turn("look").expect("look lounge");
    let look_text = look_lounge
        .lines
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(look_text.contains("Exits:"));
    assert!(look_text.contains("stairs up to 2F"));
    assert!(look_text.contains("stairs down to B1"));

    // Inspecting stairs in lounge
    let look_stairs = runtime.run_turn("look stairs").expect("look stairs");
    assert!(
        look_stairs
            .lines
            .iter()
            .any(|l| l.text.contains("cantilevered teak stairs up to 2F")
                || l.text.contains("Open floating teak steps"))
    );

    // Climb stairs to 2F
    runtime.run_turn("climb stairs").expect("climb stairs");
    assert_eq!(runtime.current_room_id().unwrap(), "studio");

    // Inspecting open stairwell down and spiral stairs up in studio
    let look_stairwell = runtime.run_turn("look stairwell").expect("look stairwell");
    assert!(look_stairwell.lines.iter().any(|l| {
        l.text
            .contains("looking down into the sunken emerald terrazzo")
    }));

    let look_spiral = runtime
        .run_turn("look spiral stairs")
        .expect("look spiral stairs");
    assert!(look_spiral.lines.iter().any(|l| {
        l.text
            .contains("matte-black steel spiral staircase corkscrewing")
    }));

    // Go up to 3F
    runtime.run_turn("go up").expect("go up to 3F");
    assert_eq!(runtime.current_room_id().unwrap(), "rooftop");

    // Go down to 2F
    runtime.run_turn("go down").expect("go down to 2F");
    assert_eq!(runtime.current_room_id().unwrap(), "studio");

    // Go down to 1F
    runtime.run_turn("go down").expect("go down to 1F");
    assert_eq!(runtime.current_room_id().unwrap(), "lounge");

    // Go down to B1
    runtime.run_turn("go down").expect("go down to B1");
    assert_eq!(runtime.current_room_id().unwrap(), "confessional");

    // Go up to 1F
    runtime.run_turn("go up").expect("go up to 1F");
    assert_eq!(runtime.current_room_id().unwrap(), "lounge");
}

#[test]
fn aera_autonomous_quest_navigation_flow() {
    use cinder_core::engine::actor_turn::{build_actor_turn, realize_actor_turn_action};
    use cinder_core::engine::dialogue::{
        ActorTurnActionDecision, ScriptedDialogueGenerator, render_actor_turn_decider_prompt,
    };
    use cinder_core::engine::events::TimestampedWorldEvent;
    use cinder_core::engine::reducer::apply_events;
    use std::sync::Arc;

    let pack = Arc::new(load_named_pack("aera", Some("en")).expect("aera loads"));
    let mut state = WorldState::new(&pack);
    let dialogue = Arc::new(ScriptedDialogueGenerator::new());
    let actor = pack.actor("aera").expect("aera defined");
    let rules = pack.movement_rules("aera");
    let mut noop_trace = |_role: &str, _topic: &str, _payload: serde_json::Value| Ok(());

    // --- STEP 1: Aera starts in Lounge with quest stage-morning-start ---
    assert_eq!(
        state.active_objective_stage_ids,
        vec!["stage-morning-start"]
    );
    assert_eq!(state.actor_room_id("aera", &actor.room_id), "lounge");

    let build1 = build_actor_turn(Arc::clone(&pack), &state, actor, &rules).expect("build step 1");
    let prompt1 = render_actor_turn_decider_prompt(&build1.request);

    // Verify Prompt Structure: Character Persona, Spatial Grounding, and Quest Guidance
    assert!(prompt1.contains("Kim Ae-ra"));
    assert!(prompt1.contains("Sunken Living Lounge"));
    assert!(prompt1.contains("You need your high-speed CFexpress card from the kitchen island"));
    // Lounge connects to entrance, which pathfinder selects as next hop to kitchen
    assert!(prompt1.contains("MOVE entrance"));

    // Realize Move to Front Entry
    let decision1 = ActorTurnActionDecision::MoveTo {
        room_id: "entrance".to_string(),
    };
    let events1 = realize_actor_turn_action(
        &pack,
        dialogue.as_ref(),
        &state,
        actor,
        &decision1,
        &build1.realization_context,
        &mut noop_trace,
    )
    .expect("realize step 1");
    let ts_events1: Vec<_> = events1
        .into_iter()
        .map(TimestampedWorldEvent::now)
        .collect();
    apply_events(&mut state, &pack, &ts_events1);
    assert_eq!(state.actor_room_id("aera", &actor.room_id), "entrance");

    // In Entrance, next hop to kitchen is Kitchen
    let build_entry =
        build_actor_turn(Arc::clone(&pack), &state, actor, &rules).expect("build entry");
    let prompt_entry = render_actor_turn_decider_prompt(&build_entry.request);
    assert!(prompt_entry.contains("MOVE kitchen"));

    let decision_entry = ActorTurnActionDecision::MoveTo {
        room_id: "kitchen".to_string(),
    };
    let events_entry = realize_actor_turn_action(
        &pack,
        dialogue.as_ref(),
        &state,
        actor,
        &decision_entry,
        &build_entry.realization_context,
        &mut noop_trace,
    )
    .expect("realize entry");
    let ts_events_entry: Vec<_> = events_entry
        .into_iter()
        .map(TimestampedWorldEvent::now)
        .collect();
    apply_events(&mut state, &pack, &ts_events_entry);

    // --- STEP 2: In Kitchen, stage advanced to stage-retrieve-card ---
    assert_eq!(state.actor_room_id("aera", &actor.room_id), "kitchen");
    assert_eq!(
        state.active_objective_stage_ids,
        vec!["stage-retrieve-card"]
    );

    let build2 = build_actor_turn(Arc::clone(&pack), &state, actor, &rules).expect("build step 2");
    let prompt2 = render_actor_turn_decider_prompt(&build2.request);
    assert!(prompt2.contains("Open Kitchen"));
    assert!(prompt2.contains("Inspect the stainless-steel island to pick up the CFexpress card"));

    // Realize Inspect Kitchen Island
    let decision2 = ActorTurnActionDecision::Command {
        command_id: "inspect_feature".to_string(),
        target_room_id: None,
        target_actor_id: None,
        feature_id: Some("kitchen-island".to_string()),
        consumable_id: None,
        context_label: None,
        freeform_text: None,
    };
    let events2 = realize_actor_turn_action(
        &pack,
        dialogue.as_ref(),
        &state,
        actor,
        &decision2,
        &build2.realization_context,
        &mut noop_trace,
    )
    .expect("realize step 2");
    let ts_events2: Vec<_> = events2
        .into_iter()
        .map(TimestampedWorldEvent::now)
        .collect();
    apply_events(&mut state, &pack, &ts_events2);

    // --- STEP 3: Stage advanced to stage-bring-to-studio. Target is 2F studio ---
    assert_eq!(
        state.active_objective_stage_ids,
        vec!["stage-bring-to-studio"]
    );

    // Multi-hop pathfinding: from Kitchen, next hop toward Studio is Front Entry
    let build3 = build_actor_turn(Arc::clone(&pack), &state, actor, &rules).expect("build step 3");
    let prompt3 = render_actor_turn_decider_prompt(&build3.request);
    assert!(prompt3.contains("Head through the living lounge and take the teak stairs"));
    assert!(prompt3.contains("MOVE entrance"));

    let decision3 = ActorTurnActionDecision::MoveTo {
        room_id: "entrance".to_string(),
    };
    let events3 = realize_actor_turn_action(
        &pack,
        dialogue.as_ref(),
        &state,
        actor,
        &decision3,
        &build3.realization_context,
        &mut noop_trace,
    )
    .expect("realize step 3");
    let ts_events3: Vec<_> = events3
        .into_iter()
        .map(TimestampedWorldEvent::now)
        .collect();
    apply_events(&mut state, &pack, &ts_events3);
    assert_eq!(state.actor_room_id("aera", &actor.room_id), "entrance");

    // From Front Entry, next hop toward Studio is Lounge
    let build_entry2 =
        build_actor_turn(Arc::clone(&pack), &state, actor, &rules).expect("build entry 2");
    let prompt_entry2 = render_actor_turn_decider_prompt(&build_entry2.request);
    assert!(prompt_entry2.contains("MOVE lounge"));

    let decision_entry2 = ActorTurnActionDecision::MoveTo {
        room_id: "lounge".to_string(),
    };
    let events_entry2 = realize_actor_turn_action(
        &pack,
        dialogue.as_ref(),
        &state,
        actor,
        &decision_entry2,
        &build_entry2.realization_context,
        &mut noop_trace,
    )
    .expect("realize entry 2");
    let ts_events_entry2: Vec<_> = events_entry2
        .into_iter()
        .map(TimestampedWorldEvent::now)
        .collect();
    apply_events(&mut state, &pack, &ts_events_entry2);
    assert_eq!(state.actor_room_id("aera", &actor.room_id), "lounge");

    // --- STEP 4: In Lounge, next hop toward Studio is Studio (via 2F stairs) ---
    let build4 = build_actor_turn(Arc::clone(&pack), &state, actor, &rules).expect("build step 4");
    let prompt4 = render_actor_turn_decider_prompt(&build4.request);
    assert!(prompt4.contains("MOVE studio"));

    let decision4 = ActorTurnActionDecision::MoveTo {
        room_id: "studio".to_string(),
    };
    let events4 = realize_actor_turn_action(
        &pack,
        dialogue.as_ref(),
        &state,
        actor,
        &decision4,
        &build4.realization_context,
        &mut noop_trace,
    )
    .expect("realize step 4");
    let ts_events4: Vec<_> = events4
        .into_iter()
        .map(TimestampedWorldEvent::now)
        .collect();
    apply_events(&mut state, &pack, &ts_events4);

    // --- STEP 5: Arrived at Studio on 2F. Stage advanced to stage-edit-at-studio ---
    assert_eq!(state.actor_room_id("aera", &actor.room_id), "studio");
    assert_eq!(
        state.active_objective_stage_ids,
        vec!["stage-edit-at-studio"]
    );

    let build5 = build_actor_turn(Arc::clone(&pack), &state, actor, &rules).expect("build step 5");
    let prompt5 = render_actor_turn_decider_prompt(&build5.request);
    assert!(prompt5.contains("Design Studio & Library"));
    assert!(prompt5.contains("Inspect the video editing station"));

    // Realize Inspect Editing Station
    let decision5 = ActorTurnActionDecision::Command {
        command_id: "inspect_feature".to_string(),
        target_room_id: None,
        target_actor_id: None,
        feature_id: Some("editing-station".to_string()),
        consumable_id: None,
        context_label: None,
        freeform_text: None,
    };
    let events5 = realize_actor_turn_action(
        &pack,
        dialogue.as_ref(),
        &state,
        actor,
        &decision5,
        &build5.realization_context,
        &mut noop_trace,
    )
    .expect("realize step 5");
    let ts_events5: Vec<_> = events5
        .into_iter()
        .map(TimestampedWorldEvent::now)
        .collect();
    apply_events(&mut state, &pack, &ts_events5);

    // Quest complete! All stages in quest completed.
    assert!(state.active_objective_stage_ids.is_empty());
}
