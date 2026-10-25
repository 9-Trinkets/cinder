//! Integration tests for Floor 6 Canon Novel Alignment:
//! - Halfling Servants: Inspecting Flora, Silvia, and Orva reveals Halfling race and porcelain masks
//! - Cliff Balcony: Navigating through vent to secret cliff balcony, inspecting soul grass and iron door
//! - Direct Descent: Taking secret stairs from cliff balcony into Reanimation Sanctum
//! - Black Binding Ledger: Inspecting mortuary ledgers reveals Siraj, Nur, Orin, Mari, Ferid, and Hana
//! - Soul Chimera Dragon: Slaying the 20m construct directly defeats Valerius, sets seer_defeated, and yields bind-scroll
//! - Platform Climax: Platform activation triggers canonical reunion, Siraj/Nur awakening, and Harun capture

use cinder_core::content::loader::load_named_pack;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::WorldState;

#[test]
fn test_floor6_halfling_servants_and_orva_clues() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "herbarium_garden".to_string();
    state.clock.turn_number = 1;

    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime creates");

    // Inspect Servant No. 4
    let look_servant4 = runtime
        .run_turn("look at servant 4")
        .expect("look servant 4");
    assert!(
        look_servant4.text().to_lowercase().contains("halfling")
            && (look_servant4.text().contains("bread")
                || look_servant4.text().contains("trembles")),
        "Servant No. 4 must reflect trembling halfling hiding stolen bread: {}",
        look_servant4.text()
    );

    // Inspect Servant No. 7
    let look_servant7 = runtime
        .run_turn("look at servant 7")
        .expect("look servant 7");
    assert!(
        look_servant7.text().to_lowercase().contains("halfling")
            && (look_servant7.text().contains("seeds")
                || look_servant7.text().contains("moss")
                || look_servant7.text().contains("mud")),
        "Servant No. 7 must reflect calm halfling harvesting seeds with cliff moss on shoes: {}",
        look_servant7.text()
    );

    // Slip through vent to cliff balcony and inspect Orva
    runtime
        .run_turn("go out")
        .expect("slip through vent to balcony");
    let look_orva = runtime.run_turn("look at orva").expect("look orva");
    assert!(
        look_orva.text().to_lowercase().contains("halfling")
            && (look_orva.text().contains("capsule")
                || look_orva.text().contains("arrow")
                || look_orva.text().contains("soul grass")),
        "Orva must reflect halfling herbalist on cliff balcony: {}",
        look_orva.text()
    );
}

#[test]
fn test_floor6_cliff_balcony_navigation_and_soul_grass() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "herbarium_garden".to_string();
    state.clock.turn_number = 1;

    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime creates");

    // 1. Slip through the vent to the secret cliff balcony
    let move_out = runtime
        .run_turn("go out")
        .expect("move out to cliff balcony");
    let s1 = runtime.export_state().unwrap();
    assert_eq!(
        s1.current_room_id,
        "herbarium_cliff_balcony",
        "Layla slips through vent into cliff balcony: {}",
        move_out.text()
    );

    // 2. Inspect wild soul grass
    let look_grass = runtime
        .run_turn("look at soul grass")
        .expect("look at soul grass");
    assert!(
        look_grass.text().contains("azure light")
            || look_grass.text().contains("dew")
            || look_grass.text().contains("Capsule"),
        "Soul grass inspect text: {}",
        look_grass.text()
    );

    // 3. Inspect chained iron door
    let look_door = runtime
        .run_turn("look at iron door")
        .expect("look at iron door");
    assert!(
        look_door.text().contains("spiral staircase")
            || look_door.text().contains("Reanimation Sanctum"),
        "Iron door inspect text: {}",
        look_door.text()
    );

    // 4. Descend secret stairs into Reanimation Sanctum
    let descend_out = runtime.run_turn("go down").expect("descend into sanctum");
    let s2 = runtime.export_state().unwrap();
    assert_eq!(
        s2.current_room_id,
        "mortuary_reanimation_sanctum",
        "Secret stairs lead directly into Reanimation Sanctum: {}",
        descend_out.text()
    );

    // 5. Ascend back up to the cliff balcony
    runtime.run_turn("go up").expect("climb back to balcony");
    assert_eq!(
        runtime.export_state().unwrap().current_room_id,
        "herbarium_cliff_balcony"
    );
}

#[test]
fn test_floor6_sanctum_ledger_reveals_true_names() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "mortuary_reanimation_sanctum".to_string();
    state.clock.turn_number = 1;

    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime creates");

    // Inspect the black binding ledger in the sanctum
    let look_ledger = runtime.run_turn("look at ledger").expect("look at ledger");
    assert!(
        look_ledger.text().contains("Siraj")
            && look_ledger.text().contains("Nur")
            && look_ledger.text().contains("Orin")
            && look_ledger.text().contains("Mari")
            && look_ledger.text().contains("Ferid")
            && look_ledger.text().contains("Hana"),
        "Binding ledger must list the true names of Zayd's parents and the four golems: {}",
        look_ledger.text()
    );
}

#[test]
fn test_floor6_defeat_dragon_chimera_and_learn_bind() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "mortuary_reanimation_sanctum".to_string();
    state.clock.turn_number = 1;
    state.active_objective_stage_ids = vec!["mq_slay_the_seer".to_string()];
    state.story_vars.set_unchecked("spiritual_sight", "true");

    // Reduce dragon HP to 1 and buff player strength for clean one-hit defeat
    let dragon_hp = state.actor_stat("sanctuary_dragon", "hp");
    state
        .adjust_actor_stat(&pack, "sanctuary_dragon", "hp", -(dragon_hp - 1))
        .unwrap();
    state
        .adjust_actor_stat(&pack, "player", "strength", 100)
        .unwrap();

    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime creates");

    // Verify both Seer and Dragon are initially present
    let look_sanctum = runtime.run_turn("look").expect("look around");
    assert!(
        look_sanctum.text().contains("Soul Chimera Dragon"),
        "Dragon must be in sanctum: {}",
        look_sanctum.text()
    );

    // Attack the Dragon construct directly
    let attack_out = runtime.run_turn("attack dragon").expect("attack dragon");
    assert!(
        attack_out.text().contains("crumbles")
            || attack_out.text().contains("dust")
            || attack_out.text().contains("Soul Chimera Dragon"),
        "attack dragon output: {}",
        attack_out.text()
    );

    let state_post_dragon = runtime.export_state().unwrap();
    // Dragon is defeated
    assert!(
        state_post_dragon.actor_is_defeated("sanctuary_dragon", "hp"),
        "Dragon construct must be defeated"
    );
    // Story var dragon_defeated is set
    assert_eq!(
        state_post_dragon.story_vars.get("dragon_defeated"),
        Some("true"),
        "dragon_defeated must be true"
    );
    // Seer is NOT dead yet; he took magical backlash and is staggered at 10 HP
    assert!(
        !state_post_dragon.actor_is_defeated("the_seer", "hp"),
        "Seer must still be alive after dragon destruction"
    );
    assert_eq!(
        state_post_dragon.actor_stat("the_seer", "hp"),
        10,
        "Seer HP must be reduced to 10 from 56 recoil damage"
    );
    assert_ne!(
        state_post_dragon
            .actors
            .actor_room_overrides
            .get("the_seer")
            .map(String::as_str),
        Some(""),
        "Seer must not be despawned from the room"
    );
    assert_eq!(
        state_post_dragon.actor_room_id("the_seer", "mortuary_reanimation_sanctum"),
        "mortuary_reanimation_sanctum",
        "Seer remains in sanctum reeling from backlash"
    );

    // Attack Seer Valerius to deliver the finishing blow
    let attack_seer = runtime.run_turn("attack seer").expect("attack seer");
    assert!(
        attack_seer.text().contains("Seer")
            || attack_seer.text().contains("valerius")
            || attack_seer.text().contains("Valerius"),
        "attack seer output: {}",
        attack_seer.text()
    );
    let state_post_seer = runtime.export_state().unwrap();
    assert!(
        state_post_seer.actor_is_defeated("the_seer", "hp"),
        "Seer must be defeated"
    );
    assert_eq!(
        state_post_seer.story_vars.get("seer_defeated"),
        Some("true"),
        "seer_defeated must be true"
    );

    // Pick up the dropped Bind Scroll
    runtime.run_turn("take bind scroll").expect("take scroll");
    assert!(runtime.export_state().unwrap().has_item("bind-scroll"));

    // Read bind scroll to learn Bind Sigil
    let read_out = runtime.run_turn("read bind scroll").expect("read scroll");
    assert!(read_out.text().contains("Bind Sigil"));
    assert_eq!(
        runtime.export_state().unwrap().story_vars.get("knows_bind"),
        Some("true")
    );
    assert!(runtime.export_state().unwrap().has_item("bind-sigil"));
}

#[test]
fn test_floor6_platform_activation_canon_dialogue() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "mortuary_reanimation_sanctum".to_string();
    state.clock.turn_number = 1;
    state.active_objective_stage_ids = vec!["mq_activate_sanctuary_platform".to_string()];
    state.story_vars.set_unchecked("seer_defeated", "true");

    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime creates");

    // Move to platform: triggers platform activation message
    let move_out = runtime.run_turn("go south").expect("move to platform");
    let text = move_out.text();
    assert!(
        text.contains("Astrid")
            && text.contains("Einar")
            && text.contains("Orva")
            && text.contains("white stair"),
        "Platform activation must narrate Astrid, Einar, Orva, and the white stair: {}",
        text
    );
}
