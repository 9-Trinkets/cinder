use crate::content::types::{
    ContentPack, PartyDecisionTier, PartyReactionAction, PartyReactionCooldown,
    PartyReactionWindow, PartySupportEffect, PartyTargetSelection,
};
use crate::engine::combat_cooldowns::{CombatCooldownKind, actor_combat_cooldown_minutes};
use crate::engine::party_scope::{
    Candidate, PartyScope, actor_is_reaction_eligible, actor_may_use_reaction, compare_candidates,
    condition_matches, lowest_health_ally, party_rules, scope_for_protected,
};
use crate::engine::state::WorldState;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PartyReactionDecision {
    pub actor_id: String,
    pub rule_id: String,
    pub action: PartyReactionAction,
    pub target: PartyTargetSelection,
    /// Actor the acting party protects (its leader, or the protected actor when
    /// the party has no authored leader). Resolves [`PartyTargetSelection::Leader`].
    pub leader_id: String,
    pub support_effect: Option<PartySupportEffect>,
    pub message: String,
    pub cooldown: PartyReactionCooldown,
}

/// Selects the party member that steps in to intercept a blow aimed at the
/// party leader. Player parties keep the player as their implicit leader; an
/// AI party shields its authored leader.
pub(crate) fn select_defensive_reaction(
    content: &ContentPack,
    state: &WorldState,
    protected_id: &str,
) -> Option<PartyReactionDecision> {
    let scope = scope_for_protected(content, state, protected_id)?;
    if protected_id != scope.leader_id {
        return None;
    }
    let room_id = state.actor_current_room_id(content, protected_id);
    for tier in PartyDecisionTier::EVALUATION_ORDER {
        for rule in party_rules(content, &scope.party_id).iter().filter(|rule| {
            rule.tier == tier
                && rule.window == PartyReactionWindow::BeforeHostileDamage
                && rule.action == PartyReactionAction::Intercept
        }) {
            let mut candidates = scope
                .members
                .iter()
                .enumerate()
                .filter(|(_, actor_id)| {
                    actor_id.as_str() != protected_id
                        && state.actor_is_in_room(content, actor_id, &room_id)
                        && !state
                            .actor_is_defeated(actor_id, &content.settings.combat.health_stat_id)
                        && state
                            .relations
                            .party_reaction_ready_at
                            .get(actor_id.as_str())
                            .is_none_or(|ready_at| *ready_at <= state.clock.current_time_minutes)
                        && actor_may_use_reaction(state, actor_id, &rule.skill_id)
                        && rule.conditions.iter().all(|condition| {
                            condition_matches(content, state, &scope, actor_id, &room_id, condition)
                        })
                })
                .map(|(content_index, actor_id)| Candidate {
                    actor_id: actor_id.as_str(),
                    content_index,
                })
                .collect::<Vec<_>>();
            candidates.sort_by(|left, right| compare_candidates(content, state, rule, left, right));
            if let Some(candidate) = candidates.first() {
                return Some(PartyReactionDecision {
                    actor_id: candidate.actor_id.to_string(),
                    rule_id: rule.id.clone(),
                    action: rule.action,
                    target: rule.target,
                    leader_id: scope.leader_id.clone(),
                    support_effect: rule.support_effect.clone(),
                    message: rule.message.clone(),
                    cooldown: rule.cooldown,
                });
            }
        }
    }
    None
}

/// Selects post-damage reactions for the player party and, when the struck
/// actor belongs to an AI-driven party, for that party too.
pub(crate) fn select_post_damage_reactions(
    content: &ContentPack,
    state: &WorldState,
    room_id: &str,
    protected_id: &str,
) -> Vec<PartyReactionDecision> {
    let mut scopes: Vec<PartyScope> = Vec::new();
    if let Some(scope) =
        scope_for_protected(content, state, &content.settings.combat.player_actor_id)
    {
        scopes.push(scope);
    }
    if let Some(scope) = scope_for_protected(content, state, protected_id)
        && !scopes
            .iter()
            .any(|existing| existing.party_id == scope.party_id)
    {
        scopes.push(scope);
    }
    let mut decisions = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for scope in &scopes {
        for actor_id in &scope.members {
            if actor_id == &content.settings.combat.player_actor_id || seen.contains(actor_id) {
                continue;
            }
            if !actor_is_reaction_eligible(content, state, actor_id, room_id) {
                continue;
            }
            let rule = PartyDecisionTier::EVALUATION_ORDER
                .into_iter()
                .find_map(|tier| {
                    party_rules(content, &scope.party_id).iter().find(|rule| {
                        rule.tier == tier
                            && rule.window == PartyReactionWindow::AfterHostileDamage
                            && matches!(
                                rule.action,
                                PartyReactionAction::Counterattack | PartyReactionAction::Support
                            )
                            && rule.conditions.iter().all(|condition| {
                                condition_matches(
                                    content, state, scope, actor_id, room_id, condition,
                                )
                            })
                            && actor_may_use_reaction(state, actor_id, &rule.skill_id)
                    })
                });
            if let Some(rule) = rule {
                seen.insert(actor_id.clone());
                decisions.push(PartyReactionDecision {
                    actor_id: actor_id.clone(),
                    rule_id: rule.id.clone(),
                    action: rule.action,
                    target: rule.target,
                    leader_id: scope.leader_id.clone(),
                    support_effect: rule.support_effect.clone(),
                    message: rule.message.clone(),
                    cooldown: rule.cooldown,
                });
            }
        }
    }
    decisions
}

pub(crate) fn consume_party_reaction(
    content: &ContentPack,
    state: &mut WorldState,
    decision: &PartyReactionDecision,
) {
    let minutes = match decision.cooldown {
        PartyReactionCooldown::ActorCombatInterval => {
            let kind = if decision.action == PartyReactionAction::Support {
                CombatCooldownKind::Spell
            } else {
                CombatCooldownKind::Physical
            };
            actor_combat_cooldown_minutes(content, state, &decision.actor_id, kind)
        }
        PartyReactionCooldown::FixedMinutes { minutes } => minutes,
    };
    state.relations.party_reaction_ready_at.insert(
        decision.actor_id.clone(),
        state.clock.current_time_minutes.saturating_add(minutes),
    );
}

pub(crate) fn resolve_party_reaction_target(
    content: &ContentPack,
    state: &WorldState,
    decision: &PartyReactionDecision,
    attacker_id: &str,
    room_id: &str,
) -> Option<String> {
    match decision.target {
        PartyTargetSelection::SelfActor => Some(decision.actor_id.clone()),
        PartyTargetSelection::Player => Some(content.settings.combat.player_actor_id.clone()),
        PartyTargetSelection::Leader => {
            Some(decision.leader_id.clone()).filter(|leader| !leader.is_empty())
        }
        PartyTargetSelection::Attacker => Some(attacker_id.to_string()),
        PartyTargetSelection::LowestHealthAlly => {
            lowest_health_ally(content, state, &decision.actor_id, room_id)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::types::{
        PartyCandidatePriority, PartyCombatDecisionRule, PartyDecisionCondition, PartyDefinition,
    };
    use crate::engine::state::ActorStance;
    use crate::engine::test_fixtures::{minimal_test_pack, rebuild_test_pack_indexes};

    fn defensive_pack() -> ContentPack {
        let mut content = minimal_test_pack();
        content.settings.combat.player_actor_id = "blair".to_string();
        content.settings.combat.health_stat_id = "stamina".to_string();
        content.settings.combat.defense_stat_id = "confidence".to_string();
        content.settings.parties = vec![PartyDefinition {
            id: "player".to_string(),
            initial_orders: std::collections::BTreeMap::from([
                ("casey".to_string(), "guard".to_string()),
                ("drew".to_string(), "guard".to_string()),
            ]),
            combat_rules: vec![PartyCombatDecisionRule {
                id: "guard-order".to_string(),
                skill_id: String::new(),
                tier: PartyDecisionTier::Order,
                window: PartyReactionWindow::BeforeHostileDamage,
                action: PartyReactionAction::Intercept,
                conditions: vec![PartyDecisionCondition::OrderIs {
                    orders: vec!["guard".to_string()],
                }],
                target: PartyTargetSelection::Player,
                candidate_priority: vec![
                    PartyCandidatePriority::HighestHealthPercent,
                    PartyCandidatePriority::HighestDefense,
                ],
                support_effect: None,
                cooldown: PartyReactionCooldown::ActorCombatInterval,
                message: "combat.guard".to_string(),
            }],
            ..PartyDefinition::default()
        }];
        let mut drew = content.actor("casey").unwrap().clone();
        drew.id = "drew".to_string();
        drew.name = "Drew".to_string();
        drew.room_id = "lounge".to_string();
        content.actors.push(drew);
        rebuild_test_pack_indexes(&mut content);
        content
    }

    fn allied_state(content: &ContentPack) -> WorldState {
        let mut state = WorldState::new(content);
        state
            .actors
            .actor_room_overrides
            .insert("casey".to_string(), "lounge".to_string());
        for actor_id in ["casey", "drew"] {
            state.set_stance(actor_id, ActorStance::Allied);
        }
        state
    }

    #[test]
    fn guard_order_uses_priority_and_consumes_readiness() {
        let content = defensive_pack();
        let mut state = allied_state(&content);
        state
            .adjust_actor_stat(&content, "casey", "stamina", -5)
            .unwrap();
        state
            .assign_party_order(&content, "casey", "guard".to_string())
            .unwrap();
        let decision = select_defensive_reaction(&content, &state, "blair").unwrap();
        assert_eq!(decision.actor_id, "drew");
        assert_eq!(decision.rule_id, "guard-order");
        consume_party_reaction(&content, &mut state, &decision);
        let next = select_defensive_reaction(&content, &state, "blair").unwrap();
        assert_eq!(next.actor_id, "casey");
    }

    #[test]
    fn assist_order_suppresses_guard_behavior() {
        let content = defensive_pack();
        let mut state = allied_state(&content);
        state
            .assign_party_order(&content, "drew", "assist".to_string())
            .unwrap();
        let decision = select_defensive_reaction(&content, &state, "blair").unwrap();
        assert_eq!(decision.actor_id, "casey");
    }

    #[test]
    fn ai_party_guard_intercepts_for_its_leader() {
        let mut content = defensive_pack();
        content.settings.combat.health_stat_id = "stamina".to_string();
        let mut king = content.actor("casey").unwrap().clone();
        king.id = "elf-king".to_string();
        king.name = "Elf King".to_string();
        king.room_id = "lounge".to_string();
        content.actors.push(king);
        content.settings.parties = vec![PartyDefinition {
            id: "elf-army".to_string(),
            leader: Some("elf-king".to_string()),
            members: vec!["elf-king".to_string(), "casey".to_string()],
            initial_orders: std::collections::BTreeMap::from([(
                "casey".to_string(),
                "guard".to_string(),
            )]),
            combat_rules: vec![PartyCombatDecisionRule {
                id: "guard-leader".to_string(),
                skill_id: String::new(),
                tier: PartyDecisionTier::Order,
                window: PartyReactionWindow::BeforeHostileDamage,
                action: PartyReactionAction::Intercept,
                conditions: vec![PartyDecisionCondition::OrderIs {
                    orders: vec!["guard".to_string()],
                }],
                target: PartyTargetSelection::Leader,
                candidate_priority: vec![PartyCandidatePriority::ContentOrder],
                support_effect: None,
                cooldown: PartyReactionCooldown::ActorCombatInterval,
                message: "combat.guard".to_string(),
            }],
            ..PartyDefinition::default()
        }];
        rebuild_test_pack_indexes(&mut content);
        let mut state = WorldState::new(&content);
        for actor_id in ["elf-king", "casey"] {
            state
                .actors
                .actor_room_overrides
                .insert(actor_id.to_string(), "lounge".to_string());
        }
        state.set_authored_party_order("casey", "guard".to_string());

        let decision = select_defensive_reaction(&content, &state, "elf-king").unwrap();
        assert_eq!(decision.actor_id, "casey");
        assert_eq!(decision.leader_id, "elf-king");
        let target =
            resolve_party_reaction_target(&content, &state, &decision, "attacker", "lounge");
        assert_eq!(target, Some("elf-king".to_string()));
    }
}
