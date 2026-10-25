//! Integration tests for Floor 6 stealth mechanics: Invisibility Potions and Spirit Pill.

use cinder_core::content::loader::load_named_pack;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::{ActorStance, SpawnActorConfig, SpawnActorOutcome, WorldState};

#[test]
fn test_invisibility_potion_consumption_and_duration() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "sanctuary_grand_terrace".to_string();
    state.add_item("invisibility-potion");

    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime creates");

    // Before drinking: visible
    assert!(!runtime.export_state().unwrap().is_player_invisible());

    // Drink potion
    let out = runtime
        .run_turn("drink invisibility potion")
        .expect("drink potion");
    assert!(out.text().contains("fade from sight") || out.text().contains("potion"));

    let s1 = runtime.export_state().unwrap();
    assert!(s1.is_player_invisible());
    assert_eq!(s1.story_vars.get("is_invisible"), Some("true"));
    assert_eq!(s1.player_invisibility_remaining_minutes(), 8);

    // Moving between rooms advances time by 1 minute per turn
    // Move 1: terrace -> crossroads (7 mins remaining)
    runtime.run_turn("go south").expect("move 1");
    let s2 = runtime.export_state().unwrap();
    assert!(s2.is_player_invisible());
    assert_eq!(s2.player_invisibility_remaining_minutes(), 7);

    // Move 6 more times: alternating between crossroads and terrace (total 7 moves, 1 min remaining)
    for i in 0..6 {
        if i % 2 == 0 {
            runtime.run_turn("go north").expect("move north");
        } else {
            runtime.run_turn("go south").expect("move south");
        }
    }
    let s3 = runtime.export_state().unwrap();
    assert!(s3.is_player_invisible());
    assert_eq!(s3.player_invisibility_remaining_minutes(), 1);

    // 8th move: invisibility expires
    let expire_turn = runtime.run_turn("go north").expect("move to expire");
    assert!(
        expire_turn.text().contains("invisibility wears off"),
        "expiration must narrate message, got: {}",
        expire_turn.text()
    );

    let s4 = runtime.export_state().unwrap();
    assert!(!s4.is_player_invisible());
    assert_eq!(s4.story_vars.get("is_invisible"), Some("false"));
}

#[test]
fn test_invisibility_evades_hostile_targeting() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "courtyard_north".to_string();
    state.clock.turn_number = 1;
    state
        .story_vars
        .set_unchecked("frost_wolf_lane_stopped", "true");

    let _actor_id = match state.spawn_actor(
        &pack,
        SpawnActorConfig {
            template_id: "frost_wolf_raider",
            room_id: Some("courtyard_north"),
            stance: None,
            follows_player: false,
            scale_with_actor_id: None,
            scale_stat: None,
            max_active_instances: None,
        },
    ) {
        SpawnActorOutcome::Success(info) => info.instance_id,
        outcome => panic!("expected spawned hostile, got {outcome:?}"),
    };

    let initial_hp = state.actor_stat("player", "hp");

    // 1. When visible, hostile strikes target the player during a tick and reduce HP
    let runtime_visible =
        CinderRuntime::from_state(pack.clone(), state.clone(), false).expect("runtime creates");
    runtime_visible.run_tick().expect("tick runs");
    let hp_after_visible = runtime_visible
        .export_state()
        .unwrap()
        .actor_stat("player", "hp");
    assert!(
        hp_after_visible < initial_hp,
        "hostile raider should strike visible player during tick, hp: {hp_after_visible} < {initial_hp}"
    );

    // 2. When invisible, hostile raider does NOT strike the player and HP is untouched
    state.combat.invisible_until_minute = Some(state.clock.current_time_minutes + 8);
    state.story_vars.set_unchecked("is_invisible", "true");

    let runtime_invisible = CinderRuntime::from_state(pack, state, false).expect("runtime creates");
    runtime_invisible.run_tick().expect("tick runs");
    let hp_after_invisible = runtime_invisible
        .export_state()
        .unwrap()
        .actor_stat("player", "hp");
    assert_eq!(
        hp_after_invisible, initial_hp,
        "hostile raider must ignore invisible player, leaving HP unchanged"
    );
}

#[test]
fn test_attacking_breaks_invisibility_early() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "sanctuary_crossroads".to_string();
    state.add_item("invisibility-potion");

    // Spawn a hostile enemy in the room
    state.set_stance("citadel_sentry", ActorStance::Hostile);
    state.actors.actor_room_overrides.insert(
        "citadel_sentry".to_string(),
        "sanctuary_crossroads".to_string(),
    );

    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime creates");

    // Drink potion
    runtime
        .run_turn("use invisibility-potion")
        .expect("drink potion");
    assert!(runtime.export_state().unwrap().is_player_invisible());

    // Attack the sentry -> breaks invisibility immediately
    let attack_out = runtime.run_turn("attack sentry").expect("attack sentry");
    assert!(
        attack_out.text().contains("breaks your invisibility"),
        "attacking must narrate breaking invisibility, got: {}",
        attack_out.text()
    );

    let after_attack = runtime.export_state().unwrap();
    assert!(
        !after_attack.is_player_invisible(),
        "player must no longer be invisible after attacking"
    );
    assert_eq!(after_attack.story_vars.get("is_invisible"), Some("false"));
}

#[test]
fn test_spirit_pill_consumption_unlocks_spiritual_sight() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "sanctuary_grand_terrace".to_string();
    state.add_item("spirit-pill");

    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime creates");

    assert_ne!(
        runtime
            .export_state()
            .unwrap()
            .story_vars
            .get("spiritual_sight"),
        Some("true")
    );

    // Swallow pill
    let out = runtime
        .run_turn("swallow spirit pill")
        .expect("swallow pill");
    assert!(out.text().contains("see spirits") || out.text().contains("pill"));

    let after = runtime.export_state().unwrap();
    assert_eq!(after.story_vars.get("spiritual_sight"), Some("true"));
}
