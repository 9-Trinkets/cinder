use crate::content::types::{
    ContentPack, PartyCandidatePriority, PartyDecisionCondition, PartyDecisionTier,
    PartyReactionAction, PartyReactionCooldown, PartyReactionWindow, PartySupportEffect,
    PartyTargetSelection,
};
use crate::engine::state::{ActorStance, WorldState};
use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PartyReactionDecision {
    pub actor_id: String,
    pub rule_id: String,
    pub action: PartyReactionAction,
    pub target: PartyTargetSelection,
    pub support_effect: Option<PartySupportEffect>,
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
                    action: rule.action,
                    target: rule.target,
                    support_effect: rule.support_effect.clone(),
                    message: rule.message.clone(),
                    cooldown: rule.cooldown,
                });
            }
        }
    }
    None
}

pub(crate) fn select_post_damage_reactions(
    content: &ContentPack,
    state: &WorldState,
) -> Vec<PartyReactionDecision> {
    content
        .onstage_actors()
        .filter(|actor| actor_is_reaction_eligible(content, state, &actor.id))
        .filter_map(|actor| {
            PartyDecisionTier::EVALUATION_ORDER
                .into_iter()
                .find_map(|tier| {
                    content.settings.party.combat_rules.iter().find(|rule| {
                        rule.tier == tier
                            && rule.window == PartyReactionWindow::AfterHostileDamage
                            && matches!(
                                rule.action,
                                PartyReactionAction::Counterattack
                                    | PartyReactionAction::Support
                                    | PartyReactionAction::Hold
                            )
                            && rule.conditions.iter().all(|condition| {
                                condition_matches(content, state, &actor.id, condition)
                            })
                    })
                })
                .map(|rule| PartyReactionDecision {
                    actor_id: actor.id.clone(),
                    rule_id: rule.id.clone(),
                    action: rule.action,
                    target: rule.target,
                    support_effect: rule.support_effect.clone(),
                    message: rule.message.clone(),
                    cooldown: rule.cooldown,
                })
        })
        .collect()
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

pub(crate) fn resolve_party_reaction_target(
    content: &ContentPack,
    state: &WorldState,
    decision: &PartyReactionDecision,
    attacker_id: &str,
) -> Option<String> {
    match decision.target {
        PartyTargetSelection::SelfActor => Some(decision.actor_id.clone()),
        PartyTargetSelection::Player => Some(content.settings.combat.player_actor_id.clone()),
        PartyTargetSelection::Attacker => Some(attacker_id.to_string()),
        PartyTargetSelection::LowestHealthAlly => lowest_health_ally(content, state),
    }
}

fn actor_is_reaction_eligible(content: &ContentPack, state: &WorldState, actor_id: &str) -> bool {
    actor_id != content.settings.combat.player_actor_id
        && state.stance(actor_id) == ActorStance::Allied
        && state.actor_is_in_room(content, actor_id, &state.current_room_id)
        && !state.actor_is_defeated(actor_id, &content.settings.combat.health_stat_id)
        && state
            .party_reaction_ready_at
            .get(actor_id)
            .is_none_or(|ready_at| *ready_at <= state.current_time_minutes)
}

fn condition_matches(
    content: &ContentPack,
    state: &WorldState,
    actor_id: &str,
    condition: &PartyDecisionCondition,
) -> bool {
    match condition {
        PartyDecisionCondition::OrderIs { orders } => state
            .party_order(content, actor_id)
            .is_some_and(|order| orders.contains(&order)),
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
    let maximum = state.actor_stat_maximum(content, actor_id, health_stat).max(1);
    (current, maximum)
}

fn lowest_health_ally(content: &ContentPack, state: &WorldState) -> Option<String> {
    let player_id = content.settings.combat.player_actor_id.as_str();
    std::iter::once(player_id)
        .chain(
            content
                .onstage_actors()
                .map(|actor| actor.id.as_str())
                .filter(|actor_id| *actor_id != player_id)
                .filter(|actor_id| state.stance(actor_id) == ActorStance::Allied),
        )
        .filter(|actor_id| {
            (*actor_id == player_id
                || state.actor_is_in_room(content, actor_id, &state.current_room_id))
                && !state.actor_is_defeated(actor_id, &content.settings.combat.health_stat_id)
        })
        .min_by(|left, right| {
            let (left_current, left_max) = health_values(content, state, left);
            let (right_current, right_max) = health_values(content, state, right);
            (i64::from(left_current) * i64::from(right_max))
                .cmp(&(i64::from(right_current) * i64::from(left_max)))
        })
        .map(str::to_string)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::types::{
        PartyCombatDecisionRule, PartyPolicyDefinition, PartyTargetSelection,
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
            initial_orders: std::collections::BTreeMap::from([
                ("casey".to_string(), "guard".to_string()),
                ("drew".to_string(), "guard".to_string()),
            ]),
            combat_rules: vec![PartyCombatDecisionRule {
                id: "guard-order".to_string(),
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
    fn guard_order_uses_priority_and_consumes_readiness() {
        let content = defensive_pack();
        let mut state = allied_state(&content);
        state.adjust_actor_stat(&content, "casey", "stamina", -5).unwrap();
        state
            .assign_party_order(&content, "casey", "guard".to_string())
            .unwrap();

        let decision = select_defensive_reaction(&content, &state).unwrap();
        assert_eq!(decision.actor_id, "drew");
        assert_eq!(decision.rule_id, "guard-order");

        consume_party_reaction(&content, &mut state, &decision);
        let next = select_defensive_reaction(&content, &state).unwrap();
        assert_eq!(next.actor_id, "casey");
        assert_eq!(next.rule_id, "guard-order");
    }

    #[test]
    fn assist_order_suppresses_guard_behavior() {
        let content = defensive_pack();
        let mut state = allied_state(&content);
        state
            .assign_party_order(&content, "casey", "assist".to_string())
            .unwrap();
        state.adjust_actor_stat(&content, "drew", "stamina", -6).unwrap();

        let decision = select_defensive_reaction(&content, &state).unwrap();
        assert_eq!(decision.actor_id, "drew");
    }
}
