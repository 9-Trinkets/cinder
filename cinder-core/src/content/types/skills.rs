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

    pub fn narration_key(&self) -> Option<&str> {
        match self {
            Self::Id(_) => None,
            Self::Configured(config) => config.narration_key.as_deref(),
        }
    }
}

/// Actor-specific parameters for an assigned skill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorSkillConfig {
    pub id: String,
    #[serde(default)]
    pub power: Option<i32>,
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

/// Top-level container for all skills declared in `skills.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkillsDefinition {
    /// When enabled, all skill references and actor capabilities are validated
    /// and runtime behavior never falls back to implicit ownership.
    #[serde(default)]
    pub strict: bool,
    #[serde(default)]
    pub skills: Vec<SkillDefinition>,
}
