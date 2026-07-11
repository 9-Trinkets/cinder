use super::{default_actor_targeted_speech, default_stat_default_value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActorMovementRulesDefinition {
    #[serde(default)]
    pub default_target_room_id: String,
    #[serde(default)]
    pub target_rules: Vec<ActorMovementTargetRuleDefinition>,
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
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ErrorTextDefinition {
    pub room_missing: String,
    pub cannot_go: String,
    pub actor_not_here: String,
    pub actor_unknown: String,
    pub feature_unknown: String,
    pub unknown_input: String,
    pub dialogue_unavailable: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PresentationTextDefinition {
    pub room_observation: String,
    pub objective: String,
    pub features: String,
    pub people: String,
    pub exits: String,
    pub feature_consumables: String,
    pub actor_speech: String,
    #[serde(default = "default_actor_targeted_speech")]
    pub actor_targeted_speech: String,
    pub actor_departed: String,
    pub actor_arrived: String,
    pub session_ended: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PresentationDefinition {
    #[serde(default)]
    pub error_text: ErrorTextDefinition,
    #[serde(default)]
    pub presentation_text: PresentationTextDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomExitDefinition {
    pub room_id: String,
    pub label: String,
    #[serde(default)]
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomDefinition {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub inspect_text: String,
    #[serde(default)]
    pub allow_rest: bool,
    pub features: Vec<RoomFeatureDefinition>,
    pub exits: Vec<RoomExitDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomFeatureDefinition {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub allow_rest: bool,
    #[serde(default)]
    pub consumables: Vec<ConsumableDefinition>,
    pub inspect_text: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConsumableKind {
    Eat,
    Drink,
    Consume,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumableDefinition {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub kind: ConsumableKind,
    #[serde(default)]
    pub initial_stock: u32,
    #[serde(default)]
    pub hunger_recovery: u32,
    #[serde(default)]
    pub stamina_recovery: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorPromptContext {
    #[serde(default)]
    pub character_notes: Vec<String>,
    #[serde(default)]
    pub subtext_notes: Vec<String>,
    #[serde(default)]
    pub response_notes: Vec<String>,
    #[serde(default)]
    pub behavior_examples: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorDefinition {
    pub id: String,
    pub name: String,
    pub room_id: String,
    #[serde(default)]
    pub initial_stats: BTreeMap<String, i32>,
    #[serde(default)]
    pub initial_pair_stats: BTreeMap<String, BTreeMap<String, i32>>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub inspect_text: String,
    #[serde(default)]
    pub required_consumable_tags: Vec<String>,
    pub prompt_context: ActorPromptContext,
    #[serde(default)]
    pub movement_rules: Option<ActorMovementRulesDefinition>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatDefinition {
    #[serde(default = "default_stat_default_value")]
    pub default: i32,
    #[serde(default)]
    pub min: Option<i32>,
    #[serde(default)]
    pub max: Option<i32>,
    #[serde(default)]
    pub time_step_minutes: Option<u32>,
}

impl StatDefinition {
    pub fn clamp(&self, value: i32) -> i32 {
        let lower = self.min.unwrap_or(i32::MIN);
        let upper = self.max.unwrap_or(i32::MAX);
        value.clamp(lower, upper)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatsDefinition {
    #[serde(default)]
    pub actor: BTreeMap<String, StatDefinition>,
    #[serde(default)]
    pub pair: BTreeMap<String, StatDefinition>,
}
