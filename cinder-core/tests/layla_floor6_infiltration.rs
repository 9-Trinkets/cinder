//! Integration tests for Floor 6 Infiltration:
//! - Handoff: Einar gives clandestine token, silver arrow pin, and 3 invisibility potions
//! - Stealth: Invisibility allows sneaking past cathedral guards
//! - Witch & Deduction: Finding Orva among gardeners (inspecting Flora's bronze leaf, Silvia's cord, Orva's silver arrow)
//! - Code Word: Speaking "The arrow always finds home" to Orva receives the spirit pill
//! - Spiritual Sight: Swallowing pill reveals trapped spirits and exposes the living Seer
//! - Boss Battle: Slaying the Seer, obtaining Bind Scroll, learning Bind Sigil
//! - Platform Activation: Linking Floor 6 to lower floors and reuniting with Astrid & Einar

use cinder_core::content::loader::load_named_pack;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::WorldState;

#[test]
fn test_floor6_handoff_and_stealth_to_witch() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "sanctuary_grand_terrace".to_string();
    state.clock.turn_number = 1;
    // Set quest stage to mq_find_the_witch as if player just arrived on Floor 6
    state.active_objective_stage_ids = vec!["mq_find_the_witch".to_string()];
    state.story_vars.set_unchecked("entered_floor6", "true");
    state.add_item("invisibility-potion");
    state.add_item("invisibility-potion");
    state.add_item("invisibility-potion");
    state.add_item("silver-arrow-pin");

    let scripted_dialogue = cinder_core::engine::dialogue::ScriptedDialogueGenerator::new()
        .with_reply(
            "orva",
            "The arrow always finds home! Einar is alive! Take this spirit pill. Swallow it before you enter the Seer's sanctum.",
        );

    let runtime =
        CinderRuntime::with_dialogue_generator(pack, state, std::sync::Arc::new(scripted_dialogue))
            .expect("runtime creates");

    // 1. Drink potion on the terrace
    let drink_out = runtime
        .run_turn("drink invisibility potion")
        .expect("drink potion");
    assert!(drink_out.text().contains("fade from sight"));
    assert!(runtime.export_state().unwrap().is_player_invisible());

    // 2. Sneak into crossroads with hostile guard
    runtime.run_turn("go south").expect("move to crossroads");
    assert_eq!(
        runtime.export_state().unwrap().current_room_id,
        "sanctuary_crossroads"
    );

    // Hostile guard in crossroads does NOT harm invisible player
    let hp_before = runtime.export_state().unwrap().actor_stat("player", "hp");
    runtime.run_tick().expect("tick in crossroads");
    let hp_after = runtime.export_state().unwrap().actor_stat("player", "hp");
    assert_eq!(
        hp_before, hp_after,
        "guard must not strike invisible player"
    );

    // 3. Move west into herbarium garden
    runtime.run_turn("go west").expect("move to garden");
    assert_eq!(
        runtime.export_state().unwrap().current_room_id,
        "herbarium_garden"
    );

    // 4. In the garden, halfling servants are present
    let look_garden = runtime.run_turn("look").expect("look around garden");
    assert!(
        look_garden.text().contains("Servant No. 4")
            && look_garden.text().contains("Servant No. 7"),
        "masked halfling servants must be listed in the room: {}",
        look_garden.text()
    );

    // Inspect Servant 4: trembles, hiding stolen bread
    let look_servant4 = runtime
        .run_turn("look at servant 4")
        .expect("look at servant 4");
    assert!(
        look_servant4.text().contains("bread") || look_servant4.text().contains("trembles"),
        "Servant 4 hides bread: {}",
        look_servant4.text()
    );

    // Inspect Servant 7: calm, harvesting seeds with cliff moss on straw shoes
    let look_servant7 = runtime
        .run_turn("look at servant 7")
        .expect("look at servant 7");
    assert!(
        look_servant7.text().contains("seeds") || look_servant7.text().contains("moss"),
        "Servant 7 has cliff moss: {}",
        look_servant7.text()
    );

    // Slip through vent flue out to cliff balcony
    runtime.run_turn("go out").expect("crawl through vent");
    assert_eq!(
        runtime.export_state().unwrap().current_room_id,
        "herbarium_cliff_balcony"
    );

    // Inspect Orva on the cliff balcony
    let look_orva = runtime.run_turn("look at orva").expect("look at orva");
    assert!(
        look_orva.text().contains("capsule")
            || look_orva.text().contains("arrow")
            || look_orva.text().contains("soul grass"),
        "Orva must be on cliff balcony, got: {}",
        look_orva.text()
    );

    // 5. Speak the secret code word to Orva
    let talk_out = runtime
        .run_turn("talk to orva The arrow always finds home")
        .expect("talk to orva");
    assert!(
        talk_out.text().to_lowercase().contains("arrow")
            || talk_out.text().to_lowercase().contains("orva"),
        "speech feedback received: {}",
        talk_out.text()
    );

    let state_after_code = runtime.export_state().unwrap();
    // Met witch should be recorded and spirit pill granted
    assert!(
        state_after_code.has_item("spirit-pill"),
        "Layla must receive spirit-pill after code word"
    );
}

#[test]
fn test_floor6_spiritual_sight_and_seer_deduction() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "mortuary_reanimation_sanctum".to_string();
    state.clock.turn_number = 1;
    state.add_item("spirit-pill");

    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime creates");

    // 1. Before swallowing pill: normal room description
    let look_normal = runtime.run_turn("look").expect("look normal");
    assert!(
        !look_normal.text().contains("Trapped human spirits"),
        "normal look should not show spiritual sight"
    );

    // 2. Swallow the spirit pill
    let pill_out = runtime
        .run_turn("swallow spirit pill")
        .expect("swallow pill");
    assert!(pill_out.text().contains("can now see spirits"));
    assert_eq!(
        runtime
            .export_state()
            .unwrap()
            .story_vars
            .get("spiritual_sight"),
        Some("true")
    );

    // 3. Look again: room description changes to reveal spirits & the living Seer
    let look_spiritual = runtime.run_turn("look").expect("look with spiritual sight");
    assert!(
        look_spiritual.text().contains("captive souls")
            || look_spiritual.text().contains("Spiritual sight"),
        "must trigger spiritual sight override: {}",
        look_spiritual.text()
    );
    assert!(
        look_spiritual.text().contains("Seer") || look_spiritual.text().contains("Valerius"),
        "must deduce the living person as the Seer: {}",
        look_spiritual.text()
    );
}

#[test]
fn test_floor6_defeat_seer_and_activate_platform() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "mortuary_reanimation_sanctum".to_string();
    state.clock.turn_number = 1;
    state.active_objective_stage_ids = vec!["mq_slay_the_seer".to_string()];
    state.story_vars.set_unchecked("spiritual_sight", "true");

    let seer_hp = state.actor_stat("the_seer", "hp");
    state
        .adjust_actor_stat(&pack, "the_seer", "hp", -(seer_hp - 1))
        .unwrap();
    state
        .adjust_actor_stat(&pack, "player", "strength", 100)
        .unwrap();

    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime creates");

    let attack_out = runtime.run_turn("attack seer").expect("attack seer");
    assert!(
        attack_out.text().contains("the_seer")
            || attack_out.text().contains("Seer")
            || attack_out.text().contains("crumbles")
            || attack_out.text().contains("dust"),
        "attack output: {}",
        attack_out.text()
    );

    let state_post_combat = runtime.export_state().unwrap();
    assert!(
        state_post_combat.actor_is_defeated("the_seer", "hp"),
        "Seer must be defeated"
    );
    assert_eq!(
        state_post_combat.story_vars.get("seer_defeated"),
        Some("true"),
        "seer_defeated story var must be set"
    );

    // Pick up bind scroll dropped by the Seer
    runtime.run_turn("take bind scroll").expect("take scroll");
    assert!(runtime.export_state().unwrap().has_item("bind-scroll"));

    // Read bind scroll to learn Bind Sigil
    let read_out = runtime.run_turn("read bind scroll").expect("read scroll");
    assert!(
        read_out.text().contains("Bind Sigil") || read_out.text().contains("mind"),
        "read scroll output: {}",
        read_out.text()
    );
    let s_after_read = runtime.export_state().unwrap();
    assert_eq!(s_after_read.story_vars.get("knows_bind"), Some("true"));
    assert!(s_after_read.has_item("bind-sigil"));

    // Move to platform
    runtime.run_turn("go south").expect("move to platform");
    assert_eq!(
        runtime.export_state().unwrap().current_room_id,
        "sanctuary_main_platform"
    );

    // Verify platform anchor armed
    let s_platform = runtime.export_state().unwrap();
    assert_eq!(
        s_platform.story_vars.get("anchor_floor6_platform"),
        Some("true"),
        "anchor_floor6_platform must be armed upon reaching platform"
    );
}

#[test]
fn test_floor6_reunion_and_ascent_to_floor7() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "mortuary_reanimation_sanctum".to_string();
    state.clock.turn_number = 1;
    state.active_objective_stage_ids = vec!["mq_activate_sanctuary_platform".to_string()];
    state.story_vars.set_unchecked("seer_defeated", "true");

    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime creates");

    // Move to platform to activate it
    let move_out = runtime.run_turn("go south").expect("move to platform");
    let s_platform = runtime.export_state().unwrap();
    assert_eq!(s_platform.current_room_id, "sanctuary_main_platform");
    assert_eq!(
        s_platform.story_vars.get("anchor_floor6_platform"),
        Some("true")
    );

    // Quest advanced to mq_prepare_the_ascent
    assert!(
        s_platform
            .active_objective_stage_ids
            .contains(&"mq_prepare_the_ascent".to_string()),
        "mq_prepare_the_ascent must be active: {:?}",
        s_platform.active_objective_stage_ids
    );

    // Astrid, Einar, and Orva relocated to platform for reunion
    assert_eq!(
        s_platform.actor_room_id("commander_astrid", ""),
        "sanctuary_main_platform"
    );
    assert_eq!(
        s_platform.actor_room_id("einar", ""),
        "sanctuary_main_platform"
    );
    assert_eq!(
        s_platform.actor_room_id("orva", ""),
        "sanctuary_main_platform"
    );

    // Companions on the platform do NOT follow player so they hold the platform
    assert!(!s_platform.follows_player("commander_astrid"));
    assert!(!s_platform.follows_player("einar"));

    // Look around platform: shows activated description with glowing stair
    let look_platform = runtime.run_turn("look").expect("look at platform");
    assert!(
        look_platform.text().contains("white light")
            || look_platform.text().contains("stair")
            || look_platform.text().contains("platform")
            || move_out.text().contains("platform"),
        "platform description: {}",
        look_platform.text()
    );

    // Inspect the glowing white stair
    let look_stair = runtime.run_turn("look at stair").expect("look at stair");
    assert!(
        look_stair.text().contains("staircase") || look_stair.text().contains("White Antechamber"),
        "stair inspect text: {}",
        look_stair.text()
    );

    // Ascend to Floor 7 (The White Antechamber)
    let ascend_out = runtime.run_turn("ascend").expect("ascend to floor 7");
    let s_antechamber = runtime.export_state().unwrap();
    assert_eq!(
        s_antechamber.current_room_id, "floor7_antechamber",
        "Layla ascends into floor7_antechamber"
    );
    assert_eq!(s_antechamber.story_vars.get("entered_floor7"), Some("true"));
    assert!(
        ascend_out.text().contains("Handler") || ascend_out.text().contains("white stone"),
        "Handler arrival line: {}",
        ascend_out.text()
    );

    // mq_prepare_the_ascent is completed
    assert!(
        s_antechamber
            .completed_stage_ids
            .contains("mq_prepare_the_ascent"),
        "mq_prepare_the_ascent must be completed"
    );

    // Companions remained stationed on Floor 6 (solo arrival in antechamber)
    assert_eq!(
        s_antechamber.actor_room_id("commander_astrid", ""),
        "sanctuary_main_platform",
        "Astrid must stay on sanctuary platform"
    );
    assert_eq!(
        s_antechamber.actor_room_id("einar", ""),
        "sanctuary_main_platform",
        "Einar must stay on sanctuary platform"
    );
}

#[test]
fn test_floor6_stealth_sentry_marcus() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "sanctuary_grand_terrace".to_string();
    state.clock.turn_number = 1;
    state.add_item("invisibility-potion");

    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime creates");

    // Drink potion before moving
    runtime
        .run_turn("drink invisibility potion")
        .expect("drink potion");
    assert!(runtime.export_state().unwrap().is_player_invisible());

    // Enter Crossroads with Sentry Marcus
    runtime.run_turn("go south").expect("enter crossroads");
    let s1 = runtime.export_state().unwrap();
    assert_eq!(s1.current_room_id, "sanctuary_crossroads");

    // Verify Sentry Marcus is present
    let look_crossroads = runtime.run_turn("look").expect("look at crossroads");
    assert!(
        look_crossroads.text().contains("Marcus") || look_crossroads.text().contains("Sentry"),
        "Sentry Marcus must be present in sanctuary_crossroads: {}",
        look_crossroads.text()
    );

    // Sentry Marcus does NOT damage invisible player during tick
    let hp_before = s1.actor_stat("player", "hp");
    runtime.run_tick().expect("tick with Marcus");
    let hp_after = runtime.export_state().unwrap().actor_stat("player", "hp");
    assert_eq!(hp_before, hp_after, "Marcus must ignore invisible player");
}

#[test]
fn test_floor6_secret_archive_side_quest() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "mortuary_reanimation_sanctum".to_string();
    state.clock.turn_number = 1;
    state.active_objective_stage_ids = vec!["sq_archive_of_the_disappeared".to_string()];

    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime creates");

    // Inspect the confidential mortuary ledger in the sanctum
    let look_ledger = runtime.run_turn("look at ledger").expect("look at ledger");
    assert!(
        look_ledger.text().contains("Siraj")
            || look_ledger.text().contains("Nur")
            || look_ledger.text().contains("harvested spirits"),
        "ledger inspect text: {}",
        look_ledger.text()
    );

    let s = runtime.export_state().unwrap();
    // Quest completed
    assert!(
        s.completed_stage_ids
            .contains("sq_archive_of_the_disappeared"),
        "sq_archive_of_the_disappeared must be completed"
    );
    assert_eq!(
        s.story_vars.get("discovered_monster_pipeline"),
        Some("true")
    );
}
