use super::{ConsumableKind, default_true};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CommandOutcomeMode {
    #[default]
    Event,
    Dialogue,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CommandInputMode {
    #[default]
    None,
    FreeformText,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CommandEffect {
    ObserveRoom,
    MoveActor,
    ObserveFeature,
    ObserveActor,
    ConsumeTargetedConsumable,
    RememberInRoom,
    RememberWithTargetActor,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CommandTargetMode {
    #[default]
    None,
    Room,
    Actor,
    ActorOptional,
    Feature,
    Consumable,
    ContextLabel,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContentEventDefinition {
    pub id: String,
    #[serde(default)]
    pub event_text: String,
    #[serde(default)]
    pub hook_id: String,
    #[serde(default)]
    pub signals: Vec<String>,
    #[serde(default)]
    pub open_menu: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayerCommandInputMetadata {
    #[serde(default)]
    pub payload_key: String,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum PlayerCommandTargetMode {
    #[default]
    None,
    RoomReference,
    ActorReference,
    FirstActorInRoom,
    ActorOrFeatureReference,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayerCommandMetadata {
    #[serde(default)]
    pub target_mode: PlayerCommandTargetMode,
    #[serde(default)]
    pub usage: String,
    #[serde(default)]
    pub example: String,
    #[serde(default)]
    pub advances_time: bool,
    #[serde(default)]
    pub input: Option<PlayerCommandInputMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandDefinition {
    pub id: String,
    pub command: String,
    #[serde(default)]
    pub group: String,
    #[serde(default)]
    pub player_enabled: bool,
    #[serde(default)]
    pub player_phrases: Vec<String>,
    #[serde(default)]
    pub outcome_mode: CommandOutcomeMode,
    #[serde(default)]
    pub input_mode: CommandInputMode,
    #[serde(default)]
    pub target_mode: CommandTargetMode,
    #[serde(default)]
    pub consumable_kind: Option<ConsumableKind>,
    #[serde(default)]
    pub effects: Vec<CommandEffect>,
    #[serde(default)]
    pub hook_id: String,
    #[serde(default)]
    pub event_text: String,
    #[serde(default)]
    pub content_event: Option<ContentEventDefinition>,
    #[serde(default)]
    pub player_command: Option<PlayerCommandMetadata>,
    #[serde(default)]
    pub allowed_rooms: Vec<String>,
    #[serde(default)]
    pub creates_item: Option<String>,
    #[serde(default)]
    pub consumes_item: Option<String>,
    #[serde(default)]
    pub requires_any: Vec<String>,
    #[serde(default)]
    pub consumes_any: Vec<String>,
    #[serde(default)]
    pub available_during: Vec<String>,
}

impl CommandDefinition {
    pub fn has_effect(&self, effect: CommandEffect) -> bool {
        self.effects.contains(&effect)
    }

    pub fn has_any_effect(&self, effects: &[CommandEffect]) -> bool {
        effects.iter().any(|effect| self.has_effect(*effect))
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommandsDefinition {
    #[serde(default)]
    pub actions: Vec<CommandDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffordanceDefinition {
    pub id: String,
    pub group: String,
    pub prompt_verb: String,
    #[serde(default)]
    pub prompt_reply_verb: String,
    pub command_id: String,
    #[serde(default = "default_affordance_sort_order")]
    pub sort_order: usize,
    #[serde(default = "default_true")]
    pub visible_by_default: bool,
}

fn default_affordance_sort_order() -> usize {
    100
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AffordancesDefinition {
    #[serde(default)]
    pub actions: Vec<AffordanceDefinition>,
}
