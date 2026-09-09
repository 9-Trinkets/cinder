use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Persistent instruction assigned to an allied actor.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartyOrderKind {
    #[default]
    Follow,
    Hold,
    Guard,
    Assist,
    Scout,
    Hunt,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PartyOrderTarget {
    #[default]
    None,
    Room {
        room_id: String,
    },
    Actor {
        actor_id: String,
    },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartyOrderStatus {
    #[default]
    Active,
    Completed,
    Cancelled,
    Failed,
}

/// Save-compatible order state. World state will store one entry per ordered
/// party member; content policies inspect `kind`, while lifecycle code owns
/// status transitions and failure codes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartyOrder {
    pub kind: PartyOrderKind,
    #[serde(default)]
    pub target: PartyOrderTarget,
    #[serde(default)]
    pub status: PartyOrderStatus,
    pub issued_at_minutes: u32,
    pub updated_at_minutes: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartyRoleDefinition {
    pub id: String,
}

/// Fixed precedence tier. Survival rules may interrupt an unsafe order;
/// explicit orders override role defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartyDecisionTier {
    Survival,
    ExplicitOrder,
    DefaultRole,
}

impl PartyDecisionTier {
    pub const EVALUATION_ORDER: [Self; 3] =
        [Self::Survival, Self::ExplicitOrder, Self::DefaultRole];
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
    HasRole { role_id: String },
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
    OrderTarget,
    LowestHealthAlly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "priority", rename_all = "snake_case")]
pub enum PartyCandidatePriority {
    RoleOrder { role_ids: Vec<String> },
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

/// Content-authored party roles and ordered combat policy. Empty defaults keep
/// existing packs behavior-compatible until they opt into policy evaluation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartyPolicyDefinition {
    #[serde(default)]
    pub roles: Vec<PartyRoleDefinition>,
    #[serde(default)]
    pub actor_roles: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub combat_rules: Vec<PartyCombatDecisionRule>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn party_orders_and_policy_round_trip_through_json() {
        let order = PartyOrder {
            kind: PartyOrderKind::Scout,
            target: PartyOrderTarget::Room {
                room_id: "north-pass".to_string(),
            },
            status: PartyOrderStatus::Active,
            issued_at_minutes: 10,
            updated_at_minutes: 10,
            failure_code: None,
        };
        let restored: PartyOrder =
            serde_json::from_value(serde_json::to_value(&order).unwrap()).unwrap();
        assert_eq!(restored, order);

        let policy = PartyPolicyDefinition {
            roles: vec![PartyRoleDefinition {
                id: "defender".to_string(),
            }],
            actor_roles: BTreeMap::from([(
                "stone-guard".to_string(),
                vec!["defender".to_string()],
            )]),
            combat_rules: vec![PartyCombatDecisionRule {
                id: "guard-player".to_string(),
                tier: PartyDecisionTier::ExplicitOrder,
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
            [
                PartyDecisionTier::Survival,
                PartyDecisionTier::ExplicitOrder,
                PartyDecisionTier::DefaultRole,
            ]
        );
    }
}
