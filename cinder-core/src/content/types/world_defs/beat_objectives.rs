use super::super::BeatObjectiveProgressRef;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BeatObjectivesDefinition {
    #[serde(default)]
    pub objectives: Vec<BeatObjectiveDefinition>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BeatObjectiveDefinition {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub stage_ids: Vec<String>,
    #[serde(default)]
    pub progress: BeatObjectiveProgressDefinition,
    #[serde(default)]
    pub completion: BeatObjectiveCompletionDefinition,
    #[serde(default)]
    pub guidance: BeatObjectiveGuidanceDefinition,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct BeatObjectiveProgressDefinition {
    #[serde(default)]
    pub keys: Vec<BeatObjectiveProgressKeyDefinition>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct BeatObjectiveProgressKeyDefinition {
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub label: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BeatObjectiveCompletionDefinition {
    #[serde(default)]
    pub mark_actor_complete_on: Vec<BeatObjectiveCompletionTrigger>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BeatObjectiveGuidanceDefinition {
    #[serde(default)]
    pub prompt_note_if_actor_incomplete: String,
    #[serde(default)]
    pub prompt_note_if_others_incomplete: String,
    #[serde(default)]
    pub prioritize: Vec<BeatObjectiveAffordancePriorityDefinition>,
    #[serde(default)]
    pub conditional: Vec<BeatObjectiveConditionalGuidanceDefinition>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BeatObjectiveConditionalGuidanceDefinition {
    #[serde(default)]
    pub required_objective_progress: Vec<BeatObjectiveProgressRef>,
    #[serde(default)]
    pub blocked_by_objective_progress: Vec<BeatObjectiveProgressRef>,
    #[serde(default)]
    pub prompt_note: String,
    #[serde(default)]
    pub prioritize: Vec<BeatObjectiveAffordancePriorityDefinition>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BeatObjectiveAffordancePriorityDefinition {
    #[serde(default)]
    pub command_id: String,
    #[serde(default)]
    pub target: BeatObjectiveAffordanceTarget,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BeatObjectiveAffordanceTarget {
    #[default]
    Any,
    Actor,
    Room,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BeatObjectiveCompletionTrigger {
    SpeechToActor,
    SpeechToRoom,
}