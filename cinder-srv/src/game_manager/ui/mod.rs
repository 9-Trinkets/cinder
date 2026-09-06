mod minimap;
mod panels;
mod room;
mod sidebar;

use cinder_core::content::loader;
use cinder_core::content::types::{PanelDataSource, UiTextDefinition};
use cinder_core::engine::runtime::{ActClosure, CinderRuntime, PanelOption};
use cinder_core::engine::state::WorldState;
use serde::Serialize;
use std::collections::BTreeMap;

use super::response;

use self::panels::{
    build_action_bar_and_take, build_active_menu, build_drop_panel_options,
    build_interactable_labels, build_look_options, build_overflow_actions, build_panel_options,
    build_talk_options,
};
use self::minimap::build_minimap;
use self::room::{build_room_consumables, crafted_consumable_labels};
use self::sidebar::{
    build_current_room_items, build_equipped_items, build_inventory, build_party_members,
    build_player_status,
};

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
}

/// A group of same-named followers in the party.
#[derive(Clone, Serialize)]
pub struct PartyMember {
    pub label: String,
    pub count: u32,
    /// Follower level (per-actor). Rendered only once levels are revealed.
    pub level: u32,
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
    pub channel_surfing_only: bool,
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
    pub inventory: Vec<InventoryItem>,
    /// Items worn in the player's equipment slots (slot → label).
    pub equipped_items: Vec<EquippedItem>,
    /// Followers grouped by name (e.g. "dark golem" ×2).
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
    pub relationship_pairs: Vec<cinder_core::engine::runtime::RelationshipPair>,
    /// Whether the sidebar shows the Vitals + Level sections (combat packs).
    pub show_vitals_sidebar: bool,
    pub theme: cinder_core::content::types::ThemeDefinition,
}

pub(super) fn build_ui_snapshot(
    runtime: &CinderRuntime,
    pack_id: &str,
    transcript_lines: &[String],
) -> Result<UiSnapshot, String> {
    let time_label = runtime
        .current_time_label()
        .map_err(|error| error.to_string())?;
    let day_number = runtime
        .current_day_number()
        .map_err(|error| error.to_string())?;
    let objectives: Vec<ObjectiveItem> = runtime
        .current_objective_summaries()
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|(summary, message)| ObjectiveItem { summary, message })
        .collect();
    let (progress_completed, progress_total) = runtime
        .current_objective_progress()
        .map_err(|error| error.to_string())?;
    let (secrets_found, secrets_total) = runtime
        .current_secret_progress()
        .map_err(|error| error.to_string())?;
    let objective_message = objectives
        .first()
        .map(|objective| objective.message.clone())
        .unwrap_or_default();
    let locales = loader::available_locales(&loader::pack_dir(pack_id))
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|locale| LocaleItem {
            code: locale.code,
            label: locale.label,
        })
        .collect();
    let content = runtime.content();

    let current_room_id = runtime
        .current_room_id()
        .map_err(|error| error.to_string())?;
    let current_room_name = content
        .room(&current_room_id)
        .map(|room| room.title.clone())
        .unwrap_or(current_room_id.clone());
    let followed_actor_name = runtime
        .followed_actor_id()
        .map_err(|error| error.to_string())?
        .and_then(|id| runtime.actor_display_name(&id).ok().flatten());

    let state = runtime.export_state().map_err(|e| e.to_string())?;

    let (action_bar_actions, take_panel_options) =
        build_action_bar_and_take(content, &state);
    let drop_panel_options = build_drop_panel_options(content, &state);
    let look_options = build_look_options(runtime)?;
    let talk_options = build_talk_options(runtime)?;
    let active_menu = build_active_menu(runtime)?;
    let interactable_labels = build_interactable_labels(&look_options);

    let bar_ids: Vec<&str> = action_bar_actions
        .iter()
        .map(|action| action.id.as_str())
        .collect();
    let overflow_actions = build_overflow_actions(
        runtime,
        content,
        &state,
        &bar_ids,
        &drop_panel_options,
    )?;
    let panel_options = build_panel_options(
        runtime,
        content,
        &state,
        take_panel_options,
        drop_panel_options,
    )?;

    Ok(UiSnapshot {
        pack_id: pack_id.to_string(),
        title: content.opening.title.clone(),
        time_label,
        npc_tick_interval_ms: content.settings.npc_tick_interval_ms,
        day_number,
        current_room_name,
        followed_actor_name,
        help_text: runtime.help_text(),
        about_body: content.ui_text.about_body.clone(),
        current_locale: content.locale.clone(),
        locale_options: locales,
        objectives,
        objective_message,
        progress_completed,
        progress_total,
        secrets_found,
        secrets_total,
        rooms: menu_option_data(
            runtime
                .panel_options(&PanelDataSource::Exits)
                .map_err(|error| error.to_string())?,
        ),
        follow_options: menu_option_data(
            runtime
                .follow_actor_options()
                .map_err(|error| error.to_string())?,
        ),
        channel_surfing_only: content.settings.channel_surfing_only,
        action_bar_actions,
        overflow_actions,
        look_options,
        interactable_labels,
        talk_options,
        panel_options,
        active_menu,
        ui_text: content.ui_text.clone(),
        act_closure: if content.settings.show_act_closure {
            response::act_closure_data(runtime, transcript_lines)
        } else {
            None
        },
        game_closure: response::game_closure_data(runtime, transcript_lines),
        party: build_party_members(runtime, &state, content),
        player: build_player_status(&state, content),
        minimap: build_minimap(&state, content, &current_room_id),
        levels_revealed: content.levels_revealed_for_room(&current_room_id),
        current_room_items: build_current_room_items(content, &state, &current_room_id),
        equipped_items: build_equipped_items(&state, content),
        inventory: build_inventory(runtime, content),
        room_consumables: build_room_consumables(runtime, content, &current_room_id),
        crafted_consumable_labels: crafted_consumable_labels(content, &current_room_id),
        show_relationship_sidebar: content.settings.show_relationship_sidebar,
        show_vitals_sidebar: content.settings.show_vitals_sidebar,
        relationship_pairs: if content.settings.show_relationship_sidebar {
            runtime.relationship_pairs().unwrap_or_default()
        } else {
            Vec::new()
        },
        theme: content.settings.theme.clone(),
    })
}

fn menu_option_data(options: Vec<PanelOption>) -> Vec<MenuOptionData> {
    options
        .into_iter()
        .map(|option| MenuOptionData {
            id: option.id,
            title: option.title,
            menu_text: option.menu_text,
        })
        .collect()
}

/// Ids of inventory items the player can currently drop (present and not
/// equipped). Equipped items must be unequipped before they can be dropped.
fn droppable_inventory_items(state: &WorldState) -> Vec<String> {
    let mut ids: Vec<String> = state
        .player_inventory
        .iter()
        .filter(|(item_id, count)| {
            **count > 0
                && !state
                    .equipment
                    .values()
                    .any(|equipped| equipped.as_str() == item_id.as_str())
        })
        .map(|(item_id, _)| item_id.clone())
        .collect();
    ids.sort();
    ids
}

fn append_stage_menu_overflow_actions(
    overflow_actions: &mut Vec<OverflowAction>,
    content: &cinder_core::content::types::ContentPack,
    active_stages: &[String],
) {
    for stage_id in active_stages {
        let Some(stage) = content
            .beats
            .stages
            .iter()
            .find(|stage| &stage.id == stage_id)
        else {
            continue;
        };
        let Some(menu) = content
            .menus
            .iter()
            .find(|menu| &menu.stage_id == stage_id && !menu.dynamic && !menu.options.is_empty())
        else {
            continue;
        };
        let waits_for_menu_selection = stage
            .advance_signals
            .iter()
            .any(|signal| signal.signal() == format!("menu_selected:{}", menu.id));
        if !waits_for_menu_selection {
            continue;
        }
        for option in &menu.options {
            if overflow_actions.iter().any(|action| action.id == option.id) {
                continue;
            }
            overflow_actions.push(OverflowAction {
                id: option.id.clone(),
                label: option.title.clone(),
                group: "support".to_string(),
                usage: String::new(),
                panel: String::new(),
                panel_config: None,
            });
        }
    }
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
