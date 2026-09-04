use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One step of growth for an actor. `exp_required` is the XP needed to
/// advance from the *current* level (the entry's index + 1) to the next.
/// `stat_changes` are deltas applied on reaching that next level; `unlocks`
/// reserves space for future skills/spells earned at that level.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LevelDefinition {
    #[serde(default)]
    pub exp_required: u32,
    #[serde(default)]
    pub stat_changes: BTreeMap<String, i32>,
    /// Identifiers of skills/spells granted by reaching this level. Not yet
    /// consumed by the engine; declared so the schema is stable for when
    /// abilities land.
    #[serde(default)]
    pub unlocks: Vec<String>,
}

/// A single advance table for one actor (or the shared default). Index `n`
/// governs the transition from level `n+1` to `n+2`.
pub type LevelTable = Vec<LevelDefinition>;

/// Leveling rules declared by the content pack. The `default` table applies
/// to every actor unless that actor has a per-actor override in `actors`,
/// enabling differentiated classes/jobs to advance on their own curves.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LevelingDefinition {
    #[serde(default)]
    pub default: LevelTable,
    /// Per-actor override tables, keyed by actor id. An actor without an
    /// entry falls back to `default`.
    #[serde(default)]
    pub actors: BTreeMap<String, LevelTable>,
}
