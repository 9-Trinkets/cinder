use super::super::common::*;
use super::transcript;
use cinder_core::content::types::{CombatSettingsDefinition, PackMessage, StatDefinition};
use cinder_core::engine::events::{TimestampedWorldEvent, WorldEvent};
use cinder_core::engine::reducer::apply_events;
use cinder_core::engine::state::{ActorStance, GamePhase, WorldState};
use std::collections::BTreeMap;

#[test]
fn hostile_strike_uses_pack_declared_combat_vocabulary() {
    let mut pack = reducer_test_pack();
    pack.stats.actor.insert(
        "vitality".to_string(),
        StatDefinition {
            default: 10,
            ..StatDefinition::default()
        },
    );
    pack.stats
        .actor
        .insert("fury".to_string(), StatDefinition::default());
    pack.stats
        .actor
        .insert("ward".to_string(), StatDefinition::default());
    for actor in &mut pack.actors {
        actor.initial_stats.insert("vitality".to_string(), 10);
        actor.initial_stats.insert("ward".to_string(), 0);
    }
    pack.settings.combat = CombatSettingsDefinition {
        player_actor_id: ACTOR_A_NAME.to_string(),
        health_stat_id: "vitality".to_string(),
        attack_stat_id: "fury".to_string(),
        defense_stat_id: "ward".to_string(),
        minimum_damage: 2,
        default_attack_interval_minutes: 7,
        ..CombatSettingsDefinition::default()
    };
    let mut state = WorldState::new(&pack);
    state
        .actor_stats
        .entry(ACTOR_A_ID.to_string())
        .or_default()
        .insert("fury".to_string(), 5);
    state.set_stance(ACTOR_A_ID, ActorStance::Hostile);
    let events = [TimestampedWorldEvent::now(WorldEvent::HostileStrike {
        actor_id: ACTOR_A_ID.to_string(),
    })];

    apply_events(&mut state, &pack, &events);

    assert_eq!(state.actor_stat(ACTOR_A_NAME, "vitality"), 5);
    assert_eq!(
        state.next_hostile_strike_at.get(ACTOR_A_ID),
        Some(&(state.current_time_minutes + 7))
    );
}

#[test]
fn player_defeat_narration_and_phase_come_from_content() {
    let mut pack = reducer_test_pack();
    let defeat_text = "The lounge lights fade for good.";
    pack.settings.combat = CombatSettingsDefinition {
        player_actor_id: ACTOR_B_NAME.to_string(),
        health_stat_id: "stamina".to_string(),
        attack_stat_id: "confidence".to_string(),
        minimum_damage: 9,
        player_defeat_text: defeat_text.to_string(),
        ..CombatSettingsDefinition::default()
    };
    let mut state = WorldState::new(&pack);
    // Move Casey into the player's room so the strike is in range.
    state
        .actor_room_overrides
        .insert(ACTOR_C_ID.to_string(), LOUNGE_ID.to_string());
    state.set_stance(ACTOR_C_ID, ActorStance::Hostile);
    // Casey shares the lounge with the player actor (Blair).
    let events = [TimestampedWorldEvent::now(WorldEvent::HostileStrike {
        actor_id: ACTOR_C_ID.to_string(),
    })];

    let output = apply_events(&mut state, &pack, &events);

    assert_eq!(state.phase, GamePhase::GameEnded);
    assert_eq!(output.phase, GamePhase::GameEnded);
    assert!(
        output.lines.iter().any(|line| line.text == defeat_text),
        "expected pack-authored defeat line, got {:?}",
        output.lines
    );
}

#[test]
fn hostile_strike_respects_defender_resistance() {
    let mut pack = reducer_test_pack();
    pack.settings.combat = CombatSettingsDefinition {
        player_actor_id: ACTOR_A_ID.to_string(),
        health_stat_id: "stamina".to_string(),
        attack_stat_id: "confidence".to_string(),
        defense_stat_id: "hunger".to_string(),
        ..CombatSettingsDefinition::default()
    };
    pack.messages.insert(
        "combat.hostile_strike".to_string(),
        PackMessage::Narration("{actor} strikes you for {damage} ({remaining} remaining).".to_string()),
    );
    pack.messages.insert(
        "combat.no_effect".to_string(),
        PackMessage::Narration("You are unharmed by {kind}.".to_string()),
    );
    let mut player = test_actor(ACTOR_A_ID, ACTOR_A_NAME, LOUNGE_ID);
    player.initial_stats = BTreeMap::from([("stamina".to_string(), 10)]);
    // The defender shuts down fire entirely.
    player.resistances = BTreeMap::from([("fire".to_string(), 999)]);
    pack.actors = vec![player];
    let mut salamander = test_actor("salamander", "salamander", LOUNGE_ID);
    salamander.initial_stats = BTreeMap::from([("stamina".to_string(), 10)]);
    salamander.attack_interval_minutes = Some(1);
    salamander.attack_kind = "fire".to_string();
    pack.actors.push(salamander);
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.set_stance("salamander", ActorStance::Hostile);

    let output = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::HostileStrike {
            actor_id: "salamander".to_string(),
        })],
    );

    assert_eq!(state.actor_stat(ACTOR_A_ID, "stamina"), 10);
    let output_text = transcript(&output.lines);
    assert!(
        output_text.contains("unharmed by fire"),
        "expected fire no_effect, got: {output_text}"
    );
}

#[test]
fn hostile_strike_intercepted_by_guard_takes_at_least_minimum_damage() {
    let mut pack = reducer_test_pack();
    pack.settings.combat = CombatSettingsDefinition {
        player_actor_id: ACTOR_A_ID.to_string(),
        health_stat_id: "stamina".to_string(),
        attack_stat_id: "confidence".to_string(),
        defense_stat_id: "hunger".to_string(),
        minimum_damage: 1,
        ..CombatSettingsDefinition::default()
    };
    pack.messages.insert(
        "combat.guard_intercepts".to_string(),
        PackMessage::Narration(
            "{actor} strikes, but {guard} steps in front of you, taking {damage} damage."
                .to_string(),
        ),
    );
    let mut player = test_actor(ACTOR_A_ID, ACTOR_A_NAME, LOUNGE_ID);
    player.initial_stats = BTreeMap::from([("stamina".to_string(), 10)]);
    pack.actors = vec![player];
    let mut salamander = test_actor("salamander", "salamander", LOUNGE_ID);
    salamander.initial_stats = BTreeMap::from([("confidence".to_string(), 3)]);
    salamander.attack_kind = "fire".to_string();
    pack.actors.push(salamander);
    // The guard soaks the entire blow against its own defense: defense 3 would
    // theoretically zero it out, but the minimum-damage floor still applies.
    let mut guard = test_actor("bodyguard", "golem bodyguard", LOUNGE_ID);
    guard.guard = true;
    guard.initial_stats = BTreeMap::from([
        ("stamina".to_string(), 10),
        ("hunger".to_string(), 3),
    ]);
    pack.actors.push(guard);
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.set_relationship(
        "bodyguard",
        cinder_core::engine::state::ActorRelationship {
            stance: ActorStance::Allied,
            follows_player: true,
        },
    );
    state.set_stance("salamander", ActorStance::Hostile);

    let output = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::HostileStrike {
            actor_id: "salamander".to_string(),
        })],
    );

    // The player takes no damage; the guard absorbs the blow for at least the
    // minimum-damage amount instead of a confusing zero.
    assert_eq!(state.actor_stat(ACTOR_A_ID, "stamina"), 10);
    assert_eq!(state.actor_stat("bodyguard", "stamina"), 9);
    let output_text = transcript(&output.lines);
    assert!(
        output_text.contains("steps in front of you, taking 1 damage"),
        "expected guard intercept narration, got: {output_text}"
    );
    assert!(
        !output_text.contains("taking 0 damage"),
        "guard should never report a zero-damage intercept, got: {output_text}"
    );
}