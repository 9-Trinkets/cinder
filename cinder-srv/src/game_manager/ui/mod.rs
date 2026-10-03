mod minimap;
mod panels;
mod room;
mod sidebar;
pub mod types;

pub use types::*;

use cinder_core::content::loader;
use cinder_core::content::types::{ContentPack, PanelDataSource};
use cinder_core::engine::runtime::{CinderRuntime, PanelOption};
use cinder_core::engine::state::{GamePhase, WorldState};

use super::response;

use self::minimap::build_minimap;
use self::panels::{
    build_action_bar_items, build_active_menu, build_drop_panel_options,
    build_equipment_panel_options, build_interactable_labels, build_look_options,
    build_overflow_actions, build_panel_options, build_talk_options, build_use_panel_options,
};
use self::room::{build_room_consumables, crafted_consumable_labels};
use self::sidebar::{
    build_current_room_items, build_equipped_items, build_inventory, build_party_members,
    build_player_status,
};

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
        .map(|item| ObjectiveItem {
            stage_id: item.stage_id,
            summary: item.summary,
            message: item.message,
            quest_id: item.quest_id,
            quest_title: item.quest_title,
            quest_kind: item.quest_kind,
        })
        .collect();
    let quests = runtime
        .quest_summaries()
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|quest| QuestItem {
            quest_id: quest.quest_id,
            title: quest.title,
            kind: quest.kind,
            status: quest.status,
            summary: quest.summary,
            message: quest.message,
        })
        .collect();
    let (secrets_found, secrets_total) = runtime
        .current_secret_progress()
        .map_err(|error| error.to_string())?;
    let objective_message = objectives
        .iter()
        .find(|o| o.quest_kind.as_deref() == Some("main"))
        .or_else(|| objectives.first())
        .map(|objective| {
            if !objective.message.is_empty() {
                objective.message.clone()
            } else {
                objective.summary.clone()
            }
        })
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
    let party = build_party_members(runtime, &state, content);

    let (action_bar_actions, take_panel_options, give_panel_options) =
        build_action_bar_items(content, &state, &party);
    let panels = PanelOptionGroups {
        take: take_panel_options,
        use_item: build_use_panel_options(content, &state),
        give: give_panel_options,
        drop: build_drop_panel_options(content, &state),
        equipment: build_equipment_panel_options(content, &state),
    };
    let look_options = build_look_options(runtime)?;
    let talk_options = build_talk_options(runtime)?;
    let active_menu = build_active_menu(runtime)?;
    let exit_labels = runtime
        .current_room_exit_labels()
        .map_err(|e| e.to_string())?;
    let interactable_labels = build_interactable_labels(&look_options, &exit_labels);

    let bar_ids: Vec<&str> = action_bar_actions
        .iter()
        .map(|action| action.id.as_str())
        .collect();
    let overflow_actions = build_overflow_actions(runtime, content, &state, &bar_ids, &panels)?;
    let mut panel_options = build_panel_options(runtime, content, &state, panels)?;
    panel_options.extend(sidebar::build_party_order_panels(content, &party));

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
        quests,
        objective_message,
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
        autonomous_actor_dialogue: content.settings.autonomous_actor_dialogue,
        show_player_input: content.settings.show_player_input,
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
        game_over: state.phase != GamePhase::Active,
        party,
        player: build_player_status(&state, content),
        minimap: if content.minimap_shown(&state) {
            build_minimap(&state, content, &current_room_id)
        } else {
            None
        },
        levels_revealed: content.levels_revealed_for_room(&current_room_id)
            || state
                .actor_known_room_ids
                .values()
                .any(|rooms| rooms.iter().any(|r| content.levels_revealed_for_room(r))),
        quests_revealed: content.quests_revealed_for_room(&current_room_id)
            || state
                .actor_known_room_ids
                .values()
                .any(|rooms| rooms.iter().any(|r| content.quests_revealed_for_room(r))),
        current_room_items: build_current_room_items(content, &state, &current_room_id),
        equipped_items: build_equipped_items(&state, content),
        inventory: build_inventory(runtime, content),
        room_consumables: build_room_consumables(runtime, content, &current_room_id),
        crafted_consumable_labels: crafted_consumable_labels(content, &current_room_id),
        show_relationship_sidebar: content.settings.show_relationship_sidebar,
        show_vitals_sidebar: content.vitals_sidebar_shown(&state),
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

/// Ids of inventory items the player can currently drop (present in inventory).
/// Equipped items are stored in `state.equipment` and removed from `player_inventory`.
/// Items remaining in `player_inventory` with count > 0 are held and can be dropped or given.
fn droppable_inventory_items(state: &WorldState) -> Vec<String> {
    let mut ids: Vec<String> = state
        .player_inventory
        .iter()
        .filter(|(_, count)| **count > 0)
        .map(|(item_id, _)| item_id.clone())
        .collect();
    ids.sort();
    ids
}

/// Ids of inventory items the player can currently use:
/// - Held items with a `use_hook` (consumables, potions, scrolls)
/// - Held keys that have an active, available lock action in the current room
fn usable_inventory_items(content: &ContentPack, state: &WorldState) -> Vec<String> {
    let current_room_id = &state.current_room_id;
    let mut ids: Vec<String> = state
        .player_inventory
        .iter()
        .filter(|(item_id, count)| {
            if **count == 0 {
                return false;
            }
            let Some(item) = content.item(item_id) else {
                return false;
            };
            if !item.use_hook.is_empty() {
                return true;
            }
            if item.kind == cinder_core::content::types::ItemKind::Key {
                return content.actions.iter().any(|action| {
                    (action.available.requires_item.as_deref() == Some(item_id.as_str())
                        || action.available.consumes_item.as_deref() == Some(item_id.as_str()))
                        && cinder_core::engine::turn_policies::action_is_available(
                            content,
                            state,
                            action,
                            current_room_id,
                        )
                });
            }
            false
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
