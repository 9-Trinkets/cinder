use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::{ActorStance, WorldState};
use cinder_core::loader::load_named_pack;

#[test]
fn hostile_bishop_heals_damaged_ally_before_striking() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);

    // Position player in d8c3 (elf-bishop-3's room)
    state.current_room_id = "d8c3".to_string();

    // Place an allied elf mob (elf-pawn-1) in d8c3 with reduced HP
    state
        .actor_room_overrides
        .insert("elf-pawn-1".to_string(), "d8c3".to_string());
    state.set_actor_stance(&pack, "elf-pawn-1", ActorStance::Hostile, false);
    state
        .actor_stats
        .entry("elf-pawn-1".to_string())
        .or_default()
        .insert("hp".to_string(), 2);
    state
        .next_hostile_strike_at
        .insert("elf-pawn-1".to_string(), state.current_time_minutes + 10);

    // elf-bishop-3 is ready to strike/act
    state.set_actor_stance(&pack, "elf-bishop-3", ActorStance::Hostile, false);
    state
        .next_hostile_strike_at
        .insert("elf-bishop-3".to_string(), state.current_time_minutes);

    let player_initial_hp = state.actor_stat("player", "hp");

    state.turn_number = 1;
    let dialogue =
        std::sync::Arc::new(cinder_core::engine::dialogue::ScriptedDialogueGenerator::new());
    let runtime =
        CinderRuntime::with_dialogue_generator(pack, state, dialogue).expect("runtime creates");

    let outcome = runtime.run_tick().expect("tick runs");
    let text = outcome.text();
    let end_state = runtime.export_state().unwrap();

    // Bishop should have healed elf-pawn-1 (amount 4: 2 -> 6)
    assert_eq!(
        end_state.actor_stat("elf-pawn-1", "hp"),
        6,
        "Elf pawn should be healed from 2 to 6"
    );

    // Player should not have been struck by the bishop
    assert_eq!(
        end_state.actor_stat("player", "hp"),
        player_initial_hp,
        "Player should take no damage when bishop heals an ally"
    );

    assert!(
        text.contains(
            "raises a crooked branch, bathing elf pawn in soft green light to restore 4 health"
        ),
        "Expected bishop heal narration in text: {text}"
    );
}

#[test]
fn hostile_bishop_heals_self_when_badly_injured() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);

    state.current_room_id = "d8c3".to_string();

    // Severely wound bishop (< 50% of 10 max HP)
    state
        .actor_stats
        .entry("elf-bishop-3".to_string())
        .or_default()
        .insert("hp".to_string(), 3);

    state.set_actor_stance(&pack, "elf-bishop-3", ActorStance::Hostile, false);
    state
        .next_hostile_strike_at
        .insert("elf-bishop-3".to_string(), 0);

    let player_initial_hp = state.actor_stat("player", "hp");

    state.turn_number = 1;
    let dialogue =
        std::sync::Arc::new(cinder_core::engine::dialogue::ScriptedDialogueGenerator::new());
    let runtime =
        CinderRuntime::with_dialogue_generator(pack, state, dialogue).expect("runtime creates");

    let outcome = runtime.run_tick().expect("tick runs");
    let text = outcome.text();
    let end_state = runtime.export_state().unwrap();

    // Bishop should have self-healed by 4 (3 -> 7)
    assert_eq!(
        end_state.actor_stat("elf-bishop-3", "hp"),
        7,
        "Elf bishop should self-heal from 3 to 7"
    );

    assert_eq!(
        end_state.actor_stat("player", "hp"),
        player_initial_hp,
        "Player should take no damage when bishop heals itself"
    );

    assert!(
        text.contains("bathing itself in soft green light to restore 4 health"),
        "Expected bishop self-heal narration in text: {text}"
    );
}

#[test]
fn hostile_bishop_strikes_player_when_all_hostiles_at_full_health() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);

    state.current_room_id = "d8c3".to_string();

    // Bishop is at full health (10 HP), no wounded allies
    state.set_actor_stance(&pack, "elf-bishop-3", ActorStance::Hostile, false);
    state
        .next_hostile_strike_at
        .insert("elf-bishop-3".to_string(), 0);

    let player_initial_hp = state.actor_stat("player", "hp");

    state.turn_number = 1;
    let dialogue =
        std::sync::Arc::new(cinder_core::engine::dialogue::ScriptedDialogueGenerator::new());
    let runtime =
        CinderRuntime::with_dialogue_generator(pack, state, dialogue).expect("runtime creates");

    let outcome = runtime.run_tick().expect("tick runs");
    let text = outcome.text();
    let end_state = runtime.export_state().unwrap();

    // Bishop should strike the player because everyone is healthy
    assert!(
        end_state.actor_stat("player", "hp") < player_initial_hp,
        "Player should take damage from bishop strike"
    );

    assert!(
        text.contains("strikes you! You take"),
        "Expected strike narration in text: {text}"
    );
}

#[test]
fn hostile_queen_heals_damaged_ally_for_six() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);

    state.current_room_id = "d8c4".to_string();

    // Place an allied elf mob in d8c4 with missing HP
    state
        .actor_room_overrides
        .insert("elf-rook-1".to_string(), "d8c4".to_string());
    state.set_actor_stance(&pack, "elf-rook-1", ActorStance::Hostile, false);
    state
        .actor_stats
        .entry("elf-rook-1".to_string())
        .or_default()
        .insert("hp".to_string(), 8);
    state
        .next_hostile_strike_at
        .insert("elf-rook-1".to_string(), state.current_time_minutes + 10);

    // elf-queen-4 ready to act
    state.set_actor_stance(&pack, "elf-queen-4", ActorStance::Hostile, false);
    state
        .next_hostile_strike_at
        .insert("elf-queen-4".to_string(), state.current_time_minutes);

    state.turn_number = 1;
    let dialogue =
        std::sync::Arc::new(cinder_core::engine::dialogue::ScriptedDialogueGenerator::new());
    let runtime =
        CinderRuntime::with_dialogue_generator(pack, state, dialogue).expect("runtime creates");

    let outcome = runtime.run_tick().expect("tick runs");
    let text = outcome.text();
    let end_state = runtime.export_state().unwrap();

    // Queen should heal for 6 (8 -> 14)
    assert_eq!(
        end_state.actor_stat("elf-rook-1", "hp"),
        14,
        "Elf rook should be healed from 8 to 14 by queen"
    );

    assert!(
        text.contains("channeling pale emerald light to restore 6 health"),
        "Expected queen heal narration in text: {text}"
    );
}

#[test]
fn allied_bishop_supports_wounded_ally_in_combat() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);

    state.current_room_id = "r2c2".to_string();

    // Place elf-bishop-3 as allied follower in r2c2
    state
        .actor_room_overrides
        .insert("elf-bishop-3".to_string(), "r2c2".to_string());
    state.set_actor_stance(&pack, "elf-bishop-3", ActorStance::Allied, false);
    state
        .assign_party_order(&pack, "elf-bishop-3", "follow".to_string())
        .expect("assign follow order");

    // Goblin-1 is hostile in r2c2 and ready to strike
    state.set_actor_stance(&pack, "goblin-1", ActorStance::Hostile, false);
    state
        .next_hostile_strike_at
        .insert("goblin-1".to_string(), state.current_time_minutes);

    // Set player HP to 6 (> 50% so bishop doesn't intercept, but taking 1 damage wounds player)
    state
        .actor_stats
        .entry("player".to_string())
        .or_default()
        .insert("hp".to_string(), 6);

    state.turn_number = 1;
    let dialogue =
        std::sync::Arc::new(cinder_core::engine::dialogue::ScriptedDialogueGenerator::new());
    let runtime =
        CinderRuntime::with_dialogue_generator(pack, state, dialogue).expect("runtime creates");

    let outcome = runtime.run_tick().expect("tick runs");
    let text = outcome.text();
    let end_state = runtime.export_state().unwrap();

    // Goblin strikes player for 1 damage (6 -> 5).
    // Bishop reacts with support heal for 4 (5 -> 9).
    assert_eq!(
        end_state.actor_stat("player", "hp"),
        9,
        "Player should be healed by 4 after strike"
    );

    assert!(
        text.contains("supports Layla, restoring 4 hp"),
        "Expected party support narration in text: {text}"
    );
}

#[test]
fn allied_bishop_counterattacks_when_no_ally_wounded() {
    let mut pack = load_named_pack("layla", Some("en")).expect("layla loads");
    // Make player immune to physical damage via resistance so player takes 0 damage and remains at full HP
    pack.actors
        .iter_mut()
        .find(|a| a.id == "player")
        .expect("player actor")
        .resistances
        .insert("physical".to_string(), 100);

    let mut state = WorldState::new(&pack);

    state.current_room_id = "r2c2".to_string();

    // Place elf-bishop-3 as allied follower in r2c2
    state
        .actor_room_overrides
        .insert("elf-bishop-3".to_string(), "r2c2".to_string());
    state.set_actor_stance(&pack, "elf-bishop-3", ActorStance::Allied, false);
    state
        .assign_party_order(&pack, "elf-bishop-3", "follow".to_string())
        .expect("assign follow order");

    // Goblin-1 is hostile in r2c2 and ready to strike
    state.set_actor_stance(&pack, "goblin-1", ActorStance::Hostile, false);
    state
        .next_hostile_strike_at
        .insert("goblin-1".to_string(), state.current_time_minutes);

    let goblin_initial_hp = state.actor_stat("goblin-1", "hp");

    state.turn_number = 1;
    let dialogue =
        std::sync::Arc::new(cinder_core::engine::dialogue::ScriptedDialogueGenerator::new());
    let runtime =
        CinderRuntime::with_dialogue_generator(pack, state, dialogue).expect("runtime creates");

    let outcome = runtime.run_tick().expect("tick runs");
    let text = outcome.text();
    let end_state = runtime.export_state().unwrap();

    // Because no ally was wounded (player resisted damage), bishop does not heal;
    // instead bishop counterattacks the goblin!
    assert!(
        end_state.actor_stat("goblin-1", "hp") < goblin_initial_hp,
        "Goblin should take counterattack damage from bishop"
    );

    assert!(
        text.contains("answers the strike, hitting the goblin for"),
        "Expected counterattack narration in text: {text}"
    );
}

#[test]
fn allied_bishop_joins_player_attack() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);

    state.current_room_id = "r2c2".to_string();

    // Place elf-bishop-3 as allied follower in r2c2
    state
        .actor_room_overrides
        .insert("elf-bishop-3".to_string(), "r2c2".to_string());
    state.set_actor_stance(&pack, "elf-bishop-3", ActorStance::Allied, false);
    state
        .assign_party_order(&pack, "elf-bishop-3", "follow".to_string())
        .expect("assign follow order");

    // Goblin-1 is hostile in r2c2 with high HP
    state.set_actor_stance(&pack, "goblin-1", ActorStance::Hostile, false);
    state
        .actor_stats
        .entry("goblin-1".to_string())
        .or_default()
        .insert("hp".to_string(), 20);

    state.turn_number = 1;
    let dialogue =
        std::sync::Arc::new(cinder_core::engine::dialogue::ScriptedDialogueGenerator::new());
    let runtime =
        CinderRuntime::with_dialogue_generator(pack, state, dialogue).expect("runtime creates");

    let outcome = runtime.run_turn("attack goblin").expect("turn runs");
    let text = outcome.text();
    let end_state = runtime.export_state().unwrap();

    // Bishop should join the assault and deal ally attack damage
    assert!(
        text.contains("elf bishop joins the strike")
            || text.contains("elf bishop joins the assault")
            || text.contains("elf bishop strikes alongside you"),
        "Expected ally attack narration in text: {text}"
    );
    assert!(
        end_state.actor_stat("goblin-1", "hp") < 20,
        "Goblin took damage from player + ally strike"
    );
}
