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
    pub group: String,
    #[serde(default)]
    pub sort_order: usize,
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

    // Sets bundle progress (carried from CommandDefinition)
    #[serde(default)]
    pub sets_bundle_progress: Vec<RuleBundleProgressRef>,
    #[serde(default)]
    pub clears_bundle_progress: Vec<RuleBundleProgressRef>,
}

// ---------------------------------------------------------------------------
// Legacy format support: deserializes commands.json / affordances.json format
// and converts to ActionDefinition.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum FlatItemStorageTarget {
    #[default]
    PlayerInventory,
    CurrentRoom,
}

impl From<FlatItemStorageTarget> for ActionItemStorageTarget {
    fn from(v: FlatItemStorageTarget) -> Self {
        match v {
            FlatItemStorageTarget::PlayerInventory => ActionItemStorageTarget::PlayerInventory,
            FlatItemStorageTarget::CurrentRoom => ActionItemStorageTarget::CurrentRoom,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum FlatItemConsumerTarget {
    #[default]
    None,
    Player,
    FirstActorInRoom,
}

impl From<FlatItemConsumerTarget> for ActionItemConsumerTarget {
    fn from(v: FlatItemConsumerTarget) -> Self {
        match v {
            FlatItemConsumerTarget::None => ActionItemConsumerTarget::None,
            FlatItemConsumerTarget::Player => ActionItemConsumerTarget::Player,
            FlatItemConsumerTarget::FirstActorInRoom => {
                ActionItemConsumerTarget::FirstActorInRoom
            }
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct FlatPlayerCommandInput {
    #[serde(default)]
    pub payload_key: String,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct FlatPlayerCommand {
    #[serde(default)]
    pub target_mode: PlayerCommandTargetMode,
    #[serde(default)]
    pub usage: String,
    #[serde(default)]
    pub example: String,
    #[serde(default)]
    pub advances_time: bool,
    #[serde(default)]
    pub input: Option<FlatPlayerCommandInput>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct FlatContentEvent {
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
pub struct FlatAction {
    pub id: String,
    #[serde(default)]
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
    pub content_event: Option<FlatContentEvent>,
    #[serde(default)]
    pub player_command: Option<FlatPlayerCommand>,
    #[serde(default)]
    pub allowed_rooms: Vec<String>,
    #[serde(default)]
    pub available_during: Vec<String>,
    #[serde(default)]
    pub required_bundle_progress: Vec<RuleBundleProgressRef>,
    #[serde(default)]
    pub blocked_by_bundle_progress: Vec<RuleBundleProgressRef>,
    #[serde(default)]
    pub sets_bundle_progress: Vec<RuleBundleProgressRef>,
    #[serde(default)]
    pub clears_bundle_progress: Vec<RuleBundleProgressRef>,
    #[serde(default)]
    pub creates_item: Option<String>,
    #[serde(default)]
    pub creates_item_story_var: Option<String>,
    #[serde(default)]
    pub creates_item_resolve_from_target: bool,
    #[serde(default)]
    pub creates_item_storage: FlatItemStorageTarget,
    #[serde(default)]
    pub consumes_item: Option<String>,
    #[serde(default)]
    pub consumes_item_storage: FlatItemStorageTarget,
    #[serde(default)]
    pub requires_any: Vec<String>,
    #[serde(default)]
    pub requires_any_storage: FlatItemStorageTarget,
    #[serde(default)]
    pub consumes_any: Vec<String>,
    #[serde(default)]
    pub consumes_any_storage: FlatItemStorageTarget,
    #[serde(default)]
    pub item_consumer: FlatItemConsumerTarget,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct FlatCommandsWrapper {
    #[serde(default)]
    pub actions: Vec<FlatAction>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct FlatAffordance {
    pub id: String,
    pub group: String,
    pub prompt_verb: String,
    #[serde(default)]
    pub prompt_reply_verb: String,
    pub command_id: String,
    #[serde(default)]
    pub sort_order: usize,
    #[serde(default = "default_true")]
    pub visible_by_default: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct FlatAffordancesWrapper {
    #[serde(default)]
    pub actions: Vec<FlatAffordance>,
}

impl FlatAction {
    fn into_action(self, affordance: Option<&FlatAffordance>) -> ActionDefinition {
        let item_creation = self.creates_item.map(|item| ActionItemCreation {
            creates_item: item,
            creates_item_story_var: self.creates_item_story_var.unwrap_or_default(),
            creates_item_resolve_from_target: self.creates_item_resolve_from_target,
            storage: self.creates_item_storage.into(),
        });

        let content_event = self.content_event.map(|ce| ActionContentEvent {
            id: ce.id,
            event_text: ce.event_text,
            hook_id: ce.hook_id,
            signals: ce.signals,
            open_menu: ce.open_menu,
        });

        let player_command = self.player_command.map(|pc| ActionPlayerCommand {
            target_mode: pc.target_mode,
            usage: pc.usage,
            example: pc.example,
            advances_time: pc.advances_time,
            input: pc.input.map(|i| ActionPlayerInput {
                payload_key: i.payload_key,
                required: i.required,
            }),
        });

        let ui_group = affordance
            .map(|a| a.group.clone())
            .unwrap_or_else(|| self.group.clone());
        let ui_sort_order = affordance.map(|a| a.sort_order).unwrap_or(100);

        ActionDefinition {
            id: self.id.clone(),
            command: self.command,
            label: String::new(),
            group: self.group,
            phrases: self.player_phrases,
            player_enabled: self.player_enabled,
            target_mode: self.target_mode,
            outcome_mode: self.outcome_mode,
            input_mode: self.input_mode,
            consumable_kind: self.consumable_kind,
            effects: self.effects,
            event_text: self.event_text,
            hook_id: self.hook_id,
            content_event,
            player_command,
            item_creation,
            item_consumer: self.item_consumer.into(),
            available: ActionAvailability {
                requires_actor_in_room: false,
                allowed_rooms: self.allowed_rooms,
                available_during: self.available_during,
                required_bundle_progress: self.required_bundle_progress,
                blocked_by_bundle_progress: self.blocked_by_bundle_progress,
                requires_any: self.requires_any,
                requires_any_storage: self.requires_any_storage.into(),
                consumes_any: self.consumes_any,
                consumes_any_storage: self.consumes_any_storage.into(),
                consumes_item: self.consumes_item,
                consumes_item_storage: self.consumes_item_storage.into(),
            },
            ui: ActionUi {
                bar: true,
                overflow: false,
                panel: None,
                group: ui_group,
                sort_order: ui_sort_order,
            },
            npc: affordance.map(|a| ActionNpc {
                prompt_verb: a.prompt_verb.clone(),
                prompt_reply_verb: a.prompt_reply_verb.clone(),
                visible_by_default: a.visible_by_default,
                actor_scope: None,
            }),
            sets_bundle_progress: self.sets_bundle_progress,
            clears_bundle_progress: self.clears_bundle_progress,
        }
    }
}

pub fn convert_legacy_commands(
    commands_json: &str,
    affordances_json: Option<&str>,
) -> Result<Vec<ActionDefinition>, String> {
    let flat: FlatCommandsWrapper =
        serde_json::from_str(commands_json).map_err(|e| format!("commands.json parse error: {e}"))?;

    let affordances: Vec<FlatAffordance> = match affordances_json {
        Some(json) => {
            let wrapper: FlatAffordancesWrapper = serde_json::from_str(json)
                .map_err(|e| format!("affordances.json parse error: {e}"))?;
            wrapper.actions
        }
        None => Vec::new(),
    };

    let affordance_map: std::collections::HashMap<&str, &FlatAffordance> = affordances
        .iter()
        .map(|a| (a.command_id.as_str(), a))
        .collect();

    Ok(flat
        .actions
        .into_iter()
        .map(|flat_action| {
            let affordance = affordance_map.get(flat_action.id.as_str()).copied();
            flat_action.into_action(affordance)
        })
        .collect())
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
