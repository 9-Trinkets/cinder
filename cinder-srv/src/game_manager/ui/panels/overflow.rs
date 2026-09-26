use super::panel_config_data;
use super::super::{OverflowAction, PanelConfigData, append_stage_menu_overflow_actions};
use cinder_core::content::types::{ActionDefinition, ContentPack, PanelDataSource, PanelSelectAction};
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::WorldState;
use cinder_core::engine::turn_policies::action_is_available;
use super::super::PanelOptionData;

/// Display title for an overflow action button: the authored `label` (e.g.
/// "Use Moss Poultice").
pub(crate) fn overflow_action_title(action: &ActionDefinition) -> String {
    action.label.clone()
}

pub(crate) fn build_overflow_actions(
    runtime: &CinderRuntime,
    content: &ContentPack,
    state: &WorldState,
    bar_ids: &[&str],
    take_panel_options: &[PanelOptionData],
    use_panel_options: &[PanelOptionData],
    give_panel_options: &[PanelOptionData],
    drop_panel_options: &[PanelOptionData],
    equipment_panel_options: &[PanelOptionData],
) -> Result<Vec<OverflowAction>, String> {
    let has_talk = bar_ids.contains(&"speak") || bar_ids.contains(&"talk");
    let modal_covered: Vec<&str> = vec!["inspect_feature", "inspect_actor"];
    let current_room_id = runtime.current_room_id().unwrap_or_default();
    let mut overflow_actions: Vec<OverflowAction> = content
        .actions
        .iter()
        .filter(|a| {
            if !a.player_enabled || bar_ids.contains(&a.id.as_str()) {
                return false;
            }
            if modal_covered.contains(&a.id.as_str()) {
                return false;
            }
            if (a.id == "speak" || a.id == "talk") && has_talk {
                return false;
            }
            if a.id == "take" && take_panel_options.is_empty() {
                return false;
            }
            if a.id == "use" && use_panel_options.is_empty() {
                return false;
            }
            if a.id == "drop" && drop_panel_options.is_empty() {
                return false;
            }
            if a.id == "give" && give_panel_options.is_empty() {
                return false;
            }
            action_is_available(content, state, a, &current_room_id)
        })
        .map(|a| {
            let label = overflow_action_title(a);
            let usage = a
                .player_command
                .as_ref()
                .map(|player_command| player_command.usage.clone())
                .unwrap_or_default();
            let group = if (a.id == "take" || a.id == "use" || a.id == "drop" || a.id == "give") && a.ui.group.is_empty() {
                "items".to_string()
            } else {
                a.ui.group.clone()
            };
            OverflowAction {
                id: a.id.clone(),
                label,
                group,
                usage,
                panel: a.ui.panel.clone().unwrap_or_default(),
                panel_config: a.ui.panel_config.as_ref().map(panel_config_data),
            }
        })
        .collect();

    if let Ok(active_stages) = runtime.active_stage_ids() {
        append_stage_menu_overflow_actions(&mut overflow_actions, content, &active_stages);
    }

    // Surface the generic `take <item>` overflow action in the "items" group
    // directly above give and drop when there are takeable items.
    if !take_panel_options.is_empty() && !overflow_actions.iter().any(|a| a.id == "take") {
        let take_action = OverflowAction {
            id: "take".to_string(),
            label: content.ui_text.take_label.clone(),
            group: "items".to_string(),
            usage: "take <item>".to_string(),
            panel: "take".to_string(),
            panel_config: Some(PanelConfigData {
                title: content.ui_text.take_label.clone(),
                prompt: String::new(),
                data_source: PanelDataSource::LooseRoomItems,
                on_select: PanelSelectAction::ExecuteCommand,
            }),
        };
        if let Some(pos) = overflow_actions.iter().position(|a| a.id == "give" || a.id == "drop") {
            overflow_actions.insert(pos, take_action);
        } else {
            overflow_actions.push(take_action);
        }
    }

    // Surface the generic `use <item>` overflow action in the "items" group
    // directly above give and drop when there are usable items.
    if !use_panel_options.is_empty() && !overflow_actions.iter().any(|a| a.id == "use") {
        let use_action = OverflowAction {
            id: "use".to_string(),
            label: content.ui_text.use_label.clone(),
            group: "items".to_string(),
            usage: "use <item>".to_string(),
            panel: "use".to_string(),
            panel_config: Some(PanelConfigData {
                title: content.ui_text.use_label.clone(),
                prompt: String::new(),
                data_source: PanelDataSource::InventoryItems,
                on_select: PanelSelectAction::ExecuteCommand,
            }),
        };
        if let Some(pos) = overflow_actions.iter().position(|a| a.id == "give" || a.id == "drop") {
            overflow_actions.insert(pos, use_action);
        } else {
            overflow_actions.push(use_action);
        }
    }

    // Surface the generic `give <item> to <companion>` overflow action in the "items" group
    // directly above drop when the player holds something droppable and has companions.
    if !give_panel_options.is_empty() && !overflow_actions.iter().any(|a| a.id == "give") {
        let give_action = OverflowAction {
            id: "give".to_string(),
            label: "Give".to_string(),
            group: "items".to_string(),
            usage: "give <item> to <companion>".to_string(),
            panel: "give".to_string(),
            panel_config: Some(PanelConfigData {
                title: "Give to Companion".to_string(),
                prompt: "Choose an item to give".to_string(),
                data_source: PanelDataSource::InventoryItems,
                on_select: PanelSelectAction::ExecuteCommand,
            }),
        };
        if let Some(drop_pos) = overflow_actions.iter().position(|a| a.id == "drop") {
            overflow_actions.insert(drop_pos, give_action);
        } else {
            overflow_actions.push(give_action);
        }
    }

    // Surface the generic `drop <item>` overflow action when the player holds
    // something droppable. Structure mirrors authored panel actions so moving
    // it to the main bar later is a placement-only change.
    if !drop_panel_options.is_empty() && !overflow_actions.iter().any(|a| a.id == "drop") {
        overflow_actions.push(OverflowAction {
            id: "drop".to_string(),
            label: content.ui_text.drop_label.clone(),
            group: "items".to_string(),
            usage: "drop <item>".to_string(),
            panel: "drop".to_string(),
            panel_config: Some(PanelConfigData {
                title: content.ui_text.drop_label.clone(),
                prompt: String::new(),
                data_source: PanelDataSource::InventoryItems,
                on_select: PanelSelectAction::ExecuteCommand,
            }),
        });
    }
    if !equipment_panel_options.is_empty() {
        overflow_actions.push(OverflowAction {
            id: "equipment".to_string(),
            label: "Equipment".to_string(),
            group: String::new(),
            usage: "equip <item> / unequip <item>".to_string(),
            panel: "equipment".to_string(),
            panel_config: Some(PanelConfigData {
                title: "Equipment".to_string(),
                prompt: "Choose an item to equip or stow.".to_string(),
                data_source: PanelDataSource::InventoryItems,
                on_select: PanelSelectAction::ExecuteCommand,
            }),
        });
    }

    Ok(overflow_actions)
}