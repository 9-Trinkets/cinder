use super::super::Condition;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// The single source of truth for all movement configuration in a content pack:
/// which stages lock all movement, which rooms are unreachable, the reactive
/// suppression rule (e.g. low stamina), and each actor's target rules.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MovementConfigDefinition {
    #[serde(default)]
    pub stage_locks: Vec<String>,
    #[serde(default)]
    pub unreachable_rooms: Vec<String>,
    #[serde(default)]
    pub suppress_when: Option<Value>,
    #[serde(default)]
    pub defaults: MovementDefaultsDefinition,
    #[serde(default)]
    pub actors: BTreeMap<String, ActorMovementRulesDefinition>,
}

/// Pack-wide movement defaults, applied to any actor that doesn't declare its
/// own rules/order. Pure `movement.json` owns direction + cadence; the
/// *eligibility* to hold vs. move is decided by `behavior.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MovementDefaultsDefinition {
    #[serde(default)]
    pub order: Option<MovementOrderDefinition>,
}

/// A movement order authored for an actor. `movement.json` is the single place
/// that says *where* (and how often) an actor drifts when it is free to move;
/// whether it is actually free to move is `behavior.json`'s concern.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MovementOrderDefinition {
    /// A word from the shared order vocabulary, e.g. `patrol`, `guard`, `exit`,
    /// `deploy` (see `engine::orders`).
    #[serde(default)]
    pub token: String,
    /// How often the actor acts: 1 = every tick, 2 = every other tick, etc.
    /// 0 means stationary.
    #[serde(default)]
    pub cadence_ticks: u32,
    /// Optional fixed destination for target-bearing orders (`deploy`/`to`).
    #[serde(default)]
    pub target_room_id: String,
    /// Optional exit label or alias for the `exit` order.
    #[serde(default)]
    pub exit_label: String,
}

/// Per-actor movement rules (and an optional per-actor order) that override the
/// pack-wide `MovementConfigDefinition::defaults`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActorMovementRulesDefinition {
    #[serde(default)]
    pub target_rules: Vec<ActorMovementTargetRuleDefinition>,
    /// Per-actor order; falls back to the pack-wide default in
    /// `MovementConfigDefinition::defaults.order`.
    #[serde(default)]
    pub order: Option<MovementOrderDefinition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MovementTargetBehavior {
    Move,
    Stay,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorMovementTargetRuleDefinition {
    #[serde(default)]
    pub target_room_id: String,
    #[serde(default)]
    pub when_player_room_id: String,
    /// Conditions that must hold for this target rule to apply.
    #[serde(default)]
    pub when: Vec<Condition>,
    #[serde(default)]
    pub any_active_stage_ids: Vec<String>,
    #[serde(default)]
    pub target_from_story_var: String,
    #[serde(default)]
    pub target_behavior: Option<MovementTargetBehavior>,
}
