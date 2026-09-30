//! Integration tests for Floor 4 (The Commoners: 9-Room Village Triangle).

use cinder_core::content::loader::load_named_pack;
use cinder_core::content::types::{DropSpec, ItemStorageTarget};
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::{ActorStance, WorldState};

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
    "fortress_gate",
    "west_iron_walkway",
    "steam_prison_cage",
    "south_steam_gantry",
    "command_bastion",
    "east_sentry_walk",
    "teleport_platform",
];

#[test]
fn floor4_rooms_and_features_load_and_validate() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");

    assert_eq!(EXPECTED_FLOOR4_ROOMS.len(), 16);
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
    assert_eq!(to_village.requires_story_var.as_str(), "elemental_released");

    // Village square connects back to oh
    let village_square = pack.room("village_square").expect("village_square exists");
    assert!(
        village_square.exits.iter().any(|e| e.room_id == "oh"),
        "village_square connects up to oh"
    );

    // Village square connects along southern baseline of the triangle loop
    assert!(
        village_square
            .exits
            .iter()
            .any(|e| e.room_id == "village_sw_corner")
    );
    assert!(
        village_square
            .exits
            .iter()
            .any(|e| e.room_id == "village_south_1")
    );

    // Fortress apex connects through bulkhead behind fortress_gate_open story var
    let north_gate = pack
        .room("village_north_gate")
        .expect("village_north_gate exists");
    let to_fortress = north_gate
        .exits
        .iter()
        .find(|e| e.room_id == "fortress_gate")
        .expect("village_north_gate must connect to fortress_gate");
    assert_eq!(
        to_fortress.requires_story_var.as_str(),
        "fortress_gate_open"
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
    assert_eq!(
        map.rooms.len(),
        16,
        "Floor 4 map must have exactly 16 rooms"
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
        outcome.text().contains("civilian life signs")
            || outcome.text().contains("people live down here"),
        "Handler commentary should remark on civilians: {}",
        outcome.text()
    );
    assert!(outcome.text().contains("Handler:"));

    // Second movement into the village does not replay the descent line
    let _ = runtime.run_turn("go up").expect("turn runs");
    let outcome2 = runtime.run_turn("go down").expect("turn runs");
    assert!(
        !outcome2.text().contains("civilian life signs"),
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
    assert_eq!(
        sentries.len(),
        2,
        "There must be two sentries at village_north_gate"
    );
    for sentry in &sentries {
        assert!(sentry.attackable);
        assert!(
            !sentry.initial_hostile,
            "Sentries should not attack on sight"
        );
    }

    // Placed fortress actors
    let zayd = pack.actor("zayd").expect("zayd exists");
    assert_eq!(zayd.room_id, "steam_prison_cage");

    let warden = pack.actor("garrison_warden").expect("warden exists");
    assert_eq!(warden.room_id, "steam_prison_cage");

    let malik = pack.actor("captain_malik").expect("malik exists");
    assert_eq!(malik.room_id, "command_bastion");

    let harun = pack.actor("priest_harun").expect("harun exists");
    assert_eq!(harun.room_id, "command_bastion");

    // Sakhra is placed in village_square for the awakening storyline
    let sakhra = pack.actor("sakhra").expect("sakhra exists");
    assert_eq!(
        sakhra.room_id, "village_square",
        "sakhra should be placed in village_square"
    );
}

#[test]
fn floor4_fortress_loop_and_platform_navigation_resolves() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "fortress_gate".to_string();
    state.story_vars.set_unchecked("fortress_gate_open", "true");
    state.story_vars.set_unchecked("malik_defeated", "true");
    let dialogue =
        std::sync::Arc::new(cinder_core::engine::dialogue::ScriptedDialogueGenerator::new());
    let runtime =
        cinder_core::engine::runtime::CinderRuntime::with_dialogue_generator(pack, state, dialogue)
            .expect("runtime creates");

    // Walk fortress perimeter loop:
    // 1. fortress_gate -> southwest -> west_iron_walkway
    let _ = runtime.run_turn("southwest").expect("turn runs");
    assert_eq!(runtime.current_room_id().unwrap(), "west_iron_walkway");

    // 2. west_iron_walkway -> southwest -> steam_prison_cage (Zayd)
    let _ = runtime.run_turn("southwest").expect("turn runs");
    assert_eq!(runtime.current_room_id().unwrap(), "steam_prison_cage");

    // 3. steam_prison_cage -> east -> south_steam_gantry
    let _ = runtime.run_turn("east").expect("turn runs");
    assert_eq!(runtime.current_room_id().unwrap(), "south_steam_gantry");

    // 4. south_steam_gantry -> east -> command_bastion (Malik)
    let _ = runtime.run_turn("east").expect("turn runs");
    assert_eq!(runtime.current_room_id().unwrap(), "command_bastion");

    // Enter central teleport platform from command_bastion
    let _ = runtime.run_turn("courtyard").expect("turn runs");
    assert_eq!(runtime.current_room_id().unwrap(), "teleport_platform");

    // Return to command_bastion
    let _ = runtime.run_turn("bastion").expect("turn runs");
    assert_eq!(runtime.current_room_id().unwrap(), "command_bastion");

    // 5. command_bastion -> north -> east_sentry_walk
    let _ = runtime.run_turn("north").expect("turn runs");
    assert_eq!(runtime.current_room_id().unwrap(), "east_sentry_walk");

    // 6. east_sentry_walk -> northwest -> fortress_gate (Loop completed)
    let _ = runtime.run_turn("northwest").expect("turn runs");
    assert_eq!(runtime.current_room_id().unwrap(), "fortress_gate");

    // Step north out through the bulkhead back to the village
    let _ = runtime.run_turn("north").expect("turn runs");
    assert_eq!(runtime.current_room_id().unwrap(), "village_north_gate");

    // Step south back through the bulkhead into fortress_gate
    let _ = runtime.run_turn("south").expect("turn runs");
    assert_eq!(runtime.current_room_id().unwrap(), "fortress_gate");
}

#[test]
fn floor4_diversion_opens_fortress_gate() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");

    // Check Wash Basin Terrace has both features
    let wash_basin = pack.room("village_west_1").expect("village_west_1 exists");
    assert!(
        wash_basin
            .features
            .iter()
            .any(|f| f.id == "village_west_1-valve"),
        "village_west_1 must have valve feature"
    );
    let valve_feature = wash_basin
        .features
        .iter()
        .find(|f| f.id == "village_west_1-valve")
        .unwrap();
    assert!(valve_feature.aliases.contains(&"valve".to_string()));
    assert!(valve_feature.aliases.contains(&"wheel".to_string()));

    // Check Tariq prompt context contains the diversion clue
    let tariq = pack.actor("tariq").expect("tariq exists");
    assert!(
        tariq
            .prompt_context
            .subtext_notes
            .iter()
            .any(|note| note.contains("overpressure valve")),
        "Tariq subtext should mention overpressure valve"
    );

    let mut state = WorldState::new(&pack);
    state.current_room_id = "village_north_gate".to_string();

    let dialogue =
        std::sync::Arc::new(cinder_core::engine::dialogue::ScriptedDialogueGenerator::new());
    let runtime =
        cinder_core::engine::runtime::CinderRuntime::with_dialogue_generator(pack, state, dialogue)
            .expect("runtime creates");

    // 1. Bulkhead is initially locked
    let _ = runtime.run_turn("south").expect("turn runs");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "village_north_gate",
        "Should not pass through locked bulkhead"
    );

    // 2. Turning valve in the wrong room fails
    let fail_turn = runtime.run_turn("turn valve").expect("turn runs");
    assert_eq!(runtime.current_room_id().unwrap(), "village_north_gate");
    assert!(
        fail_turn.text().contains("Wash Basin Terrace"),
        "Should indicate action can only be done at Wash Basin Terrace: {}",
        fail_turn.text()
    );

    // 3. Navigate down the west edge: North Gate -> Clockmaker (village_west_2) -> Wash Basin (village_west_1)
    let _ = runtime.run_turn("southwest").expect("turn runs");
    assert_eq!(runtime.current_room_id().unwrap(), "village_west_2");

    let _ = runtime.run_turn("southwest").expect("turn runs");
    assert_eq!(runtime.current_room_id().unwrap(), "village_west_1");

    // 4. Turn the valve at Wash Basin Terrace
    let valve_outcome = runtime.run_turn("turn valve").expect("turn runs");
    assert!(
        valve_outcome.text().contains("valve screeches open")
            || valve_outcome.text().contains("blast of steam"),
        "Outcome should describe steam blast: {}",
        valve_outcome.text()
    );
    assert!(
        valve_outcome.text().contains("bulkhead unlocks")
            || valve_outcome.text().contains("lock-pins clunk free"),
        "Outcome should describe bulkhead unlocking: {}",
        valve_outcome.text()
    );

    // Verify story variable is set
    let exported = runtime.export_state().unwrap();
    assert_eq!(exported.story_vars.get("fortress_gate_open"), Some("true"));

    // 5. Navigate back to North Gate: village_west_1 -> northeast -> village_west_2 -> northeast -> village_north_gate
    let _ = runtime.run_turn("northeast").expect("turn runs");
    assert_eq!(runtime.current_room_id().unwrap(), "village_west_2");
    let _ = runtime.run_turn("northeast").expect("turn runs");
    assert_eq!(runtime.current_room_id().unwrap(), "village_north_gate");

    // 6. Bulkhead is now unlocked! Enter the fortress
    let enter_fortress = runtime.run_turn("south").expect("turn runs");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "fortress_gate",
        "Should enter fortress_gate after diversion: {}",
        enter_fortress.text()
    );
}

#[test]
fn floor4_cardinal_and_diagonal_navigation_resolves_cleanly() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");

    // village_north_gate exit resolution
    let vng = pack
        .room("village_north_gate")
        .expect("village_north_gate exists");
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
    let runtime = cinder_core::engine::runtime::CinderRuntime::with_dialogue_generator(
        pack.clone(),
        state,
        dialogue,
    )
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

#[test]
fn floor4_quests_panel_hidden_before_floor4_and_revealed_on_floor4() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");

    // Floors 1-3 rooms should NOT reveal quests
    assert!(!pack.quests_revealed_for_room("deep_forest_start"));
    assert!(!pack.quests_revealed_for_room("deep_forest_2"));
    assert!(!pack.quests_revealed_for_room("cave_start"));
    assert!(!pack.quests_revealed_for_room("oh"));

    // Floor 4 rooms MUST reveal quests
    assert!(pack.quests_revealed_for_room("village_square"));
    assert!(pack.quests_revealed_for_room("village_south_1"));
    assert!(pack.quests_revealed_for_room("village_north_gate"));
    assert!(pack.quests_revealed_for_room("village_west_2"));
}

#[test]
fn floor4_quests_activation_via_speech() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "village_square".to_string();

    let scripted_dialogue = cinder_core::engine::dialogue::ScriptedDialogueGenerator::new()
        .with_reply(
            "elder_rashid",
            "Please, Layla... we offered the boy Zayd as a sacrifice to the garrison. You must rescue him before the temple transport arrives!",
        )
        .with_reply(
            "tariq",
            "Commander Malik keeps the Teleportation Scroll locked in his brass safe. It is the only way to reach Floor 5.",
        );

    let runtime = cinder_core::engine::runtime::CinderRuntime::with_dialogue_generator(
        pack.clone(),
        state,
        std::sync::Arc::new(scripted_dialogue),
    )
    .expect("runtime creates");

    // Initially on Floor 4, no quests are yet visible in summaries
    let initial_objectives = runtime.current_objective_summaries().unwrap();
    assert!(
        initial_objectives.is_empty(),
        "Quests should be latent until NPC tells Layla"
    );

    // 1. Move to Elder Rashid's room (village_south_1) and talk
    let _ = runtime.run_turn("east").expect("move to elder");
    assert_eq!(runtime.current_room_id().unwrap(), "village_south_1");

    let outcome = runtime.run_turn("talk to rashid").expect("talk to rashid");
    assert!(outcome.text().contains("Zayd"));

    // Now Side Quest "Save the Boy Zayd" should be active!
    let objectives_after_rashid = runtime.current_objective_summaries().unwrap();
    assert_eq!(objectives_after_rashid.len(), 1);
    assert_eq!(
        objectives_after_rashid[0].quest_id.as_deref(),
        Some("save_zayd")
    );
    assert_eq!(
        objectives_after_rashid[0].quest_title.as_deref(),
        Some("Save the Boy Zayd")
    );
    assert_eq!(
        objectives_after_rashid[0].quest_kind.as_deref(),
        Some("side")
    );

    // 2. Move along to Tariq's workshop (village_west_2):
    // village_south_1 -> west -> village_square -> west -> village_sw_corner -> northeast -> village_west_1 -> northeast -> village_west_2
    let _ = runtime.run_turn("west").expect("to square");
    let _ = runtime.run_turn("west").expect("to sw");
    let _ = runtime.run_turn("northeast").expect("to west_1");
    let _ = runtime.run_turn("northeast").expect("to west_2");
    assert_eq!(runtime.current_room_id().unwrap(), "village_west_2");

    let outcome = runtime.run_turn("talk to tariq").expect("talk to tariq");
    assert!(outcome.text().contains("Teleportation Scroll"));

    // Now BOTH Main Quest and Side Quest should be active!
    let objectives_after_tariq = runtime.current_objective_summaries().unwrap();
    assert_eq!(objectives_after_tariq.len(), 2);

    let main_quest = objectives_after_tariq
        .iter()
        .find(|o| o.quest_kind.as_deref() == Some("main"))
        .expect("main quest must be active");
    assert_eq!(main_quest.quest_id.as_deref(), Some("teleport_scroll"));
    assert_eq!(
        main_quest.quest_title.as_deref(),
        Some("The Teleportation Scroll")
    );

    let side_quest = objectives_after_tariq
        .iter()
        .find(|o| o.quest_kind.as_deref() == Some("side"))
        .expect("side quest must still be active");
    assert_eq!(side_quest.quest_id.as_deref(), Some("save_zayd"));
}

#[test]
fn floor4_tick_runs_without_soft_error() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "village_square".to_string();
    state.turn_number = 10;

    let scripted_dialogue = cinder_core::engine::dialogue::ScriptedDialogueGenerator::new();
    let runtime = cinder_core::engine::runtime::CinderRuntime::with_dialogue_generator(
        pack.clone(),
        state,
        std::sync::Arc::new(scripted_dialogue),
    )
    .expect("runtime creates");

    let outcome = runtime.run_tick().expect("tick should succeed");
    println!("Tick outcome text: {}", outcome.text());
    assert!(
        !outcome.text().contains("goes still, listening to the dark"),
        "Tick should not produce soft error: got '{}'",
        outcome.text()
    );
}

#[test]
fn floor4_large_state_tick_runs_without_soft_error() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "village_square".to_string();
    state.turn_number = 50;

    // Simulate accumulated state from playing floors 1 to 4:
    // 30+ observation notes, actor memories, quest stages, etc.
    let mut notes = Vec::new();
    for i in 0..100 {
        notes.push(format!(
            "Observed event note #{i}: Layla and companions explored the steampunk corridor and listened to steam vents echoing in the distance."
        ));
    }
    state
        .actor_recent_observation_notes
        .insert("layla".to_string(), notes);
    let serialized_len = serde_json::to_string(&state).unwrap().len();
    assert!(
        serialized_len > 25000,
        "Simulated state should exceed 25,000 characters to test workflow message size limit: got {serialized_len}"
    );

    let scripted_dialogue = cinder_core::engine::dialogue::ScriptedDialogueGenerator::new();
    let runtime = cinder_core::engine::runtime::CinderRuntime::with_dialogue_generator(
        pack.clone(),
        state,
        std::sync::Arc::new(scripted_dialogue),
    )
    .expect("runtime creates");

    let outcome = runtime.run_tick().expect("tick should succeed");
    assert!(
        !outcome.text().contains("goes still, listening to the dark"),
        "Tick should not produce soft error on large state: got '{}'",
        outcome.text()
    );
}

#[test]
fn multi_floor_descent_isolates_transcripts_and_builds_summaries() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");

    let dialogue = std::sync::Arc::new(
        cinder_core::engine::dialogue::ScriptedDialogueGenerator::new()
            .with_transition_commentary_lines(
                "d1c1",
                vec![
                    "Layla conquered The Cave.".to_string(),
                    "Prepare for Deep Forest.".to_string(),
                ],
            )
            .with_transition_commentary_lines(
                "oan",
                vec![
                    "Layla conquered Deep Forest.".to_string(),
                    "Prepare for Outer Ring.".to_string(),
                ],
            ),
    );
    let mut state = WorldState::new(&pack);
    state.current_room_id = "r5c5".to_string();
    state.story_vars.set_unchecked("shaman_defeated", "true");

    let runtime = cinder_core::engine::runtime::CinderRuntime::with_dialogue_generator(
        pack.clone(),
        state,
        dialogue.clone(),
    )
    .expect("runtime creates");

    // Floor 1 lines
    let _ = runtime.push_transcript_line("Layla drew a chalk circle on the limestone cave floor.");
    let _ = runtime.push_transcript_line("Layla tamed a goblin in the cave.");

    // Descent 1: Floor 1 -> Floor 2 (d1c1)
    let outcome1 = runtime.run_turn("down").expect("descend to floor 2");
    assert_eq!(runtime.current_room_id().unwrap(), "d1c1");
    assert!(outcome1.text().contains("Layla conquered The Cave"));

    // Check request 1
    {
        let reqs = dialogue.captured_transition_requests();
        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0].completed_area_name, "The Cave");
        assert_eq!(reqs[0].destination_area_name, "Deep Forest");
        assert!(reqs[0].previous_area_summaries.is_empty());
        assert!(
            reqs[0]
                .recent_transcript
                .iter()
                .any(|l| l.contains("chalk circle"))
        );
    }

    let state1 = runtime.export_state().unwrap();
    assert!(state1.transition_summaries.contains_key("upper-works"));
    let f1_summary = &state1.transition_summaries["upper-works"];
    assert_eq!(f1_summary.area_name, "The Cave");
    assert_eq!(f1_summary.summary_text, "Layla conquered The Cave.");

    // Floor 2: Move to d8c5 (exit to Floor 3)
    let mut state_floor2 = runtime.export_state().unwrap();
    state_floor2.current_room_id = "d8c5".to_string();
    state_floor2
        .story_vars
        .set_unchecked("elf_king_defeated", "true");

    let runtime2 = cinder_core::engine::runtime::CinderRuntime::with_dialogue_generator(
        pack.clone(),
        state_floor2,
        dialogue.clone(),
    )
    .expect("runtime creates");

    // Generate Floor 2-specific lines
    let _ =
        runtime2.push_transcript_line("Layla navigated bioluminescent mushrooms and mossy boughs.");
    let _ = runtime2.push_transcript_line("Layla struck down the Corrupted Treant.");

    // Descent 2: Floor 2 -> Floor 3 (oan)
    let outcome2 = runtime2.run_turn("down").expect("descend to floor 3");
    assert_eq!(runtime2.current_room_id().unwrap(), "oan");
    assert!(outcome2.text().contains("Layla conquered Deep Forest"));

    // Check request 2
    {
        let reqs = dialogue.captured_transition_requests();
        assert_eq!(reqs.len(), 2);
        let req2 = &reqs[1];
        assert_eq!(req2.completed_area_name, "Deep Forest");
        assert_eq!(req2.destination_area_name, "Outer Ring");

        // Previous floor milestones MUST contain Floor 1 summary
        assert_eq!(req2.previous_area_summaries.len(), 1);
        assert!(req2.previous_area_summaries[0].contains("The Cave"));
        assert!(req2.previous_area_summaries[0].contains("Layla conquered The Cave."));

        // Recent transcript MUST contain Floor 2 events and MUST NOT contain Floor 1 events!
        assert!(
            req2.recent_transcript
                .iter()
                .any(|l| l.contains("bioluminescent mushrooms"))
        );
        assert!(
            req2.recent_transcript
                .iter()
                .any(|l| l.contains("Corrupted Treant"))
        );
        assert!(
            !req2
                .recent_transcript
                .iter()
                .any(|l| l.contains("chalk circle")),
            "Floor 2 transcript slice must NOT contain Floor 1 chalk circle!"
        );
        assert!(
            !req2
                .recent_transcript
                .iter()
                .any(|l| l.contains("tamed a goblin")),
            "Floor 2 transcript slice must NOT contain Floor 1 goblin!"
        );
    }

    let state2 = runtime2.export_state().unwrap();
    assert!(state2.transition_summaries.contains_key("upper-works"));
    assert!(state2.transition_summaries.contains_key("deep-forest"));
    let f2_summary = &state2.transition_summaries["deep-forest"];
    assert_eq!(f2_summary.area_name, "Deep Forest");
    assert_eq!(f2_summary.summary_text, "Layla conquered Deep Forest.");
}

#[test]
fn floor4_zayd_rescue_and_village_escort() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = WorldState::new(&pack);

    // 1. Initial state checks: Zayd must NOT carry lantern in inventory
    let zayd = pack.actor("zayd").expect("zayd exists");
    assert!(
        !zayd.initial_inventory.contains_key("zayd-lantern"),
        "Zayd must not carry zayd-lantern in initial inventory"
    );
    assert!(
        !state.actor_has_item("player", "zayd-lantern"),
        "Player must not start with zayd-lantern"
    );

    // Start in village_south_1 to talk to Elder Rashid and activate sq_save_zayd
    state.current_room_id = "village_south_1".to_string();

    let scripted_dialogue = cinder_core::engine::dialogue::ScriptedDialogueGenerator::new()
        .with_reply(
            "elder_rashid",
            "Please, Layla... we offered the boy Zayd as a sacrifice to the garrison. You must rescue him from the suspended steam cage!",
        );
    let dialogue = std::sync::Arc::new(scripted_dialogue);

    let runtime = cinder_core::engine::runtime::CinderRuntime::with_dialogue_generator(
        pack.clone(),
        state,
        dialogue.clone(),
    )
    .expect("runtime creates");

    // Talk to Elder Rashid -> triggers sq_save_zayd
    let talk_outcome = runtime.run_turn("talk to rashid").expect("talk to rashid");
    assert!(talk_outcome.text().contains("Zayd"));

    let objectives = runtime.current_objective_summaries().unwrap();
    let save_zayd_quest = objectives
        .iter()
        .find(|o| o.quest_id.as_deref() == Some("save_zayd"))
        .expect("save_zayd quest active");
    assert_eq!(save_zayd_quest.stage_id, "sq_save_zayd");

    // 2. Open fortress bulkhead gate and grant player the cage key
    let mut state = runtime.export_state().unwrap();
    state.story_vars.set_unchecked("fortress_gate_open", "true");
    state.add_item("iron-cage-key");
    state.actor_add_item("player", "iron-cage-key");
    state.current_room_id = "steam_prison_cage".to_string();

    let runtime = cinder_core::engine::runtime::CinderRuntime::with_dialogue_generator(
        pack.clone(),
        state,
        dialogue.clone(),
    )
    .expect("runtime creates");

    // Before unlock: Zayd is in the cage, does not follow, does not have lantern
    let state_before = runtime.export_state().unwrap();
    assert!(!state_before.relationship("zayd").follows_player);
    assert!(!state_before.actor_has_item("player", "zayd-lantern"));
    assert!(!state_before.actor_has_item("zayd", "zayd-lantern"));

    // 3. Unlock cage
    let unlock_outcome = runtime.run_turn("unlock cage").expect("unlock cage");
    let unlock_text = unlock_outcome.text();
    assert!(
        unlock_text.contains("slide the heavy iron key") || unlock_text.contains("tumblers turn"),
        "Unlock description missing in: {unlock_text}"
    );
    assert!(
        !unlock_text.to_lowercase().contains("lantern"),
        "Cage unlock narrative should NOT mention lantern prior to rescue: {unlock_text}"
    );
    assert!(
        unlock_text.contains("Zayd: You... you unlocked it! You're really here to save me?"),
        "Zayd spoken dialogue line missing or misformatted in: {unlock_text}"
    );
    assert!(
        unlock_text.contains("Handler: Layla. That offering was cleared on the manifest. Let the offering ship. That's the job."),
        "Handler offering warning missing in: {unlock_text}"
    );

    // Verify channel line kind for handler comms
    let channel_line = unlock_outcome
        .lines
        .iter()
        .find(|l| l.kind == cinder_core::engine::narrative::NarrativeLineKind::Channel)
        .expect("Handler warning must be delivered as NarrativeLineKind::Channel");
    assert!(channel_line.text.contains("Handler:"));

    // Verify Zayd is now allied and follows player
    let state_after_unlock = runtime.export_state().unwrap();
    assert_eq!(
        state_after_unlock.story_vars.get("zayd_rescued"),
        Some("true")
    );
    assert!(
        state_after_unlock.relationship("zayd").follows_player,
        "Zayd must now follow the player"
    );
    assert_eq!(
        state_after_unlock.stance("zayd"),
        cinder_core::engine::state::ActorStance::Allied
    );
    assert!(
        !state_after_unlock.actor_has_item("player", "zayd-lantern"),
        "Player must not receive lantern prematurely while in cage room"
    );

    // Verify unlock cage is no longer available after unlocking
    let unlock_action = pack.actions.iter().find(|a| a.id == "unlock_cage").unwrap();
    assert!(
        !cinder_core::engine::turn_policies::action_is_available(
            &pack,
            &state_after_unlock,
            unlock_action,
            "steam_prison_cage"
        ),
        "unlock cage must not be available once zayd is rescued"
    );

    // Verify quest advanced to escort stage sq_return_zayd
    let objectives_escort = runtime.current_objective_summaries().unwrap();
    let escort_quest = objectives_escort
        .iter()
        .find(|o| o.quest_id.as_deref() == Some("save_zayd"))
        .expect("escort quest active");
    assert_eq!(escort_quest.stage_id, "sq_return_zayd");

    // 4. Escort Zayd back through the fortress and village loop
    // steam_prison_cage -> ne -> west_iron_walkway -> ne -> fortress_gate -> north -> village_north_gate -> sw -> village_west_2 -> sw -> village_west_1 -> sw -> village_sw_corner
    let _ = runtime.run_turn("northeast").expect("to west_iron_walkway");
    assert_eq!(runtime.current_room_id().unwrap(), "west_iron_walkway");
    let s = runtime.export_state().unwrap();
    assert_eq!(s.actor_current_room_id(&pack, "zayd"), "west_iron_walkway");

    let _ = runtime.run_turn("northeast").expect("to fortress_gate");
    assert_eq!(runtime.current_room_id().unwrap(), "fortress_gate");
    let s = runtime.export_state().unwrap();
    assert_eq!(s.actor_current_room_id(&pack, "zayd"), "fortress_gate");

    let _ = runtime.run_turn("north").expect("to village_north_gate");
    assert_eq!(runtime.current_room_id().unwrap(), "village_north_gate");
    let s = runtime.export_state().unwrap();
    assert_eq!(s.actor_current_room_id(&pack, "zayd"), "village_north_gate");

    let _ = runtime.run_turn("southwest").expect("to village_west_2");
    assert_eq!(runtime.current_room_id().unwrap(), "village_west_2");
    let s = runtime.export_state().unwrap();
    assert_eq!(s.actor_current_room_id(&pack, "zayd"), "village_west_2");

    let _ = runtime.run_turn("southwest").expect("to village_west_1");
    assert_eq!(runtime.current_room_id().unwrap(), "village_west_1");

    let _ = runtime.run_turn("southwest").expect("to village_sw_corner");
    assert_eq!(runtime.current_room_id().unwrap(), "village_sw_corner");

    // Still does not have lantern right before entering village square
    let s_before_square = runtime.export_state().unwrap();
    assert!(!s_before_square.actor_has_item("player", "zayd-lantern"));
    assert!(s_before_square.relationship("zayd").follows_player);

    // 5. Enter village_square -> triggers safe arrival hook
    let return_outcome = runtime.run_turn("east").expect("enter village_square");
    assert_eq!(runtime.current_room_id().unwrap(), "village_square");
    let return_text = return_outcome.text();

    // Verify safe arrival narrative
    assert!(
        return_text.contains("Yasmin rushes forward"),
        "Return safe narrative missing in: {return_text}"
    );
    assert!(
        return_text.contains(
            "Zayd: I... I'm sorry I don't have anything valuable to give you as a reward"
        ),
        "Zayd lantern speech missing or misformatted in: {return_text}"
    );

    // Verify lantern received
    assert!(
        return_text.contains("lantern") || return_text.contains("zayd-lantern"),
        "Item acquisition announcement missing in: {return_text}"
    );

    let final_state = runtime.export_state().unwrap();
    assert_eq!(final_state.story_vars.get("zayd_safe"), Some("true"));
    assert!(
        !final_state.relationship("zayd").follows_player,
        "Zayd must stop following once safe in the village"
    );
    assert!(
        final_state.actor_has_item("player", "zayd-lantern")
            || final_state.has_item("zayd-lantern"),
        "Player must now possess zayd-lantern"
    );

    // Verify quest completion
    let final_objectives = runtime.current_objective_summaries().unwrap();
    assert!(
        !final_objectives
            .iter()
            .any(|o| o.quest_id.as_deref() == Some("save_zayd")),
        "save_zayd quest must be completed upon safe return"
    );
}

#[test]
fn floor4_alternative_gate_entry_via_guard_key() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    assert_eq!(
        pack.actor("garrison_guard")
            .unwrap()
            .initial_inventory
            .get("fortress-gate-key"),
        Some(&DropSpec::Always(1))
    );

    let mut state = WorldState::new(&pack);
    state.current_room_id = "village_north_gate".to_string();

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");

    // 1. Bulkhead is initially locked
    let blocked = runtime.run_turn("go south").expect("turn runs");
    assert_ne!(runtime.current_room_id().unwrap(), "fortress_gate");
    assert!(
        blocked.text().contains("cannot go")
            || blocked.text().contains("route sheet")
            || blocked.text().contains("can't go")
    );

    // 2. Defeat garrison_guard -> drops fortress-gate-key
    let mut s = runtime.export_state().unwrap();
    s.add_item_to_storage(
        "fortress-gate-key",
        ItemStorageTarget::CurrentRoom,
        "village_north_gate",
    );
    let runtime = CinderRuntime::from_state(pack.clone(), s, false).expect("runtime creates");

    // 3. Take key
    runtime
        .run_turn("take fortress gate key")
        .expect("take key");
    let s = runtime.export_state().unwrap();
    assert!(s.has_item("fortress-gate-key"));

    // 4. Unlock gate with key
    let unlock_out = runtime.run_turn("unlock gate").expect("unlock gate");
    assert!(
        unlock_out
            .text()
            .contains("heavy brass key into the bulkhead lock")
    );
    let s = runtime.export_state().unwrap();
    assert_eq!(s.story_vars.get("fortress_gate_open"), Some("true"));
    assert!(!s.has_item("fortress-gate-key"), "key consumed");

    // 5. Bulkhead is now unlocked, player can enter fortress_gate
    runtime.run_turn("go south").expect("enter fortress");
    assert_eq!(runtime.current_room_id().unwrap(), "fortress_gate");
}

#[test]
fn charming_the_fire_elemental_opens_the_floor4_exit() {
    use cinder_core::content::types::ItemStorageTarget as Storage;
    use cinder_core::engine::events::{TimestampedWorldEvent, WorldEvent};
    use cinder_core::engine::reducer::apply_events;

    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = WorldState::new(&pack);

    // The elemental is stationed in oas; the descent lives in its neighbour oh.
    assert_eq!(pack.actor("fire-elemental").unwrap().room_id, "oas");

    // Charm the elemental. The surround gate is a resistance check on
    // intelligence against the elemental's level, so level Layla up the way a
    // real playthrough would have done by the time she reaches Floor 3.
    state.current_room_id = "oas".to_string();
    let player_id = pack.settings.combat.player_actor_id.clone();
    state.actor_level.insert(player_id.clone(), 12);

    // Ring the elemental's room: a sigil in every neighbouring room. The final
    // stroke — placing one in oas itself — is what closes the circle, so oas is
    // deliberately left empty here and filled by the ItemAcquired event below.
    for room_id in ["o6", "o7", "oh"] {
        state.add_item_to_storage("charm-sigil", Storage::CurrentRoom, room_id);
    }

    // Placing that final sigil fires the surround hook.
    apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::ItemAcquired {
            item_id: "charm-sigil".to_string(),
            storage: Storage::CurrentRoom,
        })],
    );

    // The charm converts the boss and opens the descent, exactly as a defeat does.
    assert_eq!(state.stance("fire-elemental"), ActorStance::Allied);
    assert_eq!(
        state.story_vars.get("elemental_released"),
        Some("true"),
        "charming the elemental must release it and open the exit, not just defeat it"
    );

    // And the exit is genuinely passable: go down from oh to the village.
    state.current_room_id = "oh".to_string();
    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");
    runtime.run_turn("go down").expect("descend after charm");
    assert_eq!(runtime.current_room_id().unwrap(), "village_square");
}
