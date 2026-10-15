use cinder_core::content::loader::load_named_pack;
use cinder_core::content::types::ItemStorageTarget;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::{ActorStance, WorldState};

#[test]
fn tracing_sigils_awards_discipline_sp() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "r1c1".to_string();
    state.add_item("magic-chalk");

    let player_id = &pack.settings.combat.player_actor_id;
    assert_eq!(state.actor_skill_sp(player_id, "trace"), 0);
    assert_eq!(state.actor_skill_sp(player_id, "charm"), 0);

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");

    // Tracing charm-sigil awards 1 SP to trace and 1 SP to charm
    runtime
        .run_turn("trace charm-sigil")
        .expect("trace charm-sigil succeeds");

    let s1 = runtime.export_state().expect("state exported");
    assert_eq!(s1.actor_skill_sp(player_id, "trace"), 1);
    assert_eq!(s1.actor_skill_sp(player_id, "charm"), 1);
}

#[test]
fn charm_bonus_scales_surround_threshold() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let state = WorldState::new(&pack);

    let player_id = &pack.settings.combat.player_actor_id;
    // Base stats: player int 3, level 1.
    // At charm level 1, charm_bonus is +1 -> player score = 3 + 1 + 1 = 5.
    // Target with intelligence 4, level 1 has resistance 4 + 2*1 = 6 > 5.
    let mut target = state
        .actor(&pack, "goblin-shaman")
        .cloned()
        .expect("shaman exists");
    target.id = "test-resistant-mob".to_string();
    target.name = "test resistant mob".to_string();
    target.level = 1;
    let mut pack_with_mob = pack.clone();
    pack_with_mob.actors.push(target);
    cinder_core::engine::test_fixtures::rebuild_test_pack_indexes(&mut pack_with_mob);

    let mut s = WorldState::new(&pack_with_mob);
    s.combat
        .actor_stats
        .entry("test-resistant-mob".to_string())
        .or_default()
        .insert("intelligence".to_string(), 4);
    s.combat
        .actor_stats
        .entry("test-resistant-mob".to_string())
        .or_default()
        .insert("hp".to_string(), 20);

    // At charm level 1 (+1 bonus): score 5 < 6 (fails resistance)
    s.set_actor_skill_level(player_id, "charm", 1);
    let passes_lvl1 = {
        let p_int = s
            .effective_actor_stat(&pack_with_mob, player_id, "intelligence")
            .max(0) as u32;
        let p_lvl = s.actor_level(player_id);
        let c_bonus = pack_with_mob
            .skill("charm")
            .and_then(|sk| sk.param_value("charm_bonus", 1))
            .unwrap_or(1) as u32;
        let t_int = s.actor_stat("test-resistant-mob", "intelligence").max(0) as u32;
        let t_lvl = s.actor_level("test-resistant-mob");
        p_int + p_lvl + c_bonus >= t_int + 2 * t_lvl
    };
    assert!(!passes_lvl1, "Level 1 charm cannot overcome resistance 6");

    // At charm level 2 (+1 bonus): score 5 < 6 (still fails resistance)
    s.set_actor_skill_level(player_id, "charm", 2);
    let passes_lvl2 = {
        let p_int = s
            .effective_actor_stat(&pack_with_mob, player_id, "intelligence")
            .max(0) as u32;
        let p_lvl = s.actor_level(player_id);
        let c_bonus = pack_with_mob
            .skill("charm")
            .and_then(|sk| sk.param_value("charm_bonus", 2))
            .unwrap_or(1) as u32;
        let t_int = s.actor_stat("test-resistant-mob", "intelligence").max(0) as u32;
        let t_lvl = s.actor_level("test-resistant-mob");
        p_int + p_lvl + c_bonus >= t_int + 2 * t_lvl
    };
    assert!(
        !passes_lvl2,
        "Level 2 charm (+1 bonus) cannot overcome resistance 6"
    );

    // At charm level 3 (+2 bonus): score 3 + 1 + 2 = 6 >= 6 (passes resistance!)
    s.set_actor_skill_level(player_id, "charm", 3);
    let passes_lvl3 = {
        let p_int = s
            .effective_actor_stat(&pack_with_mob, player_id, "intelligence")
            .max(0) as u32;
        let p_lvl = s.actor_level(player_id);
        let c_bonus = pack_with_mob
            .skill("charm")
            .and_then(|sk| sk.param_value("charm_bonus", 3))
            .unwrap_or(2) as u32;
        let t_int = s.actor_stat("test-resistant-mob", "intelligence").max(0) as u32;
        let t_lvl = s.actor_level("test-resistant-mob");
        p_int + p_lvl + c_bonus >= t_int + 2 * t_lvl
    };
    assert!(
        passes_lvl3,
        "Level 3 charm (+2 bonus) overcomes resistance 6"
    );
}

#[test]
fn multiple_followers_do_not_block_surrounds_or_charm_sp_gain() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);

    // Give player 2 active followers (which used to be the hard cap at charm level 1)
    state.set_actor_stance(&pack, "golem-dark-nw", ActorStance::Allied, true);
    state.set_actor_stance(&pack, "golem-dark-sw", ActorStance::Allied, true);

    let player_id = &pack.settings.combat.player_actor_id;
    let initial_sp = state.actor_skill_sp(player_id, "charm");

    // Set up surround around r3c7 (pale golem)
    // r3c7 neighbors in layla cave
    state.current_room_id = "r3c6".to_string();
    state.add_item("magic-chalk");

    let neighbors = pack.adjacent_room_ids("r3c7");
    for n in &neighbors {
        if n != "r3c6" {
            state.add_item_to_storage("charm-sigil", ItemStorageTarget::CurrentRoom, n);
        }
    }

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");

    // Trace the completing charm sigil to trigger surround
    let outcome = runtime
        .run_turn("trace charm-sigil")
        .expect("trace completes surround");

    let s_after = runtime.export_state().expect("state exported");
    // Target was converted to ally despite player already having 2 followers!
    assert_eq!(
        s_after.stance("golem-pale-ne"),
        ActorStance::Allied,
        "Pale golem must be converted"
    );
    // Player gained SP (1 from trace + 10 from surround conversion)
    assert_eq!(
        s_after.actor_skill_sp(player_id, "charm"),
        initial_sp + 11,
        "Player must gain Charm SP on surround conversion"
    );
    assert!(
        !outcome.text().contains("The ring does not hold"),
        "Surround must not be refused due to follower count"
    );
}
