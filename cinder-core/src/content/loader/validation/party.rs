use super::require_known_id;
use crate::content::types::{
    PackMessage, PartyCandidatePriority, PartyDecisionCondition, PartyDecisionTier,
    PartyPolicyDefinition, PartyReactionAction, PartyReactionCooldown, PartyReactionWindow,
};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;

pub(crate) fn validate_party_policy(
    policy: &PartyPolicyDefinition,
    actor_ids: &[&str],
    messages: &BTreeMap<String, PackMessage>,
) -> Result<(), Box<dyn Error>> {
    let mut role_ids = BTreeSet::new();
    for role in &policy.roles {
        let role_id = role.id.trim();
        if role_id.is_empty() {
            return Err("party.roles contains an empty role id".into());
        }
        if !role_ids.insert(role_id) {
            return Err(format!("duplicate party role id '{role_id}'").into());
        }
    }

    for (actor_id, assigned_roles) in &policy.actor_roles {
        require_known_id(
            actor_id,
            actor_ids,
            &format!("party.actor_roles actor '{actor_id}'"),
            "actors",
        )?;
        let mut actor_role_ids = BTreeSet::new();
        for role_id in assigned_roles {
            require_known_role(role_id, &role_ids, actor_id)?;
            if !actor_role_ids.insert(role_id.as_str()) {
                return Err(format!(
                    "party.actor_roles actor '{actor_id}' repeats role '{role_id}'"
                )
                .into());
            }
        }
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
        validate_rule(rule, &role_ids, messages)?;
    }
    Ok(())
}

fn require_known_role(
    role_id: &str,
    known_role_ids: &BTreeSet<&str>,
    subject: &str,
) -> Result<(), Box<dyn Error>> {
    if known_role_ids.contains(role_id) {
        Ok(())
    } else {
        Err(format!("party role '{role_id}' referenced by '{subject}' is not declared").into())
    }
}

fn validate_rule(
    rule: &crate::content::types::PartyCombatDecisionRule,
    role_ids: &BTreeSet<&str>,
    messages: &BTreeMap<String, PackMessage>,
) -> Result<(), Box<dyn Error>> {
    let has_order_condition = rule
        .conditions
        .iter()
        .any(|condition| matches!(condition, PartyDecisionCondition::OrderIs { .. }));
    let has_role_condition = rule
        .conditions
        .iter()
        .any(|condition| matches!(condition, PartyDecisionCondition::HasRole { .. }));
    if rule.tier == PartyDecisionTier::ExplicitOrder && !has_order_condition {
        return Err(format!(
            "party combat rule '{}' is explicit_order but has no order_is condition",
            rule.id
        )
        .into());
    }
    if rule.tier == PartyDecisionTier::DefaultRole && !has_role_condition {
        return Err(format!(
            "party combat rule '{}' is default_role but has no has_role condition",
            rule.id
        )
        .into());
    }
    for condition in &rule.conditions {
        match condition {
            PartyDecisionCondition::HasRole { role_id } => {
                require_known_role(role_id, role_ids, &rule.id)?;
            }
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
    for priority in &rule.candidate_priority {
        if let PartyCandidatePriority::RoleOrder {
            role_ids: priority_roles,
        } = priority
        {
            if priority_roles.is_empty() {
                return Err(format!(
                    "party combat rule '{}' role_order priority must not be empty",
                    rule.id
                )
                .into());
            }
            for role_id in priority_roles {
                require_known_role(role_id, role_ids, &rule.id)?;
            }
        }
    }
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
            crate::content::types::PartyTargetSelection::Player,
        )
        | (
            PartyReactionWindow::AfterHostileDamage,
            PartyReactionAction::Counterattack
            | PartyReactionAction::Support
            | PartyReactionAction::Hold,
            _,
        ) => {}
        _ => {
            return Err(format!(
                "party combat rule '{}' action is incompatible with its reaction window",
                rule.id
            )
            .into());
        }
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
        PartyRoleDefinition, PartyTargetSelection,
    };

    fn valid_policy() -> PartyPolicyDefinition {
        PartyPolicyDefinition {
            roles: vec![PartyRoleDefinition {
                id: "defender".to_string(),
            }],
            actor_roles: BTreeMap::from([("guard".to_string(), vec!["defender".to_string()])]),
            combat_rules: vec![PartyCombatDecisionRule {
                id: "ordered-guard".to_string(),
                tier: PartyDecisionTier::ExplicitOrder,
                window: PartyReactionWindow::BeforeHostileDamage,
                action: PartyReactionAction::Intercept,
                conditions: vec![PartyDecisionCondition::OrderIs {
                    orders: vec![PartyOrderKind::Guard],
                }],
                target: PartyTargetSelection::Player,
                candidate_priority: Vec::new(),
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
            &BTreeMap::from([(
                "combat.guard".to_string(),
                PackMessage::Narration("Guard.".to_string()),
            )]),
        )
        .unwrap();
    }

    #[test]
    fn rejects_unknown_roles_and_invalid_explicit_order_rules() {
        let mut policy = valid_policy();
        policy
            .actor_roles
            .insert("guard".to_string(), vec!["missing".to_string()]);
        let error = validate_party_policy(&policy, &["guard"], &BTreeMap::new())
            .unwrap_err()
            .to_string();
        assert!(error.contains("not declared"), "{error}");

        let mut policy = valid_policy();
        policy.combat_rules[0].conditions.clear();
        let error = validate_party_policy(&policy, &["guard"], &BTreeMap::new())
            .unwrap_err()
            .to_string();
        assert!(error.contains("no order_is condition"), "{error}");
    }
}
