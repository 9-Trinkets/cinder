use super::require_known_id;
use crate::content::types::{
    PackMessage, PartyDecisionCondition, PartyDecisionTier, PartyPolicyDefinition,
    PartyReactionAction, PartyReactionCooldown, PartyReactionWindow, PartySupportEffect,
    PartyTargetSelection,
};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;

pub(crate) fn validate_party_policy(
    policy: &PartyPolicyDefinition,
    actor_ids: &[&str],
    actor_stat_ids: &[&str],
    messages: &BTreeMap<String, PackMessage>,
) -> Result<(), Box<dyn Error>> {
    for actor_id in policy.initial_orders.keys() {
        require_known_id(
            actor_id,
            actor_ids,
            &format!("party.initial_orders actor '{actor_id}'"),
            "actors",
        )?;
    }

    let mut rule_ids = BTreeSet::new();
    for rule in &policy.combat_rules {
        let rule_id = rule.id.trim();
        if rule_id.is_empty() {
            return Err("party.combat_rules contains an empty rule id".into());
        }
        if !rule_ids.insert(rule_id) {
            return Err(format!("duplicate party combat rule id '{rule_id}'").into());
        }
        validate_rule(rule, actor_stat_ids, messages)?;
    }
    Ok(())
}

fn validate_rule(
    rule: &crate::content::types::PartyCombatDecisionRule,
    actor_stat_ids: &[&str],
    messages: &BTreeMap<String, PackMessage>,
) -> Result<(), Box<dyn Error>> {
    let has_order_condition = rule
        .conditions
        .iter()
        .any(|condition| matches!(condition, PartyDecisionCondition::OrderIs { .. }));
    if rule.tier == PartyDecisionTier::Order && !has_order_condition {
        return Err(format!(
            "party combat rule '{}' is order-tier but has no order_is condition",
            rule.id
        )
        .into());
    }
    for condition in &rule.conditions {
        match condition {
            PartyDecisionCondition::OrderIs { orders } if orders.is_empty() => {
                return Err(format!(
                    "party combat rule '{}' has an empty order_is condition",
                    rule.id
                )
                .into());
            }
            PartyDecisionCondition::ActorHealthAtMostPercent { percent }
            | PartyDecisionCondition::ActorHealthAtLeastPercent { percent }
            | PartyDecisionCondition::PlayerHealthAtMostPercent { percent }
                if *percent > 100 =>
            {
                return Err(format!(
                    "party combat rule '{}' health percentage must be at most 100",
                    rule.id
                )
                .into());
            }
            _ => {}
        }
    }
    let _ = &rule.candidate_priority;
    if matches!(
        rule.cooldown,
        PartyReactionCooldown::FixedMinutes { minutes: 0 }
    ) {
        return Err(format!(
            "party combat rule '{}' fixed cooldown must be positive",
            rule.id
        )
        .into());
    }
    match (rule.window, rule.action, rule.target) {
        (
            PartyReactionWindow::BeforeHostileDamage,
            PartyReactionAction::Intercept,
            PartyTargetSelection::Player,
        )
        | (
            PartyReactionWindow::AfterHostileDamage,
            PartyReactionAction::Counterattack,
            PartyTargetSelection::Attacker,
        )
        | (
            PartyReactionWindow::AfterHostileDamage,
            PartyReactionAction::Support,
            PartyTargetSelection::SelfActor
            | PartyTargetSelection::Player
            | PartyTargetSelection::LowestHealthAlly,
        )
        | (
            PartyReactionWindow::AfterHostileDamage,
            PartyReactionAction::Hold,
            PartyTargetSelection::SelfActor,
        ) => {}
        _ => {
            return Err(format!(
                "party combat rule '{}' action is incompatible with its reaction window",
                rule.id
            )
            .into());
        }
    }
    match (&rule.action, &rule.support_effect) {
        (
            PartyReactionAction::Support,
            Some(PartySupportEffect::AdjustActorStat { stat, delta }),
        ) => {
            require_known_id(
                stat,
                actor_stat_ids,
                &format!("party combat rule '{}' support stat '{stat}'", rule.id),
                "stats.actor",
            )?;
            if *delta <= 0 {
                return Err(format!(
                    "party combat rule '{}' support stat delta must be positive",
                    rule.id
                )
                .into());
            }
        }
        (PartyReactionAction::Support, None) => {
            return Err(format!(
                "party combat rule '{}' support action requires support_effect",
                rule.id
            )
            .into());
        }
        (_, Some(_)) => {
            return Err(format!(
                "party combat rule '{}' has support_effect but is not a support action",
                rule.id
            )
            .into());
        }
        (_, None) => {}
    }
    if !rule.message.trim().is_empty() && !messages.contains_key(&rule.message) {
        return Err(format!(
            "party combat rule '{}' message '{}' not found in locale messages",
            rule.id, rule.message
        )
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::types::{
        PackMessage, PartyCombatDecisionRule, PartyDecisionCondition, PartyDecisionTier,
        PartyOrderKind, PartyReactionAction, PartyReactionCooldown, PartyReactionWindow,
        PartyTargetSelection,
    };

    fn valid_policy() -> PartyPolicyDefinition {
        PartyPolicyDefinition {
            initial_orders: BTreeMap::from([("guard".to_string(), PartyOrderKind::Guard)]),
            combat_rules: vec![PartyCombatDecisionRule {
                id: "ordered-guard".to_string(),
                tier: PartyDecisionTier::Order,
                window: PartyReactionWindow::BeforeHostileDamage,
                action: PartyReactionAction::Intercept,
                conditions: vec![PartyDecisionCondition::OrderIs {
                    orders: vec![PartyOrderKind::Guard],
                }],
                target: PartyTargetSelection::Player,
                candidate_priority: Vec::new(),
                support_effect: None,
                cooldown: PartyReactionCooldown::ActorCombatInterval,
                message: "combat.guard".to_string(),
            }],
        }
    }

    #[test]
    fn accepts_a_well_formed_party_policy() {
        validate_party_policy(
            &valid_policy(),
            &["guard"],
            &["stamina"],
            &BTreeMap::from([(
                "combat.guard".to_string(),
                PackMessage::Narration("Guard.".to_string()),
            )]),
        )
        .unwrap();
    }

    #[test]
    fn rejects_unknown_initial_actors_and_invalid_order_rules() {
        let mut policy = valid_policy();
        policy
            .initial_orders
            .insert("missing".to_string(), PartyOrderKind::Guard);
        let error = validate_party_policy(&policy, &["guard"], &["stamina"], &BTreeMap::new())
            .unwrap_err()
            .to_string();
        assert!(error.contains("not found"), "{error}");

        let mut policy = valid_policy();
        policy.combat_rules[0].conditions.clear();
        let error = validate_party_policy(&policy, &["guard"], &["stamina"], &BTreeMap::new())
            .unwrap_err()
            .to_string();
        assert!(error.contains("no order_is condition"), "{error}");
    }

    #[test]
    fn validates_support_effects_and_targets() {
        let mut policy = valid_policy();
        let rule = &mut policy.combat_rules[0];
        rule.window = PartyReactionWindow::AfterHostileDamage;
        rule.action = PartyReactionAction::Support;
        rule.target = PartyTargetSelection::Player;
        rule.message.clear();
        rule.support_effect = Some(PartySupportEffect::AdjustActorStat {
            stat: "stamina".to_string(),
            delta: 2,
        });
        validate_party_policy(&policy, &["guard"], &["stamina"], &BTreeMap::new()).unwrap();

        let mut invalid = policy.clone();
        invalid.combat_rules[0].support_effect = Some(PartySupportEffect::AdjustActorStat {
            stat: "stamina".to_string(),
            delta: 0,
        });
        let error = validate_party_policy(&invalid, &["guard"], &["stamina"], &BTreeMap::new())
            .unwrap_err()
            .to_string();
        assert!(error.contains("must be positive"), "{error}");

        let mut invalid = policy;
        invalid.combat_rules[0].target = PartyTargetSelection::Attacker;
        let error = validate_party_policy(&invalid, &["guard"], &["stamina"], &BTreeMap::new())
            .unwrap_err()
            .to_string();
        assert!(error.contains("incompatible"), "{error}");
    }
}
