use crate::content::types::{
    ContentPack, PartyCandidatePriority, PartyDecisionCondition, PartyDecisionTier, PartyOrderKind,
    PartyReactionAction, PartyReactionCooldown, PartyReactionWindow,
};
use crate::engine::state::{ActorStance, WorldState};
use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PartyReactionDecision {
    pub actor_id: String,
    pub rule_id: String,
    pub message: String,
    pub cooldown: PartyReactionCooldown,
}

pub(crate) fn select_defensive_reaction(
    content: &ContentPack,
    state: &WorldState,
) -> Option<PartyReactionDecision> {
    for tier in PartyDecisionTier::EVALUATION_ORDER {
        for rule in content.settings.party.combat_rules.iter().filter(|rule| {
            rule.tier == tier
                && rule.window == PartyReactionWindow::BeforeHostileDamage
                && rule.action == PartyReactionAction::Intercept
        }) {
            let mut candidates = content
                .onstage_actors()
                .enumerate()
                .filter(|(_, actor)| {
                    actor.id != content.settings.combat.player_actor_id
                        && state.stance(&actor.id) == ActorStance::Allied
                        && state.actor_is_in_room(content, &actor.id, &state.current_room_id)
                        && !state
                            .actor_is_defeated(&actor.id, &content.settings.combat.health_stat_id)
                        && state
                            .party_reaction_ready_at
                            .get(&actor.id)
                            .is_none_or(|ready_at| *ready_at <= state.current_time_minutes)
                        && tier_allows_actor(state, &actor.id, tier)
                        && rule.conditions.iter().all(|condition| {
                            condition_matches(content, state, &actor.id, condition)
                        })
                })
                .map(|(content_index, actor)| Candidate {
                    actor_id: actor.id.as_str(),
                    content_index,
                })
                .collect::<Vec<_>>();
            candidates.sort_by(|left, right| compare_candidates(content, state, rule, left, right));
            if let Some(candidate) = candidates.first() {
                return Some(PartyReactionDecision {
                    actor_id: candidate.actor_id.to_string(),
                    rule_id: rule.id.clone(),
                    message: rule.message.clone(),
                    cooldown: rule.cooldown,
                });
            }
        }
    }
    None
}

pub(crate) fn consume_party_reaction(
    content: &ContentPack,
    state: &mut WorldState,
    decision: &PartyReactionDecision,
) {
    let minutes = match decision.cooldown {
        PartyReactionCooldown::ActorCombatInterval => content
            .actor(&decision.actor_id)
            .map(|actor| {
                actor.attack_interval_minutes(
                    content.settings.combat.default_attack_interval_minutes,
                )
            })
            .unwrap_or(content.settings.combat.default_attack_interval_minutes),
        PartyReactionCooldown::FixedMinutes { minutes } => minutes,
    };
    state.party_reaction_ready_at.insert(
        decision.actor_id.clone(),
        state.current_time_minutes.saturating_add(minutes),
    );
}

fn tier_allows_actor(state: &WorldState, actor_id: &str, tier: PartyDecisionTier) -> bool {
    if tier != PartyDecisionTier::DefaultRole {
        return true;
    }
    state
        .active_party_order(actor_id)
        .is_none_or(|order| order.kind == PartyOrderKind::Follow)
}

fn condition_matches(
    content: &ContentPack,
    state: &WorldState,
    actor_id: &str,
    condition: &PartyDecisionCondition,
) -> bool {
    match condition {
        PartyDecisionCondition::HasRole { role_id } => content
            .settings
            .party
            .actor_roles
            .get(actor_id)
            .is_some_and(|roles| roles.iter().any(|role| role == role_id)),
        PartyDecisionCondition::OrderIs { orders } => state
            .active_party_order(actor_id)
            .is_some_and(|order| orders.contains(&order.kind)),
        PartyDecisionCondition::ActorHealthAtMostPercent { percent } => {
            health_percent_at_most(content, state, actor_id, *percent)
        }
        PartyDecisionCondition::ActorHealthAtLeastPercent { percent } => {
            !health_percent_at_most(content, state, actor_id, percent.saturating_sub(1))
        }
        PartyDecisionCondition::PlayerHealthAtMostPercent { percent } => health_percent_at_most(
            content,
            state,
            &content.settings.combat.player_actor_id,
            *percent,
        ),
    }
}

fn health_percent_at_most(
    content: &ContentPack,
    state: &WorldState,
    actor_id: &str,
    percent: u8,
) -> bool {
    let (current, maximum) = health_values(content, state, actor_id);
    i64::from(current) * 100 <= i64::from(maximum) * i64::from(percent)
}

fn health_values(content: &ContentPack, state: &WorldState, actor_id: &str) -> (i32, i32) {
    let health_stat = &content.settings.combat.health_stat_id;
    let current = state.actor_stat(actor_id, health_stat).max(0);
    let initial = state
        .initial_actor_stats
        .get(actor_id)
        .and_then(|stats| stats.get(health_stat))
        .copied()
        .unwrap_or(current);
    let level = state.actor_level.get(actor_id).copied().unwrap_or(1);
    let growth = (1..level)
        .filter_map(|prior_level| content.level_definition(actor_id, prior_level))
        .filter_map(|definition| definition.stat_changes.get(health_stat))
        .copied()
        .sum::<i32>();
    (current, initial.saturating_add(growth).max(1))
}

#[derive(Debug)]
struct Candidate<'a> {
    actor_id: &'a str,
    content_index: usize,
}

fn compare_candidates(
    content: &ContentPack,
    state: &WorldState,
    rule: &crate::content::types::PartyCombatDecisionRule,
    left: &Candidate<'_>,
    right: &Candidate<'_>,
) -> Ordering {
    for priority in &rule.candidate_priority {
        let ordering = match priority {
            PartyCandidatePriority::RoleOrder { role_ids } => role_rank(
                content,
                left.actor_id,
                role_ids,
            )
            .cmp(&role_rank(content, right.actor_id, role_ids)),
            PartyCandidatePriority::HighestHealthPercent => {
                let (left_current, left_max) = health_values(content, state, left.actor_id);
                let (right_current, right_max) = health_values(content, state, right.actor_id);
                (i64::from(right_current) * i64::from(left_max))
                    .cmp(&(i64::from(left_current) * i64::from(right_max)))
            }
            PartyCandidatePriority::HighestDefense => state
                .effective_actor_stat(
                    content,
                    right.actor_id,
                    &content.settings.combat.defense_stat_id,
                )
                .cmp(&state.effective_actor_stat(
                    content,
                    left.actor_id,
                    &content.settings.combat.defense_stat_id,
                )),
            PartyCandidatePriority::ContentOrder => left.content_index.cmp(&right.content_index),
        };
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    left.content_index.cmp(&right.content_index)
}

fn role_rank(content: &ContentPack, actor_id: &str, role_ids: &[String]) -> usize {
    let assigned = content.settings.party.actor_roles.get(actor_id);
    role_ids
        .iter()
        .position(|role_id| assigned.is_some_and(|roles| roles.iter().any(|role| role == role_id)))
        .unwrap_or(usize::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::types::{
        PartyCombatDecisionRule, PartyOrderTarget, PartyPolicyDefinition, PartyRoleDefinition,
        PartyTargetSelection,
    };
    use crate::engine::test_fixtures::{minimal_test_pack, rebuild_test_pack_indexes};

    fn defensive_pack() -> ContentPack {
        let mut content = minimal_test_pack();
        content.settings.combat.player_actor_id = "blair".to_string();
        content.settings.combat.health_stat_id = "stamina".to_string();
        content.settings.combat.defense_stat_id = "confidence".to_string();
        let mut drew = content.actor("casey").unwrap().clone();
        drew.id = "drew".to_string();
        drew.name = "Drew".to_string();
        drew.room_id = "lounge".to_string();
        content.actors.push(drew);
        content.settings.party = PartyPolicyDefinition {
            roles: vec![PartyRoleDefinition {
                id: "defender".to_string(),
            }],
            actor_roles: std::collections::BTreeMap::from([
                ("casey".to_string(), vec!["defender".to_string()]),
                ("drew".to_string(), vec!["defender".to_string()]),
            ]),
            combat_rules: vec![
                PartyCombatDecisionRule {
                    id: "ordered-guard".to_string(),
                    tier: PartyDecisionTier::ExplicitOrder,
                    window: PartyReactionWindow::BeforeHostileDamage,
                    action: PartyReactionAction::Intercept,
                    conditions: vec![PartyDecisionCondition::OrderIs {
                        orders: vec![PartyOrderKind::Guard],
                    }],
                    target: PartyTargetSelection::Player,
                    candidate_priority: vec![PartyCandidatePriority::ContentOrder],
                    cooldown: PartyReactionCooldown::ActorCombatInterval,
                    message: "combat.guard".to_string(),
                },
                PartyCombatDecisionRule {
                    id: "default-defender".to_string(),
                    tier: PartyDecisionTier::DefaultRole,
                    window: PartyReactionWindow::BeforeHostileDamage,
                    action: PartyReactionAction::Intercept,
                    conditions: vec![PartyDecisionCondition::HasRole {
                        role_id: "defender".to_string(),
                    }],
                    target: PartyTargetSelection::Player,
                    candidate_priority: vec![
                        PartyCandidatePriority::HighestHealthPercent,
                        PartyCandidatePriority::HighestDefense,
                    ],
                    cooldown: PartyReactionCooldown::ActorCombatInterval,
                    message: "combat.guard".to_string(),
                },
            ],
        };
        rebuild_test_pack_indexes(&mut content);
        content
    }

    fn allied_state(content: &ContentPack) -> WorldState {
        let mut state = WorldState::new(content);
        state
            .actor_room_overrides
            .insert("casey".to_string(), "lounge".to_string());
        for actor_id in ["casey", "drew"] {
            state.set_stance(actor_id, ActorStance::Allied);
        }
        state
    }

    #[test]
    fn explicit_guard_order_overrides_health_priority_and_consumes_readiness() {
        let content = defensive_pack();
        let mut state = allied_state(&content);
        state.adjust_actor_stat("casey", "stamina", -5).unwrap();
        state
            .assign_party_order(
                &content,
                "casey",
                PartyOrderKind::Guard,
                PartyOrderTarget::None,
            )
            .unwrap();

        let decision = select_defensive_reaction(&content, &state).unwrap();
        assert_eq!(decision.actor_id, "casey");
        assert_eq!(decision.rule_id, "ordered-guard");

        consume_party_reaction(&content, &mut state, &decision);
        let next = select_defensive_reaction(&content, &state).unwrap();
        assert_eq!(next.actor_id, "drew");
        assert_eq!(next.rule_id, "default-defender");
    }

    #[test]
    fn non_guard_orders_suppress_default_role_behavior() {
        let content = defensive_pack();
        let mut state = allied_state(&content);
        state
            .assign_party_order(
                &content,
                "casey",
                PartyOrderKind::Assist,
                PartyOrderTarget::None,
            )
            .unwrap();
        state.adjust_actor_stat("drew", "stamina", -6).unwrap();

        let decision = select_defensive_reaction(&content, &state).unwrap();
        assert_eq!(decision.actor_id, "drew");
    }
}
