use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::{ActorStance, WorldState};
use cinder_core::loader::load_named_pack;

#[test]
fn guarding_ally_intercepts_and_counterattacks_attacker() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);

    // Position player in r2c2 (goblin-1's room)
    state.current_room_id = "r2c2".to_string();

    // Place sakhra in r2c2 as an allied follower on guard
    state
        .actor_room_overrides
        .insert("sakhra".to_string(), "r2c2".to_string());
    state.set_actor_stance(&pack, "sakhra", ActorStance::Allied, false);
    state
        .assign_party_order(&pack, "sakhra", "guard".to_string())
        .expect("assign guard order");

    // Ensure goblin-1 is hostile and ready to strike
    state.set_actor_stance(&pack, "goblin-1", ActorStance::Hostile, false);
    state
        .next_hostile_strike_at
        .insert("goblin-1".to_string(), 0);

    let goblin_initial_hp = state.actor_stat("goblin-1", "hp");
    let sakhra_initial_hp = state.actor_stat("sakhra", "hp");
    let player_initial_hp = state.actor_stat("player", "hp");

    state.turn_number = 1;

    let dialogue =
        std::sync::Arc::new(cinder_core::engine::dialogue::ScriptedDialogueGenerator::new());
    let runtime =
        CinderRuntime::with_dialogue_generator(pack, state, dialogue).expect("runtime creates");

    // NPC tick triggers hostile strike from goblin-1
    let outcome = runtime.run_tick().expect("tick runs");
    let text = outcome.text();

    let end_state = runtime.export_state().unwrap();

    // 1. Guard intercepts damage meant for the player: player takes 0 damage, sakhra takes damage
    assert_eq!(
        end_state.actor_stat("player", "hp"),
        player_initial_hp,
        "Player should take no damage when guard intercepts"
    );
    assert!(
        end_state.actor_stat("sakhra", "hp") < sakhra_initial_hp,
        "Sakhra should have absorbed the intercept damage"
    );
    assert!(
        text.contains("steps across the goblin's strike and takes"),
        "Expected intercept narration in text: {text}"
    );

    // 2. Guard counterattacks: goblin takes counterattack damage from Sakhra
    assert!(
        end_state.actor_stat("goblin-1", "hp") < goblin_initial_hp,
        "Goblin should have taken counterattack damage from Sakhra"
    );
    assert!(
        text.contains("answers the strike, hitting the goblin for"),
        "Expected counterattack narration in text: {text}"
    );
}
