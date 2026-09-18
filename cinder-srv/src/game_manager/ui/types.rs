use cinder_core::content::types::{PanelDataSource, ThemeDefinition, UiTextDefinition};
use cinder_core::engine::runtime::{ActClosure, RelationshipPair};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Serialize)]
pub struct LocaleItem {
    pub code: String,
    pub label: String,
}

#[derive(Clone, Serialize)]
pub struct ObjectiveItem {
    pub summary: String,
    pub message: String,
}

#[derive(Clone, Serialize)]
pub struct InventoryItem {
    pub label: String,
    pub count: u32,
    /// Present for loose room items so the UI can dispatch a generic
    /// `take <id>` command.
    pub id: Option<String>,
}

/// An item worn in one of the player's equipment slots.
#[derive(Clone, Serialize)]
pub struct EquippedItem {
    pub slot: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

/// One living follower and its current combat directive.
#[derive(Clone, Serialize)]
pub struct PartyMember {
    pub id: String,
    pub label: String,
    pub level: u32,
    pub hp: u32,
    pub hp_max: u32,
    pub order: String,
    pub order_panel: String,
    pub inventory: Vec<InventoryItem>,
    pub equipped_items: Vec<EquippedItem>,
}

/// A single stat value shown on the player's status.
#[derive(Clone, Serialize)]
pub struct StatValue {
    pub id: String,
    pub value: i32,
}

/// The player's vitals and other stats for the sidebar.
#[derive(Clone, Serialize)]
pub struct PlayerStatus {
    pub hp: u32,
    pub hp_max: u32,
    pub stats: Vec<StatValue>,
    /// The player's (per-actor) level.
    pub level: u32,
    /// The player's XP toward the next level (or cumulative if no curve).
    pub xp: u32,
    /// XP required to advance from the current level to the next (0 = maxed).
    pub xp_max: u32,
}

#[derive(Clone, Serialize)]
pub struct MinimapRoom {
    pub id: String,
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub current: bool,
    pub visited: bool,
}

#[derive(Clone, Serialize)]
pub struct MinimapConnection {
    pub from: String,
    pub to: String,
}

#[derive(Clone, Serialize)]
pub struct MinimapData {
    pub id: String,
    pub label: String,
    pub fully_revealed: bool,
    pub visited_count: usize,
    pub total_count: Option<usize>,
    pub rooms: Vec<MinimapRoom>,
    pub connections: Vec<MinimapConnection>,
}

#[derive(Clone, Serialize)]
pub struct ConsumableInfo {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub stock: u32,
    pub is_crafted: bool,
}

#[derive(Clone, Serialize)]
pub struct RoomConsumableGroup {
    pub feature_label: String,
    pub items: Vec<ConsumableInfo>,
}

#[derive(Clone, Serialize)]
pub struct ActionBarAction {
    pub id: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub panel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub panel_config: Option<PanelConfigData>,
}

#[derive(Clone, Serialize)]
pub struct PanelConfigData {
    pub title: String,
    pub prompt: String,
    pub data_source: PanelDataSource,
    pub on_select: cinder_core::content::types::PanelSelectAction,
}

#[derive(Clone, Serialize)]
pub struct PanelOptionData {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub disabled: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub selected: bool,
}

#[derive(Clone, Serialize)]
pub struct OverflowAction {
    pub id: String,
    pub label: String,
    pub group: String,
    pub usage: String,
    /// Panel this action opens (e.g. the speak/talk picker), if any.
    #[serde(default)]
    pub panel: String,
    #[serde(default)]
    pub panel_config: Option<PanelConfigData>,
}

#[derive(Clone, Serialize)]
pub struct LookOptionData {
    pub id: String,
    pub title: String,
    pub command: String,
}

#[derive(Clone, Serialize)]
pub struct MenuOptionData {
    pub id: String,
    pub title: String,
    pub menu_text: String,
}

#[derive(Clone, Serialize)]
pub struct ActiveMenuData {
    pub prompt: String,
    pub options: Vec<MenuOptionData>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub max_selections: usize,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub min_selections: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selected_ids: Vec<String>,
}

fn is_zero(value: &usize) -> bool {
    *value == 0
}

#[derive(Clone, Serialize)]
pub struct UiSnapshot {
    pub pack_id: String,
    pub title: String,
    pub time_label: String,
    pub npc_tick_interval_ms: u64,
    pub day_number: u32,
    pub current_room_name: String,
    pub followed_actor_name: Option<String>,
    pub help_text: String,
    pub about_body: String,
    pub current_locale: String,
    pub locale_options: Vec<LocaleItem>,
    pub objectives: Vec<ObjectiveItem>,
    pub objective_message: String,
    pub progress_completed: usize,
    pub progress_total: usize,
    pub secrets_found: usize,
    pub secrets_total: usize,
    pub rooms: Vec<MenuOptionData>,
    pub follow_options: Vec<MenuOptionData>,
    pub autonomous_actor_dialogue: bool,
    /// Whether the web UI should show the player's free-text input box.
    /// False for spectator packs (e.g. Aera) where the player watches rather
    /// than types commands.
    pub show_player_input: bool,
    pub action_bar_actions: Vec<ActionBarAction>,
    pub overflow_actions: Vec<OverflowAction>,
    pub look_options: Vec<LookOptionData>,
    /// Labels of actors/features/items present in the current room, for
    /// transcript highlighting.
    #[serde(default)]
    pub interactable_labels: Vec<String>,
    pub talk_options: Vec<MenuOptionData>,
    #[serde(default)]
    pub panel_options: BTreeMap<String, Vec<PanelOptionData>>,
    pub active_menu: Option<ActiveMenuData>,
    pub ui_text: UiTextDefinition,
    pub act_closure: Option<ActClosure>,
    pub game_closure: Option<ActClosure>,
    /// Whether the game has ended (phase-derived, independent of whether any
    /// game-closure text is configured). The single source of truth the web UI
    /// uses to lock command input at game over.
    #[serde(default)]
    pub game_over: bool,
    pub inventory: Vec<InventoryItem>,
    /// Items worn in the player's equipment slots (slot → label).
    pub equipped_items: Vec<EquippedItem>,
    /// Living followers, listed individually so each can receive an order.
    pub party: Vec<PartyMember>,
    /// The player's vitals and other stats for the sidebar.
    pub player: PlayerStatus,
    pub minimap: Option<MinimapData>,
    /// Whether party levels are visible yet. Derived from the content's
    /// `level_reveal_room_prefix`: false until the player has travelled to a
    /// room on that board. Level info stays hidden to reward descent.
    pub levels_revealed: bool,
    /// Loose items lying in the current room (dropped there).
    pub current_room_items: Vec<InventoryItem>,
    pub room_consumables: Vec<RoomConsumableGroup>,
    pub crafted_consumable_labels: Vec<String>,
    pub show_relationship_sidebar: bool,
    pub relationship_pairs: Vec<RelationshipPair>,
    /// Whether the sidebar shows the Vitals + Level sections (combat packs).
    /// Honors `show_vitals_sidebar` and the `vitals_sidebar_story_var` gate.
    pub show_vitals_sidebar: bool,
    pub theme: ThemeDefinition,
}

#[cfg(test)]
mod tests {
    use super::{ActiveMenuData, MenuOptionData};

    #[test]
    fn zero_selection_counts_are_omitted_from_menu_payload() {
        let payload = serde_json::to_value(ActiveMenuData {
            prompt: "Choose".to_string(),
            options: vec![MenuOptionData {
                id: "1".to_string(),
                title: "First".to_string(),
                menu_text: "First option".to_string(),
            }],
            max_selections: 0,
            min_selections: 0,
            selected_ids: vec![],
        })
        .expect("serialize active menu");

        assert!(payload.get("max_selections").is_none());
        assert!(payload.get("min_selections").is_none());
        assert!(payload.get("selected_ids").is_none());
    }
}
