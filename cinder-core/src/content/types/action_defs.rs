use super::{
    CommandEffect, CommandInputMode, CommandOutcomeMode, CommandTargetMode, ConsumableKind,
    ItemStorageTarget, PlayerCommandTargetMode, RuleBundleProgressRef,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActionAvailability {
    #[serde(default)]
    pub requires_actor_in_room: bool,
    #[serde(default)]
    pub allowed_rooms: Vec<String>,
    #[serde(default)]
    pub available_during: Vec<String>,
    #[serde(default)]
    pub required_bundle_progress: Vec<RuleBundleProgressRef>,
    #[serde(default)]
    pub blocked_by_bundle_progress: Vec<RuleBundleProgressRef>,
    #[serde(default)]
    pub requires_any: Vec<String>,
    #[serde(default)]
    pub requires_any_storage: ActionItemStorageTarget,
    #[serde(default)]
    pub consumes_any: Vec<String>,
    #[serde(default)]
    pub consumes_any_storage: ActionItemStorageTarget,
    #[serde(default)]
    pub consumes_item: Option<String>,
    #[serde(default)]
    pub consumes_item_storage: ActionItemStorageTarget,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionItemStorageTarget {
    #[default]
    PlayerInventory,
    CurrentRoom,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActionUi {
    #[serde(default = "default_true")]
    pub bar: bool,
    #[serde(default)]
    pub overflow: bool,
    #[serde(default)]
    pub panel: Option<String>,
    #[serde(default)]
    pub panel_config: Option<PanelConfig>,
    #[serde(default)]
    pub group: String,
    #[serde(default)]
    pub sort_order: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PanelConfig {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub data_source: PanelDataSource,
    #[serde(default)]
    pub on_select: PanelSelectAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PanelDataSource {
    ActorsInRoom,
    Exits,
    Features,
}

impl Default for PanelDataSource {
    fn default() -> Self {
        Self::ActorsInRoom
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PanelSelectAction {
    ExecuteCommand,
    PrefillInput,
    SwitchRoom,
    FollowActor,
}

impl Default for PanelSelectAction {
    fn default() -> Self {
        Self::ExecuteCommand
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActionNpc {
    #[serde(default)]
    pub prompt_verb: String,
    #[serde(default)]
    pub prompt_reply_verb: String,
    #[serde(default = "default_true")]
    pub visible_by_default: bool,
    #[serde(default)]
    pub actor_scope: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActionContentEvent {
    #[serde(default)]
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
pub struct ActionPlayerInput {
    #[serde(default)]
    pub payload_key: String,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActionPlayerCommand {
    #[serde(default)]
    pub target_mode: PlayerCommandTargetMode,
    #[serde(default)]
    pub usage: String,
    #[serde(default)]
    pub example: String,
    #[serde(default)]
    pub advances_time: bool,
    #[serde(default)]
    pub input: Option<ActionPlayerInput>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActionItemCreation {
    #[serde(default)]
    pub creates_item: String,
    #[serde(default)]
    pub creates_item_story_var: String,
    #[serde(default)]
    pub creates_item_resolve_from_target: bool,
    #[serde(default)]
    pub storage: ActionItemStorageTarget,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActionDefinition {
    pub id: String,
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub group: String,

    // Player input
    #[serde(default)]
    pub phrases: Vec<String>,
    #[serde(default)]
    pub player_enabled: bool,

    // Behavior
    #[serde(default)]
    pub target_mode: CommandTargetMode,
    #[serde(default)]
    pub outcome_mode: CommandOutcomeMode,
    #[serde(default)]
    pub input_mode: CommandInputMode,
    #[serde(default)]
    pub consumable_kind: Option<ConsumableKind>,
    #[serde(default)]
    pub effects: Vec<CommandEffect>,
    #[serde(default)]
    pub event_text: String,
    #[serde(default)]
    pub hook_id: String,
    #[serde(default)]
    pub content_event: Option<ActionContentEvent>,
    #[serde(default)]
    pub player_command: Option<ActionPlayerCommand>,
    #[serde(default)]
    pub item_creation: Option<ActionItemCreation>,
    #[serde(default)]
    pub item_consumer: ActionItemConsumerTarget,

    // Availability
    #[serde(default)]
    pub available: ActionAvailability,

    // UI
    #[serde(default)]
    pub ui: ActionUi,

    // NPC
    #[serde(default)]
    pub npc: Option<ActionNpc>,

    #[serde(default)]
    pub sets_bundle_progress: Vec<RuleBundleProgressRef>,
    #[serde(default)]
    pub clears_bundle_progress: Vec<RuleBundleProgressRef>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ActionItemConsumerTarget {
    #[default]
    None,
    Player,
    FirstActorInRoom,
}

impl From<ActionItemStorageTarget> for ItemStorageTarget {
    fn from(v: ActionItemStorageTarget) -> Self {
        match v {
            ActionItemStorageTarget::PlayerInventory => ItemStorageTarget::PlayerInventory,
            ActionItemStorageTarget::CurrentRoom => ItemStorageTarget::CurrentRoom,
        }
    }
}

impl ActionDefinition {
    pub fn has_effect(&self, effect: CommandEffect) -> bool {
        self.effects.contains(&effect)
    }

    pub fn has_any_effect(&self, effects: &[CommandEffect]) -> bool {
        effects.iter().any(|effect| self.has_effect(*effect))
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActionsDefinition {
    #[serde(default)]
    pub actions: Vec<ActionDefinition>,
}
