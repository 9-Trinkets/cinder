//! Layla combat integration tests, driven through the public engine API.

use cinder_core::content::loader::load_named_pack;
use cinder_core::content::types::{ContentPack, ContentSettingsDefinition};
use cinder_core::engine::events::{ObservationMode, TimestampedWorldEvent, WorldEvent};
use cinder_core::engine::reducer::apply_events;
use cinder_core::engine::state::{ActorRelationship, ActorStance, GamePhase, WorldState};

fn layla_test_pack() -> ContentPack {
    let mut pack = load_named_pack("layla", Some("en")).expect("load layla");
    pack.settings = ContentSettingsDefinition {
        tick_minutes_per_turn: 1,
        ..ContentSettingsDefinition::default()
    };
    pack.opening.start_room_id = "r1c1".to_string();
    pack.opening.start_room_ids = vec!["r1c1".to_string()];
    pack
}

fn layla_test_state() -> (ContentPack, WorldState) {
    let content = layla_test_pack();
    let mut state = WorldState::new(&content);
    state.current_room_id = "r1c1".to_string();
    (content, state)
}

fn actors_present_in_room<'a>(
    content: &'a ContentPack,
    state: &WorldState,
    room_id: &str,
) -> Vec<&'a str> {
    content
        .actors
        .iter()
        .filter(|actor| {
            !state.actor_is_defeated(&actor.id)
                && state.actor_room_id(&actor.id, &actor.room_id) == room_id
        })
        .map(|actor| actor.id.as_str())
        .collect()
}

fn layla_attack_event(room_id: &str, target_actor_id: &str) -> TimestampedWorldEvent {
    TimestampedWorldEvent::now(WorldEvent::ActorCommandUsed {
        actor_id: "player".to_string(),
        actor_name: "Layla".to_string(),
        room_id: room_id.to_string(),
        command_id: "attack".to_string(),
        target_room_id: None,
        target_actor_id: Some(target_actor_id.to_string()),
        target_actor_name: Some("Golem".to_string()),
        context_label: None,
        feature_id: None,
        consumable_id: None,
        freeform_text: None,
    })
}

fn layla_turn_started() -> TimestampedWorldEvent {
    TimestampedWorldEvent::now(WorldEvent::TurnStarted {
        turn_number: 1,
        raw_input: "look".to_string(),
        advances_time: false,
    })
}

fn layla_background_tick(turn_number: u32) -> TimestampedWorldEvent {
    TimestampedWorldEvent::now(WorldEvent::TurnStarted {
        turn_number,
        raw_input: "tick".to_string(),
        advances_time: true,
    })
}

#[test]
fn layla_tick_strikes_respect_attack_cooldown() {
    let (content, mut state) = layla_test_state();
    state.current_room_id = "r3c3".to_string();
    state.set_stance("golem-dark-nw", ActorStance::Hostile);
    // Cooldown still in the future relative to the time this tick advances to.
    // The game clock starts at `start_time_minutes`, so seed relative to it.
    let next_strike = state.current_time_minutes + 5;
    state
        .next_hostile_strike_at
        .insert("golem-dark-nw".to_string(), next_strike);

    let output = apply_events(&mut state, &content, &[layla_background_tick(2)]);

    assert_eq!(
        state.actor_stat("player", "hp"),
        10,
        "a golem whose attack cooldown has not elapsed must not strike"
    );
    assert!(!output.lines.iter().any(|l| l.contains("strikes you")));
}

#[test]
fn layla_place_flag_adds_to_flagged_rooms() {
    let (content, mut state) = layla_test_state();
    state.current_room_id = "r4c4".to_string();

    let output = apply_events(
        &mut state,
        &content,
        &[TimestampedWorldEvent::now(WorldEvent::ActorCommandUsed {
            actor_id: "player".to_string(),
            actor_name: "Layla".to_string(),
            room_id: "r4c4".to_string(),
            command_id: "place_flag".to_string(),
            target_room_id: None,
            target_actor_id: None,
            target_actor_name: None,
            context_label: None,
            feature_id: None,
            consumable_id: None,
            freeform_text: None,
        })],
    );

    assert!(state.flagged_rooms.contains("r4c4"));
    assert!(output.lines.iter().any(|l| l.contains("marker")));
}

#[test]
fn layla_flag_supply_decrements_on_place_and_refills_on_pickup() {
    let (mut content, mut state) = layla_test_state();
    content.settings.flag_supply = 2;
    state.flags_remaining = content.settings.flag_supply;

    fn flag_event(room_id: &str, command_id: &str) -> TimestampedWorldEvent {
        TimestampedWorldEvent::now(WorldEvent::ActorCommandUsed {
            actor_id: "player".to_string(),
            actor_name: "Layla".to_string(),
            room_id: room_id.to_string(),
            command_id: command_id.to_string(),
            target_room_id: None,
            target_actor_id: None,
            target_actor_name: None,
            context_label: None,
            feature_id: None,
            consumable_id: None,
            freeform_text: None,
        })
    }

    apply_events(
        &mut state,
        &content,
        &[flag_event("r4c4", "place_flag"), flag_event("r3c3", "place_flag")],
    );
    assert_eq!(
        state.flags_remaining, 0,
        "each placement consumes one marker"
    );

    // Realization backstop: placing with an empty pool must be refused.
    let output = apply_events(&mut state, &content, &[flag_event("r2c2", "place_flag")]);
    assert!(!state.flagged_rooms.contains("r2c2"));
    assert!(
        output.lines.is_empty(),
        "exhausted supply must produce no effects, got: {:?}",
        output.lines
    );

    apply_events(&mut state, &content, &[flag_event("r4c4", "pick_up_flag")]);
    assert_eq!(state.flags_remaining, 1, "picked-up markers return to the pool");
}

#[test]
fn layla_pick_up_flag_removes_from_flagged_rooms() {
    let (content, mut state) = layla_test_state();
    state.current_room_id = "r4c4".to_string();
    state.flagged_rooms.insert("r4c4".to_string());

    let output = apply_events(
        &mut state,
        &content,
        &[TimestampedWorldEvent::now(WorldEvent::ActorCommandUsed {
            actor_id: "player".to_string(),
            actor_name: "Layla".to_string(),
            room_id: "r4c4".to_string(),
            command_id: "pick_up_flag".to_string(),
            target_room_id: None,
            target_actor_id: None,
            target_actor_name: None,
            context_label: None,
            feature_id: None,
            consumable_id: None,
            freeform_text: None,
        })],
    );

    assert!(!state.flagged_rooms.contains("r4c4"));
    assert!(
        output
            .lines
            .iter()
            .any(|l| l.contains("pull") || l.contains("marker")),
        "expected flag removal message, got: {:?}",
        output.lines
    );
}

#[test]
fn layla_attack_deals_damage_to_golem_and_wakes_it() {
    let (content, mut state) = layla_test_state();
    state.current_room_id = "r3c3".to_string();
    state
        .actor_stats
        .entry("golem-dark-nw".to_string())
        .or_default()
        .insert("hp".to_string(), 8);

    let output = apply_events(
        &mut state,
        &content,
        &[layla_attack_event("r3c3", "golem-dark-nw")],
    );

    let golem_hp = state
        .actor_stats
        .get("golem-dark-nw")
        .and_then(|s| s.get("hp"))
        .copied()
        .unwrap_or(8);
    assert!(
        golem_hp < 8,
        "golem should have taken damage, hp={}",
        golem_hp
    );
    assert!(output.lines.iter().any(|l| l.contains("damage")));
    // Regression: the player must keep the stats.json default HP, not die instantly.
    assert_eq!(state.actor_stat("player", "hp"), 10);
    assert_eq!(state.phase, GamePhase::Active);
    assert!(
        state.stance("golem-dark-nw") == ActorStance::Hostile,
        "surviving golem should wake hostile"
    );
}

#[test]
fn layla_slain_golem_does_not_become_hostile() {
    let (content, mut state) = layla_test_state();
    state.current_room_id = "r3c3".to_string();
    state
        .actor_stats
        .entry("golem-dark-nw".to_string())
        .or_default()
        .insert("hp".to_string(), 2);

    let output = apply_events(
        &mut state,
        &content,
        &[layla_attack_event("r3c3", "golem-dark-nw")],
    );

    assert!(
        output.lines.iter().any(|l| l.contains("crumbles")),
        "expected kill message, got: {:?}",
        output.lines
    );
    assert!(
        state.stance("golem-dark-nw") != ActorStance::Hostile,
        "slain golem must not turn hostile"
    );
}

#[test]
fn layla_hostile_golem_strikes_on_due_tick() {
    let (content, mut state) = layla_test_state();
    state.current_room_id = "r3c3".to_string();
    state.set_stance("golem-dark-nw", ActorStance::Hostile);
    state
        .next_hostile_strike_at
        .insert("golem-dark-nw".to_string(), 0);

    let output = apply_events(&mut state, &content, &[layla_background_tick(2)]);

    assert!(
        state.actor_stat("player", "hp") < 10,
        "hostile golem sharing the room should strike once its cooldown elapses"
    );
    assert!(
        output.lines.iter().any(|l| l.contains("strikes you")),
        "expected mob strike line, got: {:?}",
        output.lines
    );
}

#[test]
fn layla_player_command_turns_do_not_trigger_strikes() {
    let (content, mut state) = layla_test_state();
    state.current_room_id = "r3c3".to_string();
    state.set_stance("golem-dark-nw", ActorStance::Hostile);
    state
        .next_hostile_strike_at
        .insert("golem-dark-nw".to_string(), 0);

    let output = apply_events(&mut state, &content, &[layla_turn_started()]);

    assert_eq!(
        state.actor_stat("player", "hp"),
        10,
        "strikes are autonomous tick events, not same-turn retaliation"
    );
    assert!(!output.lines.iter().any(|l| l.contains("strikes you")));
}

#[test]
fn layla_distant_hostile_golem_leaves_player_untouched() {
    let (content, mut state) = layla_test_state();
    state.current_room_id = "r5c5".to_string();
    state.set_stance("golem-dark-nw", ActorStance::Hostile);
    state
        .next_hostile_strike_at
        .insert("golem-dark-nw".to_string(), 0);

    let output = apply_events(&mut state, &content, &[layla_background_tick(2)]);

    assert_eq!(
        state.actor_stat("player", "hp"),
        10,
        "hostile golem in another room must not reach the player"
    );
    assert!(!output.lines.iter().any(|l| l.contains("strikes you")));
}

#[test]
fn layla_woken_golem_kills_player_if_ignored() {
    let (content, mut state) = layla_test_state();
    state.current_room_id = "r3c3".to_string();
    state.set_stance("golem-dark-nw", ActorStance::Hostile);
    state
        .actor_stats
        .entry("player".to_string())
        .or_default()
        .insert("hp".to_string(), 1);
    state
        .next_hostile_strike_at
        .insert("golem-dark-nw".to_string(), 0);

    let output = apply_events(&mut state, &content, &[layla_background_tick(2)]);

    assert_eq!(
        state.phase,
        GamePhase::GameEnded,
        "ignoring a woken golem should be lethal"
    );
    assert!(
        output.lines.iter().any(|l| l.contains("world tilts")),
        "expected defeat line, got: {:?}",
        output.lines
    );
}

#[test]
fn layla_waking_golem_gets_grace_window_before_first_strike() {
    let (content, mut state) = layla_test_state();
    state.current_room_id = "r3c3".to_string();
    state
        .actor_stats
        .entry("golem-dark-nw".to_string())
        .or_default()
        .insert("hp".to_string(), 8);

    // Wake the golem with an attack; it must not retaliate this turn.
    let output = apply_events(
        &mut state,
        &content,
        &[layla_attack_event("r3c3", "golem-dark-nw")],
    );
    assert!(
        output.lines.iter().any(|l| l.contains("eyes snap open")),
        "expected wake line, got: {:?}",
        output.lines
    );
    assert_eq!(
        state.actor_stat("player", "hp"),
        10,
        "a freshly woken golem must not strike immediately"
    );

    // One tick later the cooldown (wake time + interval) has not elapsed yet.
    let output = apply_events(&mut state, &content, &[layla_background_tick(2)]);
    assert_eq!(
        state.actor_stat("player", "hp"),
        10,
        "grace window must cover the first tick after waking"
    );
    assert!(!output.lines.iter().any(|l| l.contains("strikes you")));
}

#[test]
fn layla_encirclement_converts_golem() {
    let (content, mut state) = layla_test_state();
    state.current_room_id = "r4c3".to_string();
    state
        .actor_stats
        .entry("golem-dark-nw".to_string())
        .or_default()
        .insert("hp".to_string(), 8);

    let neighbors = ["r2c3", "r3c2", "r3c4"];
    for n in &neighbors {
        state.flagged_rooms.insert(n.to_string());
    }

    let output = apply_events(
        &mut state,
        &content,
        &[TimestampedWorldEvent::now(WorldEvent::ActorCommandUsed {
            actor_id: "player".to_string(),
            actor_name: "Layla".to_string(),
            room_id: "r4c3".to_string(),
            command_id: "place_flag".to_string(),
            target_room_id: None,
            target_actor_id: None,
            target_actor_name: None,
            context_label: None,
            feature_id: None,
            consumable_id: None,
            freeform_text: None,
        })],
    );

    assert!(
        output
            .lines
            .iter()
            .any(|l| l.contains("shudders") || l.contains("follow")),
        "expected conversion message, got: {:?}",
        output.lines
    );
    assert_eq!(
        state.followed_actor_id, None,
        "conversion must not flip the player-follows-actor relationship"
    );
    assert!(
        state.follows_player("golem-dark-nw"),
        "converted golem should follow the player"
    );
    assert!(
        state.stance("golem-dark-nw") == ActorStance::Allied,
        "converted golem should count as an ally"
    );
}

#[test]
fn layla_follower_relocates_when_player_moves() {
    let (content, mut state) = layla_test_state();
    state.current_room_id = "r3c3".to_string();
    state.set_follows_player("golem-dark-nw", true);
    state
        .actor_room_overrides
        .insert("golem-dark-nw".to_string(), "r3c3".to_string());

    let output = apply_events(
        &mut state,
        &content,
        &[TimestampedWorldEvent::now(WorldEvent::PlayerMoved {
            from_room_id: "r3c3".to_string(),
            to_room_id: "r4c3".to_string(),
        })],
    );

    assert_eq!(
        state
            .actor_room_overrides
            .get("golem-dark-nw")
            .map(String::as_str),
        Some("r4c3"),
        "converted golem must move along with the player"
    );
    assert!(
        output.lines.iter().any(|l| l.contains("follows you")),
        "expected follow line, got: {:?}",
        output.lines
    );
}

#[test]
fn layla_encirclement_skips_hostile_golem() {
    let (content, mut state) = layla_test_state();
    state.current_room_id = "r3c4".to_string();
    state.set_stance("golem-dark-nw", ActorStance::Hostile);

    let neighbors = ["r2c3", "r3c4", "r4c3"];
    for n in &neighbors {
        state.flagged_rooms.insert(n.to_string());
    }

    apply_events(
        &mut state,
        &content,
        &[TimestampedWorldEvent::now(WorldEvent::ActorCommandUsed {
            actor_id: "player".to_string(),
            actor_name: "Layla".to_string(),
            room_id: "r3c2".to_string(),
            command_id: "place_flag".to_string(),
            target_room_id: None,
            target_actor_id: None,
            target_actor_name: None,
            context_label: None,
            feature_id: None,
            consumable_id: None,
            freeform_text: None,
        })],
    );

    assert!(
        state.stance("golem-dark-nw") != ActorStance::Allied
            && !state.follows_player("golem-dark-nw"),
        "a woken hostile golem must not convert through encirclement"
    );
}

#[test]
fn layla_cannot_attack_dead_golem() {
    let (content, mut state) = layla_test_state();
    state.current_room_id = "r3c3".to_string();
    state
        .actor_stats
        .entry("golem-dark-nw".to_string())
        .or_default()
        .insert("hp".to_string(), 0);

    let output = apply_events(
        &mut state,
        &content,
        &[TimestampedWorldEvent::now(WorldEvent::ActorCommandUsed {
            actor_id: "player".to_string(),
            actor_name: "Layla".to_string(),
            room_id: "r3c3".to_string(),
            command_id: "attack".to_string(),
            target_room_id: None,
            target_actor_id: Some("golem-dark-nw".to_string()),
            target_actor_name: Some("Dark Golem".to_string()),
            context_label: None,
            feature_id: None,
            consumable_id: None,
            freeform_text: None,
        })],
    );

    assert!(
        output.lines.is_empty(),
        "attack on dead golem should produce no output, got: {:?}",
        output.lines
    );
}

#[test]
fn layla_defeated_golem_vanishes_from_room_presence() {
    let (content, mut state) = layla_test_state();
    state.current_room_id = "r3c3".to_string();
    state
        .actor_stats
        .entry("golem-dark-nw".to_string())
        .or_default()
        .insert("hp".to_string(), 1);
    // Allies can no longer be attacked, so the killable case is a hostile.
    state.set_stance("golem-dark-nw", ActorStance::Hostile);

    apply_events(
        &mut state,
        &content,
        &[layla_attack_event("r3c3", "golem-dark-nw")],
    );

    assert!(state.actor_is_defeated("golem-dark-nw"));
    assert!(
        actors_present_in_room(&content, &state, "r3c3").is_empty(),
        "defeated golem must not be listed among actors in the room"
    );
    assert_eq!(
        state.relationship("golem-dark-nw"),
        ActorRelationship::default(),
        "defeat must clear the relationship entirely"
    );

    // Attacking the corpse again must be rejected outright.
    let before = state.turn_number;
    let output = apply_events(
        &mut state,
        &content,
        &[layla_attack_event("r3c3", "golem-dark-nw")],
    );
    assert!(
        output.lines.is_empty() && state.turn_number == before,
        "attack on defeated golem should produce no output, got: {:?}",
        output.lines
    );
}

#[test]
fn layla_duplicate_golems_group_in_room_presence() {
    let (content, mut state) = layla_test_state();
    // Two dark golems share r3c3: the nw spawn plus the sw golem relocated in.
    state
        .actor_room_overrides
        .insert("golem-dark-sw".to_string(), "r3c3".to_string());

    let output = apply_events(
        &mut state,
        &content,
        &[TimestampedWorldEvent::now(
            WorldEvent::CurrentRoomObserved {
                room_id: "r3c3".to_string(),
                mode: ObservationMode::Summary,
            },
        )],
    );

    assert!(
        output
            .lines
            .iter()
            .any(|line| line.contains("dark golem ×2")),
        "duplicate names should group with a count, got: {:?}",
        output.lines
    );
}

#[test]
fn layla_allies_boost_damage_and_hostiles_do_not() {
    let (content, mut state) = layla_test_state();
    state.current_room_id = "r3c3".to_string();
    state
        .actor_stats
        .entry("golem-dark-nw".to_string())
        .or_default()
        .insert("hp".to_string(), 18);
    state
        .actor_stats
        .entry("golem-pale-ne".to_string())
        .or_default()
        .insert("strength".to_string(), 3);
    state.set_stance("golem-pale-ne", ActorStance::Allied);
    // The ally must share the attack room to contribute.
    state
        .actor_room_overrides
        .insert("golem-pale-ne".to_string(), "r3c3".to_string());
    // A hostile golem's strength must never add to the player's attacks.
    state
        .actor_stats
        .entry("golem-pale-se".to_string())
        .or_default()
        .insert("strength".to_string(), 9);
    state.set_stance("golem-pale-se", ActorStance::Hostile);

    let output = apply_events(
        &mut state,
        &content,
        &[layla_attack_event("r3c3", "golem-dark-nw")],
    );

    let golem_hp = state
        .actor_stats
        .get("golem-dark-nw")
        .and_then(|s| s.get("hp"))
        .copied()
        .unwrap_or(18);
    assert_eq!(
        golem_hp, 13,
        "expected exactly player (2) + one ally (3) damage, hp={}",
        golem_hp
    );
    assert!(
        output
            .lines
            .iter()
            .any(|l| l.contains("strikes alongside") && l.contains("for 3 damage")),
        "expected ally join narration with its damage contribution, got: {:?}",
        output.lines
    );
}

#[test]
fn layla_distant_allies_do_not_boost_damage() {
    let (content, mut state) = layla_test_state();
    state.current_room_id = "r3c3".to_string();
    state
        .actor_stats
        .entry("golem-dark-nw".to_string())
        .or_default()
        .insert("hp".to_string(), 18);
    // Allied but in its home room (r3c7), far from the attack at r3c3.
    state
        .actor_stats
        .entry("golem-pale-ne".to_string())
        .or_default()
        .insert("strength".to_string(), 9);
    state.set_stance("golem-pale-ne", ActorStance::Allied);

    apply_events(
        &mut state,
        &content,
        &[layla_attack_event("r3c3", "golem-dark-nw")],
    );

    let golem_hp = state
        .actor_stats
        .get("golem-dark-nw")
        .and_then(|s| s.get("hp"))
        .copied()
        .unwrap_or(18);
    assert_eq!(
        golem_hp, 16,
        "an ally outside the attack room must not add damage, hp={}",
        golem_hp
    );
}

#[test]
fn layla_cannot_attack_an_ally() {
    let (content, mut state) = layla_test_state();
    state.current_room_id = "r3c3".to_string();
    state
        .actor_stats
        .entry("golem-dark-nw".to_string())
        .or_default()
        .insert("hp".to_string(), 8);
    state.set_stance("golem-dark-nw", ActorStance::Allied);

    let output = apply_events(
        &mut state,
        &content,
        &[layla_attack_event("r3c3", "golem-dark-nw")],
    );

    let golem_hp = state
        .actor_stats
        .get("golem-dark-nw")
        .and_then(|s| s.get("hp"))
        .copied()
        .unwrap_or(8);
    assert_eq!(golem_hp, 8, "allies must never take damage from attacks");
    assert!(
        output.lines.is_empty(),
        "attacking an ally should be refused silently at the mechanism level, got: {:?}",
        output.lines
    );
}
