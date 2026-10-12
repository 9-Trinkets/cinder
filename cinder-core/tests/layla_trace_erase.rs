use cinder_core::content::loader::load_named_pack;
use cinder_core::content::types::ItemStorageTarget;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::WorldState;

#[test]
fn test_trace_unlimited_capacity_sp_gain_mp_discount_and_erase() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.acquire_player_item(&pack, "magic-chalk");
    state.story_vars.set_unchecked("knows_teleport", "true");
    state.story_vars.set_unchecked("knows_drain", "true");
    state
        .combat
        .initial_actor_stats
        .entry("player".to_string())
        .or_default()
        .insert("mp".to_string(), 50);
    state
        .combat
        .actor_stats
        .entry("player".to_string())
        .or_default()
        .insert("mp".to_string(), 50);
    state.current_room_id = "village_square".to_string();

    let dialogue =
        std::sync::Arc::new(cinder_core::engine::dialogue::ScriptedDialogueGenerator::new());
    let runtime = CinderRuntime::with_dialogue_generator(pack.clone(), state, dialogue)
        .expect("runtime creates");

    // Player starts at Trace Level 1, 0 SP
    let s0 = runtime.export_state().unwrap();
    assert_eq!(s0.actor_skill_level("player", "trace"), 1);
    assert_eq!(s0.actor_skill_sp("player", "trace"), 0);

    // 1. Trace drain sigil in village_square
    let out1 = runtime.run_turn("trace drain-sigil").expect("trace 1");
    assert!(out1.text().contains("draw") || out1.text().contains("chalk"));
    let s1 = runtime.export_state().unwrap();
    assert!(s1.has_item_in_storage(
        "drain-sigil",
        ItemStorageTarget::CurrentRoom,
        "village_square"
    ));
    assert_eq!(s1.actor_skill_sp("player", "trace"), 1);
    // Base MP cost of drain-sigil is 2, Trace level 1 discount is 0 -> 48 MP
    assert_eq!(s1.actor_stat_u32("player", "mp"), 48);

    // 2. Move to village_sw_corner and trace 2nd drain sigil
    runtime.run_turn("go west").expect("move 1");
    runtime.run_turn("trace drain-sigil").expect("trace 2");
    let s2 = runtime.export_state().unwrap();
    assert_eq!(s2.actor_skill_sp("player", "trace"), 2);
    assert_eq!(s2.actor_stat_u32("player", "mp"), 46);
    assert!(s2.has_item_in_storage(
        "drain-sigil",
        ItemStorageTarget::CurrentRoom,
        "village_square"
    ));

    // 3. Move to village_west_1 and trace 3rd drain sigil
    runtime.run_turn("go northeast").expect("move 2");
    runtime.run_turn("trace drain-sigil").expect("trace 3");
    let s3 = runtime.export_state().unwrap();
    assert_eq!(s3.actor_skill_sp("player", "trace"), 3);
    assert!(s3.has_item_in_storage(
        "drain-sigil",
        ItemStorageTarget::CurrentRoom,
        "village_square"
    ));

    // 4. Move to village_west_2 and trace 4th drain sigil
    runtime.run_turn("go northeast").expect("move 4");
    runtime.run_turn("trace drain-sigil").expect("trace 4");
    let s4 = runtime.export_state().unwrap();
    assert_eq!(s4.actor_skill_sp("player", "trace"), 4);
    assert!(s4.has_item_in_storage(
        "drain-sigil",
        ItemStorageTarget::CurrentRoom,
        "village_square"
    ));

    // 5. Move to village_north_gate and trace 5th drain sigil (previously blocked by 4-sigil capacity!)
    runtime.run_turn("go northeast").expect("move 5");
    let out5 = runtime.run_turn("trace drain-sigil").expect("trace 5");
    assert!(!out5.text().contains("cannot sustain more than"));
    let s5 = runtime.export_state().unwrap();
    assert!(s5.has_item_in_storage(
        "drain-sigil",
        ItemStorageTarget::CurrentRoom,
        "village_north_gate"
    ));
    assert_eq!(s5.actor_skill_sp("player", "trace"), 5);

    // All 5 sigils remain in their respective rooms!
    assert!(s5.has_item_in_storage(
        "drain-sigil",
        ItemStorageTarget::CurrentRoom,
        "village_square"
    ));
    assert!(s5.has_item_in_storage(
        "drain-sigil",
        ItemStorageTarget::CurrentRoom,
        "village_sw_corner"
    ));
    assert!(s5.has_item_in_storage(
        "drain-sigil",
        ItemStorageTarget::CurrentRoom,
        "village_west_1"
    ));
    assert!(s5.has_item_in_storage(
        "drain-sigil",
        ItemStorageTarget::CurrentRoom,
        "village_west_2"
    ));
    assert!(s5.has_item_in_storage(
        "drain-sigil",
        ItemStorageTarget::CurrentRoom,
        "village_north_gate"
    ));

    // 6. Test Erase command in village_north_gate
    let erase_out = runtime.run_turn("erase").expect("erase turn");
    assert!(erase_out.text().contains("rub out"));
    let s6 = runtime.export_state().unwrap();
    assert!(!s6.has_item_in_storage(
        "drain-sigil",
        ItemStorageTarget::CurrentRoom,
        "village_north_gate"
    ));

    // Erasing again when room has no sigil rejects
    let erase_empty = runtime.run_turn("erase").expect("erase turn");
    assert!(erase_empty.text().contains("no chalk sigils to erase here"));

    // 7. Test Erase with teleport anchor
    runtime
        .run_turn("trace teleport sigil")
        .expect("trace teleport");
    let s7 = runtime.export_state().unwrap();
    assert!(s7.has_chalk_anchor("village_north_gate"));
    runtime.run_turn("erase").expect("erase teleport");
    let s8 = runtime.export_state().unwrap();
    assert!(!s8.has_chalk_anchor("village_north_gate"));
    assert!(!s8.has_item_in_storage(
        "teleport-sigil",
        ItemStorageTarget::CurrentRoom,
        "village_north_gate"
    ));
}

#[test]
fn test_trace_mp_discount_scaling() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.acquire_player_item(&pack, "magic-chalk");
    state.story_vars.set_unchecked("knows_teleport", "true");
    // Level up Trace to Level 5 (discount 2 MP)
    state.set_actor_skill_level("player", "trace", 5);
    state
        .combat
        .initial_actor_stats
        .entry("player".to_string())
        .or_default()
        .insert("mp".to_string(), 50);
    state
        .combat
        .actor_stats
        .entry("player".to_string())
        .or_default()
        .insert("mp".to_string(), 50);
    state.current_room_id = "village_square".to_string();

    let dialogue =
        std::sync::Arc::new(cinder_core::engine::dialogue::ScriptedDialogueGenerator::new());
    let runtime = CinderRuntime::with_dialogue_generator(pack.clone(), state, dialogue)
        .expect("runtime creates");

    // Base cost of teleport-sigil is 4 MP. At Trace Level 5, discount is 2 MP -> costs 2 MP.
    let mp_before = runtime
        .export_state()
        .unwrap()
        .actor_stat_u32("player", "mp");
    runtime.run_turn("trace teleport sigil").expect("trace tp");
    let mp_after = runtime
        .export_state()
        .unwrap()
        .actor_stat_u32("player", "mp");
    assert_eq!(mp_after, mp_before - 2);

    // Teleporting itself (base 6 MP) with Trace Level 5 discount (2 MP) -> costs 4 MP.
    runtime.run_turn("go west").expect("move west");
    let mp_before_tp = runtime
        .export_state()
        .unwrap()
        .actor_stat_u32("player", "mp");
    runtime
        .run_turn("teleport village_square")
        .expect("teleport");
    let mp_after_tp = runtime
        .export_state()
        .unwrap()
        .actor_stat_u32("player", "mp");
    assert_eq!(mp_after_tp, mp_before_tp - 4);
}
