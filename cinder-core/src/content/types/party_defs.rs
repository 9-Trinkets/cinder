use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Persistent combat directive assigned to an allied actor. Party members
/// continue following the player under either directive.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartyOrderKind {
    #[default]
    Guard,
    Assist,
}

/// Fixed precedence tier. Survival rules may temporarily interrupt the
/// member's persistent combat directive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartyDecisionTier {
    Survival,
    Order,
}

impl PartyDecisionTier {
    pub const EVALUATION_ORDER: [Self; 2] = [Self::Survival, Self::Order];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartyReactionWindow {
    BeforeHostileDamage,
    AfterHostileDamage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartyReactionAction {
    Intercept,
    Counterattack,
    Support,
    Hold,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum PartyReactionCooldown {
    #[default]
    ActorCombatInterval,
    FixedMinutes {
        minutes: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "condition", rename_all = "snake_case")]
pub enum PartyDecisionCondition {
    OrderIs { orders: Vec<PartyOrderKind> },
    ActorHealthAtMostPercent { percent: u8 },
    ActorHealthAtLeastPercent { percent: u8 },
    PlayerHealthAtMostPercent { percent: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartyTargetSelection {
    SelfActor,
    Player,
    Attacker,
    LowestHealthAlly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "priority", rename_all = "snake_case")]
pub enum PartyCandidatePriority {
    HighestHealthPercent,
    HighestDefense,
    ContentOrder,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "effect", rename_all = "snake_case")]
pub enum PartySupportEffect {
    AdjustActorStat { stat: String, delta: i32 },
}

/// One deterministic combat choice. Rules retain authored order within their
/// precedence tier; the first eligible rule wins for each actor and window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartyCombatDecisionRule {
    pub id: String,
    pub tier: PartyDecisionTier,
    pub window: PartyReactionWindow,
    pub action: PartyReactionAction,
    #[serde(default)]
    pub conditions: Vec<PartyDecisionCondition>,
    pub target: PartyTargetSelection,
    #[serde(default)]
    pub candidate_priority: Vec<PartyCandidatePriority>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub support_effect: Option<PartySupportEffect>,
    #[serde(default)]
    pub cooldown: PartyReactionCooldown,
    /// Locale message key used when this action resolves.
    #[serde(default)]
    pub message: String,
}

/// Content-authored initial directives and ordered combat policy. Empty
/// defaults keep packs without controllable parties behavior-compatible.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartyPolicyDefinition {
    #[serde(default)]
    pub initial_orders: BTreeMap<String, PartyOrderKind>,
    #[serde(default)]
    pub combat_rules: Vec<PartyCombatDecisionRule>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn party_orders_and_policy_round_trip_through_json() {
        let policy = PartyPolicyDefinition {
            initial_orders: BTreeMap::from([("stone-guard".to_string(), PartyOrderKind::Guard)]),
            combat_rules: vec![PartyCombatDecisionRule {
                id: "guard-player".to_string(),
                tier: PartyDecisionTier::Order,
                window: PartyReactionWindow::BeforeHostileDamage,
                action: PartyReactionAction::Intercept,
                conditions: vec![PartyDecisionCondition::OrderIs {
                    orders: vec![PartyOrderKind::Guard],
                }],
                target: PartyTargetSelection::Player,
                candidate_priority: vec![
                    PartyCandidatePriority::HighestHealthPercent,
                    PartyCandidatePriority::HighestDefense,
                    PartyCandidatePriority::ContentOrder,
                ],
                support_effect: None,
                cooldown: PartyReactionCooldown::ActorCombatInterval,
                message: "combat.guard_intercepts".to_string(),
            }],
        };
        let restored: PartyPolicyDefinition =
            serde_json::from_value(serde_json::to_value(&policy).unwrap()).unwrap();
        assert_eq!(restored, policy);
    }

    #[test]
    fn empty_party_policy_is_backward_compatible() {
        let policy: PartyPolicyDefinition = serde_json::from_str("{}").unwrap();
        assert_eq!(policy, PartyPolicyDefinition::default());
        assert_eq!(
            PartyDecisionTier::EVALUATION_ORDER,
            [PartyDecisionTier::Survival, PartyDecisionTier::Order]
        );
    }
}
