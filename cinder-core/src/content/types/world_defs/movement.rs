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
/// own rules/destination. Pure `movement.json` owns direction + cadence; the
/// *eligibility* to hold vs. move is decided by `behavior.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MovementDefaultsDefinition {
    #[serde(default)]
    pub wander: Option<WanderDefinition>,
}

/// Destination directive for a wandering actor. `movement.json` is the single
/// place that says *where* (and how often) an actor drifts when it is free to
/// move; whether it is actually free to move is `behavior.json`'s concern.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WanderDefinition {
    /// How the destination is chosen when the actor wanders.
    #[serde(default)]
    pub mode: WanderMode,
    /// How often the actor wanders: 1 = every tick, 2 = every other tick, etc.
    /// 0 means it never wanders.
    #[serde(default)]
    pub cadence_ticks: u32,
    /// Optional fixed destination; used by `WanderMode::To` and ignored
    /// otherwise.
    #[serde(default)]
    pub room_id: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WanderMode {
    /// Pick a random reachable adjacent room each time.
    #[default]
    RandomAdjacent,
    /// Step toward the player's current room.
    TowardPlayer,
    /// Stay put (movement suppressed).
    Stay,
    /// Move toward the fixed `room_id` destination.
    To,
}

/// Per-actor movement rules (and an optional per-actor wander directive) that
/// override the pack-wide `MovementConfigDefinition::defaults`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActorMovementRulesDefinition {
    #[serde(default)]
    pub target_rules: Vec<ActorMovementTargetRuleDefinition>,
    /// Per-actor wander directive; falls back to the pack-wide default in
    /// `MovementConfigDefinition::defaults.wander`.
    #[serde(default)]
    pub wander: Option<WanderDefinition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MovementTargetBehavior {
    Move,
    Stay,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActorMovementTargetRuleDefinition {
    #[serde(default)]
    pub target_room_id: String,
    #[serde(default)]
    pub when_player_room_id: String,
    #[serde(default)]
    pub required_story_var: String,
    #[serde(default)]
    pub any_active_stage_ids: Vec<String>,
    #[serde(default)]
    pub target_from_story_var: String,
    #[serde(default)]
    pub target_behavior: Option<MovementTargetBehavior>,
}