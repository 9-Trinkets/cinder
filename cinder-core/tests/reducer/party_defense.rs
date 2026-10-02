use super::common::*;
use cinder_core::content::types::{
    ContentPack, PackMessage, PartyCandidatePriority, PartyCombatDecisionRule,
    PartyDecisionCondition, PartyDecisionTier, PartyPolicyDefinition, PartyReactionAction,
    PartyReactionCooldown, PartyReactionWindow, PartyTargetSelection, SkillDefinition,
};
use cinder_core::engine::events::{TimestampedWorldEvent, WorldEvent};
use cinder_core::engine::reducer::apply_events;
use cinder_core::engine::state::{ActorStance, WorldState};
use std::collections::BTreeMap;

#[test]
fn policy_selected_defender_takes_the_full_unsplit_strike_and_becomes_unready() {
    let mut pack = reducer_test_pack();
    pack.settings.combat.player_actor_id = ACTOR_A_ID.to_string();
    pack.settings.combat.health_stat_id = "stamina".to_string();
    pack.settings.combat.attack_stat_id = "confidence".to_string();
    pack.settings.combat.defense_stat_id = "hunger".to_string();
    pack.settings.party = PartyPolicyDefinition {
        initial_orders: BTreeMap::from([(ACTOR_B_ID.to_string(), "guard".to_string())]),
        combat_rules: vec![PartyCombatDecisionRule {
            id: "defender-intercepts".to_string(),
            tier: PartyDecisionTier::Order,
            window: PartyReactionWindow::BeforeHostileDamage,
            action: PartyReactionAction::Intercept,
            conditions: vec![PartyDecisionCondition::OrderIs {
                orders: vec!["guard".to_string()],
            }],
            target: PartyTargetSelection::Player,
            candidate_priority: vec![PartyCandidatePriority::HighestDefense],
            support_effect: None,
            cooldown: PartyReactionCooldown::FixedMinutes { minutes: 5 },
            message: "combat.guard_intercepts".to_string(),
        }],
    };
    pack.messages.insert(
        "combat.guard_intercepts".to_string(),
        PackMessage::Narration("{guard} blocks {actor} and takes {damage} damage.".to_string()),
    );
    let mut state = WorldState::new(&pack);
    state
        .actor_room_overrides
        .insert(ACTOR_C_ID.to_string(), LOUNGE_ID.to_string());
    state.set_stance(ACTOR_B_ID, ActorStance::Allied);
    state.set_stance(ACTOR_C_ID, ActorStance::Hostile);
    let player_before = state.actor_stat(ACTOR_A_ID, "stamina");
    let defender_before = state.actor_stat(ACTOR_B_ID, "stamina");

    let output = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::HostileStrike {
            actor_id: ACTOR_C_ID.to_string(),
        })],
    );

    assert_eq!(state.actor_stat(ACTOR_A_ID, "stamina"), player_before);
    assert!(state.actor_stat(ACTOR_B_ID, "stamina") < defender_before);
    assert_eq!(
        state.party_reaction_ready_at.get(ACTOR_B_ID),
        Some(&(state.current_time_minutes + 5))
    );
    assert_eq!(output.lines.len(), 1);
    assert!(output.lines[0].text.contains("Blair blocks Casey"));
}

#[test]
fn a_policy_with_no_ready_defender_falls_back_to_player_damage() {
    let mut pack = reducer_test_pack();
    pack.settings.combat.player_actor_id = ACTOR_A_ID.to_string();
    pack.settings.combat.health_stat_id = "stamina".to_string();
    pack.settings.combat.attack_stat_id = "confidence".to_string();
    pack.settings.combat.defense_stat_id = "hunger".to_string();
    pack.settings.party = PartyPolicyDefinition {
        initial_orders: BTreeMap::from([(ACTOR_B_ID.to_string(), "guard".to_string())]),
        combat_rules: vec![PartyCombatDecisionRule {
            id: "defender-intercepts".to_string(),
            tier: PartyDecisionTier::Order,
            window: PartyReactionWindow::BeforeHostileDamage,
            action: PartyReactionAction::Intercept,
            conditions: vec![PartyDecisionCondition::OrderIs {
                orders: vec!["guard".to_string()],
            }],
            target: PartyTargetSelection::Player,
            candidate_priority: vec![],
            support_effect: None,
            cooldown: PartyReactionCooldown::ActorCombatInterval,
            message: String::new(),
        }],
    };
    let mut state = WorldState::new(&pack);
    state
        .actor_room_overrides
        .insert(ACTOR_C_ID.to_string(), LOUNGE_ID.to_string());
    state.set_stance(ACTOR_B_ID, ActorStance::Allied);
    state.set_stance(ACTOR_C_ID, ActorStance::Hostile);
    state
        .party_reaction_ready_at
        .insert(ACTOR_B_ID.to_string(), state.current_time_minutes + 10);
    let player_before = state.actor_stat(ACTOR_A_ID, "stamina");
    let defender_before = state.actor_stat(ACTOR_B_ID, "stamina");

    apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::HostileStrike {
            actor_id: ACTOR_C_ID.to_string(),
        })],
    );

    assert!(state.actor_stat(ACTOR_A_ID, "stamina") < player_before);
    assert_eq!(state.actor_stat(ACTOR_B_ID, "stamina"), defender_before);
}

#[test]
fn follower_intercepts_strike_when_player_health_is_low() {
    let mut pack = reducer_test_pack();
    pack.settings.combat.player_actor_id = ACTOR_A_ID.to_string();
    pack.settings.combat.health_stat_id = "stamina".to_string();
    pack.settings.combat.attack_stat_id = "confidence".to_string();
    pack.settings.combat.defense_stat_id = "hunger".to_string();
    pack.settings.party = PartyPolicyDefinition {
        initial_orders: BTreeMap::from([(ACTOR_B_ID.to_string(), "follow".to_string())]),
        combat_rules: vec![
            PartyCombatDecisionRule {
                id: "guard-order".to_string(),
                tier: PartyDecisionTier::Order,
                window: PartyReactionWindow::BeforeHostileDamage,
                action: PartyReactionAction::Intercept,
                conditions: vec![PartyDecisionCondition::OrderIs {
                    orders: vec!["guard".to_string()],
                }],
                target: PartyTargetSelection::Player,
                candidate_priority: vec![],
                support_effect: None,
                cooldown: PartyReactionCooldown::ActorCombatInterval,
                message: "combat.guard_intercepts".to_string(),
            },
            PartyCombatDecisionRule {
                id: "follow-protect-player".to_string(),
                tier: PartyDecisionTier::Order,
                window: PartyReactionWindow::BeforeHostileDamage,
                action: PartyReactionAction::Intercept,
                conditions: vec![
                    PartyDecisionCondition::OrderIs {
                        orders: vec!["follow".to_string()],
                    },
                    PartyDecisionCondition::PlayerHealthAtMostPercent { percent: 50 },
                ],
                target: PartyTargetSelection::Player,
                candidate_priority: vec![],
                support_effect: None,
                cooldown: PartyReactionCooldown::ActorCombatInterval,
                message: "combat.guard_intercepts".to_string(),
            },
        ],
    };
    pack.messages.insert(
        "combat.guard_intercepts".to_string(),
        PackMessage::Narration(
            "{guard} steps in front of {actor} taking {damage} damage.".to_string(),
        ),
    );

    let mut state = WorldState::new(&pack);
    state
        .actor_room_overrides
        .insert(ACTOR_C_ID.to_string(), LOUNGE_ID.to_string());
    state.set_stance(ACTOR_B_ID, ActorStance::Allied);
    state.set_stance(ACTOR_C_ID, ActorStance::Hostile);

    state
        .initial_actor_stats
        .entry(ACTOR_A_ID.to_string())
        .or_default()
        .insert("stamina".to_string(), 10);
    state
        .actor_stats
        .entry(ACTOR_A_ID.to_string())
        .or_default()
        .insert("stamina".to_string(), 10);

    // 1. With player at full health (stamina 10/10 = 100%), follower does NOT intercept
    let player_before = state.actor_stat(ACTOR_A_ID, "stamina");
    let defender_before = state.actor_stat(ACTOR_B_ID, "stamina");

    apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::HostileStrike {
            actor_id: ACTOR_C_ID.to_string(),
        })],
    );

    assert!(state.actor_stat(ACTOR_A_ID, "stamina") < player_before);
    assert_eq!(state.actor_stat(ACTOR_B_ID, "stamina"), defender_before);

    // 2. Reduce player stamina to 4 (4/10 = 40% <= 50%) and advance time past attack interval
    state.current_time_minutes += 10;
    state
        .actor_stats
        .entry(ACTOR_A_ID.to_string())
        .or_default()
        .insert("stamina".to_string(), 4);
    let player_low = state.actor_stat(ACTOR_A_ID, "stamina");

    apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::HostileStrike {
            actor_id: ACTOR_C_ID.to_string(),
        })],
    );

    // Follower intercepted the blow: player stamina untouched, defender took damage!
    assert_eq!(state.actor_stat(ACTOR_A_ID, "stamina"), player_low);
    assert!(state.actor_stat(ACTOR_B_ID, "stamina") < defender_before);
}

/// Builds the same follow-protect-player policy the Layla pack ships.
fn intercept_policy() -> cinder_core::content::types::PartyPolicyDefinition {
    PartyPolicyDefinition {
        initial_orders: BTreeMap::from([(ACTOR_B_ID.to_string(), "follow".to_string())]),
        combat_rules: vec![PartyCombatDecisionRule {
            id: "follow-protect-player".to_string(),
            tier: PartyDecisionTier::Order,
            window: PartyReactionWindow::BeforeHostileDamage,
            action: PartyReactionAction::Intercept,
            conditions: vec![
                PartyDecisionCondition::OrderIs {
                    orders: vec!["follow".to_string()],
                },
                PartyDecisionCondition::PlayerHealthAtMostPercent { percent: 50 },
            ],
            target: PartyTargetSelection::Player,
            candidate_priority: vec![],
            support_effect: None,
            cooldown: PartyReactionCooldown::ActorCombatInterval,
            message: "combat.guard_intercepts".to_string(),
        }],
    }
}

/// Stages an allied follower, a hostile, and a player at 40% stamina, which is
/// the state in which `follow-protect-player` wants to intercept.
fn intercept_scenario(pack: &ContentPack) -> WorldState {
    let mut state = WorldState::new(pack);
    state
        .actor_room_overrides
        .insert(ACTOR_C_ID.to_string(), LOUNGE_ID.to_string());
    state.set_stance(ACTOR_B_ID, ActorStance::Allied);
    state.set_stance(ACTOR_C_ID, ActorStance::Hostile);
    state
        .initial_actor_stats
        .entry(ACTOR_A_ID.to_string())
        .or_default()
        .insert("stamina".to_string(), 10);
    state
        .actor_stats
        .entry(ACTOR_A_ID.to_string())
        .or_default()
        .insert("stamina".to_string(), 4);
    state
}

#[test]
fn a_follower_without_the_intercept_skill_does_not_intercept() {
    let mut pack = reducer_test_pack();
    pack.settings.combat.player_actor_id = ACTOR_A_ID.to_string();
    pack.settings.combat.health_stat_id = "stamina".to_string();
    pack.settings.combat.attack_stat_id = "confidence".to_string();
    pack.settings.combat.defense_stat_id = "hunger".to_string();
    pack.settings.party = intercept_policy();
    pack.messages.insert(
        "combat.guard_intercepts".to_string(),
        PackMessage::Narration("{guard} takes {damage}.".to_string()),
    );
    // The pack ships an intercept skill and the follower opts into the skills
    // system, but never learned it.
    pack.skills.skills = vec![SkillDefinition {
        id: "intercept".to_string(),
        label: "Intercept".to_string(),
        ..SkillDefinition::default()
    }];
    let follower = pack
        .actors
        .iter_mut()
        .find(|actor| actor.id == ACTOR_B_ID)
        .expect("follower exists");
    follower.skills = vec!["strike".to_string(), "hold".to_string()];
    rebuild_test_pack_indexes(&mut pack);

    let mut state = intercept_scenario(&pack);
    assert!(!state.actor_has_skill(ACTOR_B_ID, "intercept"));
    let player_before = state.actor_stat(ACTOR_A_ID, "stamina");
    let defender_before = state.actor_stat(ACTOR_B_ID, "stamina");

    apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::HostileStrike {
            actor_id: ACTOR_C_ID.to_string(),
        })],
    );

    // Without the skill the blow lands on the player and the follower is free.
    assert!(state.actor_stat(ACTOR_A_ID, "stamina") < player_before);
    assert_eq!(state.actor_stat(ACTOR_B_ID, "stamina"), defender_before);
}

#[test]
fn the_same_follower_intercepts_once_it_learns_the_skill() {
    let mut pack = reducer_test_pack();
    pack.settings.combat.player_actor_id = ACTOR_A_ID.to_string();
    pack.settings.combat.health_stat_id = "stamina".to_string();
    pack.settings.combat.attack_stat_id = "confidence".to_string();
    pack.settings.combat.defense_stat_id = "hunger".to_string();
    pack.settings.party = intercept_policy();
    pack.messages.insert(
        "combat.guard_intercepts".to_string(),
        PackMessage::Narration("{guard} takes {damage}.".to_string()),
    );
    pack.skills.skills = vec![SkillDefinition {
        id: "intercept".to_string(),
        label: "Intercept".to_string(),
        ..SkillDefinition::default()
    }];
    let follower = pack
        .actors
        .iter_mut()
        .find(|actor| actor.id == ACTOR_B_ID)
        .expect("follower exists");
    follower.skills = vec![
        "strike".to_string(),
        "hold".to_string(),
        "intercept".to_string(),
    ];
    rebuild_test_pack_indexes(&mut pack);

    let mut state = intercept_scenario(&pack);
    assert!(state.actor_has_skill(ACTOR_B_ID, "intercept"));
    let player_before = state.actor_stat(ACTOR_A_ID, "stamina");
    let defender_before = state.actor_stat(ACTOR_B_ID, "stamina");

    apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::HostileStrike {
            actor_id: ACTOR_C_ID.to_string(),
        })],
    );

    assert_eq!(state.actor_stat(ACTOR_A_ID, "stamina"), player_before);
    assert!(state.actor_stat(ACTOR_B_ID, "stamina") < defender_before);
}

#[test]
fn followers_outside_the_skills_system_keep_intercepting() {
    let mut pack = reducer_test_pack();
    pack.settings.combat.player_actor_id = ACTOR_A_ID.to_string();
    pack.settings.combat.health_stat_id = "stamina".to_string();
    pack.settings.combat.attack_stat_id = "confidence".to_string();
    pack.settings.combat.defense_stat_id = "hunger".to_string();
    pack.settings.party = intercept_policy();
    pack.messages.insert(
        "combat.guard_intercepts".to_string(),
        PackMessage::Narration("{guard} takes {damage}.".to_string()),
    );
    // Pack ships the skill, but the follower declares none: it stays on the
    // legacy rule-driven path. This is what keeps the golems and sprites that
    // hooks convert to allies behaving exactly as they did before.
    pack.skills.skills = vec![SkillDefinition {
        id: "intercept".to_string(),
        label: "Intercept".to_string(),
        ..SkillDefinition::default()
    }];
    let follower = pack
        .actors
        .iter_mut()
        .find(|actor| actor.id == ACTOR_B_ID)
        .expect("follower exists");
    follower.skills = vec![];
    rebuild_test_pack_indexes(&mut pack);

    let mut state = intercept_scenario(&pack);
    let player_before = state.actor_stat(ACTOR_A_ID, "stamina");
    let defender_before = state.actor_stat(ACTOR_B_ID, "stamina");

    apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::HostileStrike {
            actor_id: ACTOR_C_ID.to_string(),
        })],
    );

    assert_eq!(state.actor_stat(ACTOR_A_ID, "stamina"), player_before);
    assert!(state.actor_stat(ACTOR_B_ID, "stamina") < defender_before);
}
