use super::super::Condition;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoomExitDefinition {
    pub room_id: String,
    pub label: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    /// Optional label shown in the room-switch menu in place of the
    /// destination room title. When absent the room title is used.
    #[serde(default)]
    pub menu_label: Option<String>,
    /// Conditions that must hold for this exit to be visible to and
    /// traversable by the player. Until they do the exit is hidden from the
    /// room's prose, the room-switch menu, and `move` resolution.
    ///
    /// Gated exits are also treated as *non*-connectors by the room graph
    /// (`adjacent_room_ids` / `reachable_room_ids`): revealing them lets the
    /// player cross, but the two sides remain separate boards for NPC activity,
    /// so the destination level only comes alive once the player is on it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub when: Vec<Condition>,
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
    /// Loose items lying in the room when play starts (item id → count), which
    /// the player can pick up with Take.
    #[serde(default)]
    pub items: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoomDescriptionOverride {
    /// This override replaces the room's text once every condition holds. An
    /// override with no conditions never applies.
    #[serde(default)]
    pub when: Vec<Condition>,
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
    /// Items this feature holds, item id → how many are there when play starts.
    /// The player can take them (unless the item is a consumable, which is used
    /// in place), and a count of zero marks an item that belongs here and shows
    /// up once something puts it there.
    #[serde(default)]
    pub items: BTreeMap<String, u32>,
    /// How the feature's items are listed when it is inspected, with `{items}`
    /// and `{feature_label}`. Empty uses the pack's `feature_items` line.
    #[serde(default)]
    pub items_text: String,
    pub inspect_text: String,
}
