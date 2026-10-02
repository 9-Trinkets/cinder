use serde::{Deserialize, Serialize};

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

/// Declarative identity and presentation metadata for an ability in Cinder.
/// Invocation timing, requirements, and effects stay with the action, party
/// rule, or behavior that explicitly binds this skill id.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillDefinition {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub kind: Option<SkillKind>,
    #[serde(default)]
    pub target: Option<SkillTargetMode>,
    #[serde(default)]
    pub description: String,
}

/// Top-level container for all skills declared in `skills.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillsDefinition {
    /// When enabled, all skill references and actor capabilities are validated
    /// and runtime behavior never falls back to implicit ownership.
    #[serde(default)]
    pub strict: bool,
    #[serde(default)]
    pub skills: Vec<SkillDefinition>,
}
