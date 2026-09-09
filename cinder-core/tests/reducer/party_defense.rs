use super::common::*;
use cinder_core::content::types::{
    PackMessage, PartyCandidatePriority, PartyCombatDecisionRule, PartyDecisionCondition,
    PartyDecisionTier, PartyPolicyDefinition, PartyReactionAction, PartyReactionCooldown,
    PartyReactionWindow, PartyRoleDefinition, PartyTargetSelection,
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
        roles: vec![PartyRoleDefinition {
            id: "defender".to_string(),
        }],
        actor_roles: BTreeMap::from([(ACTOR_B_ID.to_string(), vec!["defender".to_string()])]),
        combat_rules: vec![PartyCombatDecisionRule {
            id: "defender-intercepts".to_string(),
            tier: PartyDecisionTier::DefaultRole,
            window: PartyReactionWindow::BeforeHostileDamage,
            action: PartyReactionAction::Intercept,
            conditions: vec![PartyDecisionCondition::HasRole {
                role_id: "defender".to_string(),
            }],
            target: PartyTargetSelection::Player,
            candidate_priority: vec![PartyCandidatePriority::HighestDefense],
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
        roles: vec![PartyRoleDefinition {
            id: "defender".to_string(),
        }],
        actor_roles: BTreeMap::from([(ACTOR_B_ID.to_string(), vec!["defender".to_string()])]),
        combat_rules: vec![PartyCombatDecisionRule {
            id: "defender-intercepts".to_string(),
            tier: PartyDecisionTier::DefaultRole,
            window: PartyReactionWindow::BeforeHostileDamage,
            action: PartyReactionAction::Intercept,
            conditions: vec![PartyDecisionCondition::HasRole {
                role_id: "defender".to_string(),
            }],
            target: PartyTargetSelection::Player,
            candidate_priority: vec![],
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
