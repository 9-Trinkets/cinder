use serde::{Deserialize, Serialize};

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

/// Specification for an attack skill.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttackSkillSpec {
    #[serde(default)]
    pub stat: String,
    #[serde(default = "default_power_multiplier")]
    pub power_multiplier: u32,
}

fn default_power_multiplier() -> u32 {
    1
}

/// Trigger condition for defensive reactions.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefendTriggerCondition {
    #[serde(default)]
    pub ally_health_at_most_percent: Option<u8>,
    #[serde(default)]
    pub self_health_at_least_percent: Option<u8>,
    #[serde(default)]
    pub self_health_at_most_percent: Option<u8>,
}

/// Specification for a defend skill.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefendSkillSpec {
    #[serde(default)]
    pub interception: bool,
    #[serde(default)]
    pub trigger_condition: Option<DefendTriggerCondition>,
}

/// Trigger condition for healing reactions.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealTriggerCondition {
    #[serde(default)]
    pub ally_health_at_most_percent: Option<u8>,
}

/// Specification for a healing skill.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealSkillSpec {
    #[serde(default)]
    pub amount: i32,
    #[serde(default)]
    pub trigger_condition: Option<HealTriggerCondition>,
}

/// Specification for a spell / utility skill.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpellSkillSpec {
    #[serde(default)]
    pub requires_item: Option<String>,
    #[serde(default)]
    pub requires_story_var: Option<String>,
}

/// Declarative definition of an ability in Cinder.
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
    #[serde(default)]
    pub attack: Option<AttackSkillSpec>,
    #[serde(default)]
    pub defend: Option<DefendSkillSpec>,
    #[serde(default)]
    pub heal: Option<HealSkillSpec>,
    #[serde(default)]
    pub spell: Option<SpellSkillSpec>,
    #[serde(default)]
    pub narration_key: Option<String>,
}

/// Top-level container for all skills declared in `skills.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillsDefinition {
    #[serde(default)]
    pub skills: Vec<SkillDefinition>,
}
