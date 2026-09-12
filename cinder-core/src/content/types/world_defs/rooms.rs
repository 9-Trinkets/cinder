use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomExitDefinition {
    pub room_id: String,
    pub label: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    /// Optional label shown in the room-switch menu in place of the
    /// destination room title. When absent the room title is used.
    #[serde(default)]
    pub menu_label: Option<String>,
    /// A story var that must be truthy for this exit to be visible to and
    /// traversable by the player. Until revealed the exit is hidden from the
    /// room's prose, the room-switch menu, and `move` resolution.
    ///
    /// Gated exits are also treated as *non*-connectors by the room graph
    /// (`adjacent_room_ids` / `reachable_room_ids`): revealing them lets the
    /// player cross, but the two sides remain separate boards for NPC activity,
    /// so the destination level only comes alive once the player is on it.
    #[serde(default)]
    pub requires_story_var: String,
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
    /// Conditional description overrides, checked in order; the first whose
    /// condition matches the current world state replaces the room's
    /// `summary`/`inspect_text`. Keeps prose fresh as the world changes (e.g.
    /// the throne room reads differently after its king falls).
    #[serde(default)]
    pub descriptions: Vec<RoomDescriptionOverride>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomDescriptionOverride {
    /// When non-empty, this override applies only once the named actor has
    /// been defeated.
    #[serde(default)]
    pub actor_defeated: String,
    pub summary: String,
    pub inspect_text: String,
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
    VideoClip,
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