use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::{ActorStance, WorldState};
use cinder_core::loader::load_named_pack;

#[test]
fn guarding_ally_is_targeted_and_counterattacks_off_screen() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);

    // Leave Layla elsewhere while Sakhra holds the goblin's room.
    state.current_room_id = "r1c1".to_string();

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

    // 1. The hostile targets the stationed guard without requiring Layla nearby.
    assert_eq!(
        end_state.actor_stat("player", "hp"),
        player_initial_hp,
        "Layla should take no damage during an off-screen engagement"
    );
    assert!(
        end_state.actor_stat("sakhra", "hp") < sakhra_initial_hp,
        "Sakhra should be the goblin's direct target"
    );
    // Raw off-screen combat narration is suppressed from the player's view
    assert!(
        !text.contains("The goblin strikes Sakhra"),
        "Raw party-target strike narration should be suppressed off-screen: {text}"
    );

    // 2. Guard counterattacks: goblin takes counterattack damage from Sakhra mechanically
    assert!(
        end_state.actor_stat("goblin-1", "hp") < goblin_initial_hp,
        "Goblin should have taken counterattack damage from Sakhra"
    );
    assert!(
        !text.contains("answers the strike, hitting the goblin for"),
        "Raw counterattack narration should be suppressed off-screen: {text}"
    );
}

#[test]
fn guarding_ally_with_comms_transmits_remote_channel_dispatch() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);

    // Leave Layla in r1c1 while awakened Sakhra holds r2c2
    state.current_room_id = "r1c1".to_string();

    state
        .actor_room_overrides
        .insert("sakhra".to_string(), "r2c2".to_string());
    state.set_actor_stance(&pack, "sakhra", ActorStance::Allied, false);
    state
        .assign_party_order(&pack, "sakhra", "guard".to_string())
        .expect("assign guard order");

    // Grant comms to Sakhra (e.g. via awakening)
    state.grant_actor_skill("sakhra", "comms");

    // Ensure goblin-1 is hostile in r2c2 and ready to strike with enough HP to survive counterattack
    state
        .actor_stats
        .entry("goblin-1".to_string())
        .or_default()
        .insert("hp".to_string(), 20);
    state.set_actor_stance(&pack, "goblin-1", ActorStance::Hostile, false);
    state
        .next_hostile_strike_at
        .insert("goblin-1".to_string(), 0);

    state.turn_number = 1;

    let dialogue =
        std::sync::Arc::new(cinder_core::engine::dialogue::ScriptedDialogueGenerator::new());
    let runtime =
        CinderRuntime::with_dialogue_generator(pack, state, dialogue).expect("runtime creates");

    let outcome = runtime.run_tick().expect("tick runs");
    let text = outcome.text();

    // Raw combat spam is suppressed
    assert!(
        !text.contains("The goblin strikes Sakhra"),
        "Raw strike narration must be suppressed: {text}"
    );
    assert!(
        !text.contains("answers the strike, hitting the goblin for"),
        "Raw counterattack narration must be suppressed: {text}"
    );

    // But a channel comms dispatch from Sakhra is delivered!
    let channel_line = outcome.lines.iter().find(|l| {
        l.kind == cinder_core::engine::narrative::NarrativeLineKind::Channel
            && l.text.starts_with("Sakhra:")
    });
    assert!(
        channel_line.is_some(),
        "Expected Sakhra comms dispatch on Channel line kind, found: {:?}",
        outcome.lines
    );
    let dispatch = channel_line.unwrap();
    assert!(
        dispatch.text.contains("Hostiles engaged"),
        "Dispatch should report contact milestone: {}",
        dispatch.text
    );
}
