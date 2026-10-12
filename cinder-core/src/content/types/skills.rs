use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{ActionDefinition, PartyCombatDecisionRule};

/// One skill assigned to an actor.
///
/// Most assignments are authored as a plain skill id. The object form carries
/// actor-specific tuning for skills whose output varies by user while keeping
/// the capability itself defined once in `skills.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ActorSkillAssignment {
    Id(String),
    Configured(ActorSkillConfig),
}

impl ActorSkillAssignment {
    pub fn id(&self) -> &str {
        match self {
            Self::Id(id) => id,
            Self::Configured(config) => &config.id,
        }
    }

    pub fn power(&self) -> Option<i32> {
        match self {
            Self::Id(_) => None,
            Self::Configured(config) => config.power,
        }
    }

    pub fn level(&self) -> Option<u32> {
        match self {
            Self::Id(_) => None,
            Self::Configured(config) => config.level,
        }
    }

    pub fn narration_key(&self) -> Option<&str> {
        match self {
            Self::Id(_) => None,
            Self::Configured(config) => config.narration_key.as_deref(),
        }
    }
}

/// Actor-specific parameters for an assigned skill.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorSkillConfig {
    pub id: String,
    #[serde(default)]
    pub power: Option<i32>,
    #[serde(default)]
    pub level: Option<u32>,
    #[serde(default)]
    pub narration_key: Option<String>,
}

/// Operational kind of a skill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillKind {
    Attack,
    Defend,
    Heal,
    Spell,
    Passive,
    Support,
}

/// Target selection mode for skill execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillTargetMode {
    SingleEnemy,
    SingleAlly,
    LowestHealthAlly,
    SelfActor,
    Room,
    Anchor,
    None,
}

/// An autonomous use of a skill by a hostile actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillAutonomousAction {
    Strike,
    Heal,
}

impl SkillAutonomousAction {
    pub fn effect_kind(self) -> &'static str {
        match self {
            Self::Strike => "strike",
            Self::Heal => "heal",
        }
    }
}

/// Target selection for an autonomous skill use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillAutonomousTarget {
    Player,
    LowestHealthHostileAlly,
}

/// One content-authored autonomous activation path. Lower priorities are
/// considered first, so support skills can preempt attacks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillAutonomousUse {
    pub id: String,
    #[serde(default)]
    pub priority: i32,
    pub action: SkillAutonomousAction,
    pub target: SkillAutonomousTarget,
    /// Symbolic `effect_table` rule evaluated against the shared hostile
    /// actor/world input. It emits the autonomous action's effect kind.
    pub rule: Value,
    /// When targeting hostile allies, prefer self below this health threshold.
    #[serde(default)]
    pub prefer_self_below_percent: Option<u8>,
}

/// Scaling parameters for a single numerical property of a skill across levels.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillParameterScaling {
    /// Base value at level 1 (default: 0).
    #[serde(default)]
    pub base: i64,
    /// Value added per level above 1 (value = base + (level - 1) * growth_per_level).
    #[serde(default)]
    pub growth_per_level: i64,
    /// If specified, growth occurs every `step_levels` (e.g. 2 for every 2 levels).
    /// Formula: base + ((level - 1) / step_levels) * growth_per_step
    #[serde(default)]
    pub step_levels: Option<u32>,
    /// Value added per step if `step_levels` is set. If not set, defaults to `growth_per_level`.
    #[serde(default)]
    pub growth_per_step: Option<i64>,
    /// Explicit table of values by level (1-indexed, level 1 = table[0]). Overrides formula if non-empty.
    #[serde(default)]
    pub table: Vec<i64>,
}

impl SkillParameterScaling {
    pub fn evaluate(&self, level: u32) -> i64 {
        if !self.table.is_empty() {
            let idx = (level.max(1) - 1) as usize;
            return self
                .table
                .get(idx)
                .copied()
                .unwrap_or_else(|| self.table.last().copied().unwrap_or(self.base));
        }

        let step = self.step_levels.unwrap_or(1).max(1);
        let growth = self.growth_per_step.unwrap_or(self.growth_per_level);
        let steps_count = (level.saturating_sub(1) / step) as i64;
        self.base + steps_count * growth
    }
}

/// Declarative scaling configuration for a skill across levels 1..=10.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillScalingDefinition {
    /// Named parameters and their per-level scaling curves.
    #[serde(default)]
    pub parameters: BTreeMap<String, SkillParameterScaling>,
    /// Format string for displaying the scalar summary in UI (e.g. "Bonus damage: +{bonus_damage}").
    #[serde(default)]
    pub summary_template: Option<String>,
    /// Optional override string when all parameters evaluate to 0 (e.g. "Base MP cost").
    #[serde(default)]
    pub zero_summary: Option<String>,
}

impl SkillScalingDefinition {
    pub fn evaluate_params(&self, level: u32) -> BTreeMap<String, i64> {
        self.parameters
            .iter()
            .map(|(k, v)| (k.clone(), v.evaluate(level)))
            .collect()
    }

    pub fn format_summary(&self, level: u32) -> Option<String> {
        let template = self.summary_template.as_ref()?;
        let values = self.evaluate_params(level);

        if let Some(zero_msg) = &self.zero_summary {
            if values.values().all(|&v| v == 0) {
                return Some(zero_msg.clone());
            }
        }

        let mut formatted = template.clone();
        for (name, val) in &values {
            formatted = formatted.replace(&format!("{{{}}}", name), &val.to_string());
        }
        Some(formatted)
    }
}

/// Declarative definition of an ability in Cinder. Every activation path for
/// the skill is authored here: player command, party reactions, and autonomous
/// hostile uses. The loader binds nested actions and reactions back to this id
/// before the generic runtime engines consume them.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkillDefinition {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub kind: Option<SkillKind>,
    #[serde(default)]
    pub target: Option<SkillTargetMode>,
    #[serde(default)]
    pub description: String,
    /// Per-skill override for SP level-up thresholds.
    #[serde(default)]
    pub sp_thresholds: Option<Vec<u32>>,
    /// Declarative parameter scaling across skill levels.
    #[serde(default)]
    pub scaling: Option<SkillScalingDefinition>,
    #[serde(default)]
    pub player_action: Option<ActionDefinition>,
    /// Cross-skill party reaction precedence. Lower values are evaluated first;
    /// authored order is retained among skills with the same value.
    #[serde(default)]
    pub reaction_priority: i32,
    #[serde(default)]
    pub reactions: Vec<PartyCombatDecisionRule>,
    #[serde(default)]
    pub autonomous: Vec<SkillAutonomousUse>,
}

impl SkillDefinition {
    /// Evaluates a named scaling parameter at the given skill level.
    pub fn param_value(&self, param_name: &str, level: u32) -> Option<i64> {
        let scaling = self.scaling.as_ref()?;
        let param = scaling.parameters.get(param_name)?;
        Some(param.evaluate(level))
    }

    /// Formats the UI scalar summary using `summary_template` at the given skill level.
    pub fn format_summary(&self, level: u32) -> Option<String> {
        let scaling = self.scaling.as_ref()?;
        scaling.format_summary(level)
    }
}

/// Top-level container for all skills declared in `skills.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkillsDefinition {
    /// When enabled, all skill references and actor capabilities are validated
    /// and runtime behavior never falls back to implicit ownership.
    #[serde(default)]
    pub strict: bool,
    /// Pack-level default SP level-up thresholds for skills.
    #[serde(default)]
    pub sp_thresholds: Option<Vec<u32>>,
    #[serde(default)]
    pub skills: Vec<SkillDefinition>,
}
