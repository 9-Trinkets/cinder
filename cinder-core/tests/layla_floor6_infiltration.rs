//! Integration tests for Floor 6 Infiltration:
//! - Handoff: Einar gives clandestine token, silver arrow pin, and 3 invisibility potions
//! - Stealth: Invisibility allows sneaking past cathedral guards
//! - Witch: Meeting Selena in herb garden, speaking code word "The arrow always finds home", receiving spirit pill
//! - Spiritual Sight: Swallowing pill reveals trapped spirits and exposes the living Seer
//! - Boss Battle: Slaying the Seer, obtaining Revival Scroll, learning Revival Sigil
//! - Platform Activation: Linking Floor 6 to lower floors and reuniting with Astrid & Einar

use cinder_core::content::loader::load_named_pack;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::WorldState;

#[test]
fn test_floor6_handoff_and_stealth_to_witch() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "sanctuary_grand_terrace".to_string();
    state.turn_number = 1;
    // Set quest stage to mq_find_the_witch as if player just arrived on Floor 6
    state.active_objective_stage_ids = vec!["mq_find_the_witch".to_string()];
    state.story_vars.set_unchecked("entered_floor6", "true");
    state.add_item("invisibility-potion");
    state.add_item("invisibility-potion");
    state.add_item("invisibility-potion");
    state.add_item("silver-arrow-pin");

    let scripted_dialogue = cinder_core::engine::dialogue::ScriptedDialogueGenerator::new()
        .with_reply(
            "selena",
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
    runtime.run_turn("go north").expect("move to crossroads");
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

    // 3. Move to herbarium portal, then into herb garden
    runtime.run_turn("go southwest").expect("move to portal");
    runtime.run_turn("go south").expect("move to garden");
    assert_eq!(
        runtime.export_state().unwrap().current_room_id,
        "herbarium_garden"
    );

    // 4. Inspect Selena to see silver arrow hairpin
    let inspect_out = runtime.run_turn("look at selena").expect("look at selena");
    assert!(
        inspect_out.text().contains("silver arrow hairpin"),
        "must describe silver arrow hairpin, got: {}",
        inspect_out.text()
    );

    // 5. Speak the secret code word to Selena
    let talk_out = runtime
        .run_turn("talk to selena The arrow always finds home")
        .expect("talk to selena");
    assert!(
        talk_out.text().to_lowercase().contains("arrow")
            || talk_out.text().to_lowercase().contains("selena"),
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
    state.turn_number = 1;
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
        look_spiritual.text().contains("Trapped human spirits"),
        "must trigger spiritual sight override: {}",
        look_spiritual.text()
    );
    assert!(
        look_spiritual.text().contains("Seer"),
        "must deduce the living person as the Seer: {}",
        look_spiritual.text()
    );
}

#[test]
fn test_floor6_defeat_seer_and_activate_platform() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "mortuary_reanimation_sanctum".to_string();
    state.turn_number = 1;
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

    // Pick up revival scroll dropped by the Seer
    runtime
        .run_turn("take revival scroll")
        .expect("take scroll");
    assert!(runtime.export_state().unwrap().has_item("revival-scroll"));

    // Read revival scroll to learn Revival Sigil
    let read_out = runtime
        .run_turn("read revival scroll")
        .expect("read scroll");
    assert!(
        read_out.text().contains("Revival Sigil") || read_out.text().contains("mind"),
        "read scroll output: {}",
        read_out.text()
    );
    let s_after_read = runtime.export_state().unwrap();
    assert_eq!(s_after_read.story_vars.get("knows_revive"), Some("true"));
    assert!(s_after_read.has_item("revive-sigil"));

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
