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
    FollowActor,
    DropItem,
    PickUpItem,
    AttackTarget,
    /// Moves one unit of the action's item from the player's inventory into
    /// its equipment slot (replacing anything there, which returns to the
    /// inventory) and applies its stat bonuses via effective stat reads.
    EquipItem,
    /// Returns the action's item from its equipment slot to the inventory.
    UnequipItem,
    /// Consumes one unit of the action's item from the inventory and fires
    /// the item's `use_hook` content rule (e.g. a potion healing hp).
    UseItem,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ItemStorageTarget {
    #[default]
    PlayerInventory,
    CurrentRoom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ItemConsumerTarget {
    #[default]
    None,
    Player,
    FirstActorInRoom,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuleBundleProgressRef {
    #[serde(default)]
    pub bundle_id: String,
    #[serde(default)]
    pub key: String,
}
