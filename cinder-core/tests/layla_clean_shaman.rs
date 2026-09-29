//! Integration tests for Goblin Shaman clean-run secret quest rewarding Shaman Ring.

use cinder_core::content::loader::load_named_pack;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::WorldState;

#[test]
fn clean_shaman_trial_triggers_and_rewards_ring_on_clean_run() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);

    // Initial state: sq_clean_shaman_listen is in active stages
    assert!(
        state
            .active_objective_stage_ids
            .contains(&"sq_clean_shaman_listen".to_string()),
        "sq_clean_shaman_listen must start active"
    );

    // Layla moves directly to r5c5 (the Goblin Shaman's room) without attacking anyone
    state.current_room_id = "r5c4".to_string();
    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");

    // Move east into r5c5
    runtime.run_turn("go east").expect("move to r5c5 succeeds");

    let s1 = runtime.export_state().expect("state exported");
    assert_eq!(s1.current_room_id, "r5c5");
    assert!(
        s1.active_objective_stage_ids
            .contains(&"sq_defeat_clean_shaman".to_string()),
        "Entering r5c5 on clean run activates sq_defeat_clean_shaman"
    );

    // Objectives query reveals the secret quest
    let summaries = runtime
        .current_objective_summaries()
        .expect("objectives fetched");
    let clean_shaman_obj = summaries
        .iter()
        .find(|o| o.quest_id.as_deref() == Some("clean_shaman"))
        .expect("clean_shaman objective present");
    assert_eq!(clean_shaman_obj.quest_kind.as_deref(), Some("secret"));
    assert_eq!(
        clean_shaman_obj.quest_title.as_deref(),
        Some("Trial of the Silent Path")
    );

    // Defeat the shaman (set player strength high to defeat in one attack)
    let mut s2 = runtime.export_state().expect("state exported");
    s2.actor_stats
        .entry("player".to_string())
        .or_default()
        .insert("strength".to_string(), 100);

    let runtime2 = CinderRuntime::from_state(pack.clone(), s2, false).expect("runtime creates");
    runtime2
        .run_turn("attack goblin shaman")
        .expect("attack shaman succeeds");

    let final_state = runtime2.export_state().expect("state exported");
    assert_eq!(
        final_state.story_vars.get("shaman_defeated"),
        Some("true"),
        "Shaman must be marked defeated"
    );
    assert!(
        final_state.has_item("shaman-ring"),
        "Player must receive shaman-ring as secret quest reward"
    );
    assert!(
        final_state
            .completed_stage_ids
            .contains("sq_defeat_clean_shaman"),
        "sq_defeat_clean_shaman must be completed"
    );
    // Sakhra on Floor 4 is unaffected by the Shaman's defeat
    assert_eq!(
        final_state.relationship("sakhra").stance,
        cinder_core::engine::state::ActorStance::Neutral
    );
    assert!(!final_state.follows_player("sakhra"));
    assert!(!final_state.is_party_member(&pack, "sakhra"));

    // Equipping the Shaman's ring converts Floor 1 cave golems, but NOT Sakhra on Floor 4
    let _ = runtime2
        .run_turn("equip shaman-ring")
        .expect("equip shaman ring");
    let s_equip = runtime2.export_state().unwrap();
    assert_eq!(
        s_equip.relationship("sakhra").stance,
        cinder_core::engine::state::ActorStance::Neutral,
        "Sakhra must remain neutral on Floor 4 after equipping the shaman ring"
    );
    assert!(
        !s_equip.follows_player("sakhra"),
        "Sakhra must not follow after equipping the shaman ring"
    );
    assert!(
        !s_equip.is_party_member(&pack, "sakhra"),
        "Sakhra must not join party after equipping the shaman ring"
    );
    assert_eq!(
        s_equip.actor_current_room_id(&pack, "sakhra"),
        "village_square"
    );
}

#[test]
fn clean_shaman_trial_forfeited_if_layla_attacked_floor1_mob() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);

    // Layla attacks a goblin on floor 1
    state.current_room_id = "r2c8".to_string(); // goblin-2 is in r2c8
    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");
    runtime
        .run_turn("attack goblin")
        .expect("attack goblin succeeds");

    let s1 = runtime.export_state().expect("state exported");
    assert_eq!(
        s1.story_vars.get("layla_attacked_floor_one_mob"),
        Some("true")
    );

    // Now move Layla to r5c5
    let mut s2 = s1;
    s2.current_room_id = "r5c4".to_string();
    let runtime2 = CinderRuntime::from_state(pack.clone(), s2, false).expect("runtime creates");
    runtime2.run_turn("go east").expect("move to r5c5 succeeds");

    let s3 = runtime2.export_state().expect("state exported");
    assert!(
        !s3.active_objective_stage_ids
            .contains(&"sq_defeat_clean_shaman".to_string()),
        "sq_defeat_clean_shaman must NOT activate if floor 1 mob was attacked"
    );

    // Defeat the shaman
    let mut s4 = s3;
    s4.actor_stats
        .entry("player".to_string())
        .or_default()
        .insert("strength".to_string(), 100);
    let runtime3 = CinderRuntime::from_state(pack.clone(), s4, false).expect("runtime creates");
    runtime3
        .run_turn("attack goblin shaman")
        .expect("attack shaman succeeds");

    let final_state = runtime3.export_state().expect("state exported");
    assert_eq!(final_state.story_vars.get("shaman_defeated"), Some("true"));
    assert!(
        !final_state.has_item("shaman-ring"),
        "Player must NOT receive shaman-ring if floor 1 mobs were attacked"
    );
}
