use cinder_core::content::loader;
use cinder_core::content::types::UiTextDefinition;
use cinder_core::engine::runtime::{
    ActiveMenuInfo, CinderRuntime, LookOptionItem, MenuChoiceOption,
};
use serde::Serialize;

use super::response;
use super::response::SessionFeedbackData;

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
}

#[derive(Clone, Serialize)]
pub struct ActionBarAction {
    pub id: String,
    pub label: String,
}

#[derive(Clone, Serialize)]
pub struct OverflowAction {
    pub id: String,
    pub label: String,
    pub group: String,
    pub usage: String,
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
}

#[derive(Clone, Serialize)]
pub struct UiSnapshot {
    pub title: String,
    pub time_label: String,
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
    pub talk_options: Vec<MenuOptionData>,
    pub active_menu: Option<ActiveMenuData>,
    pub ui_text: UiTextDefinition,
    pub session_feedback: Option<SessionFeedbackData>,
    pub inventory: Vec<InventoryItem>,
}

pub(super) fn build_ui_snapshot(
    runtime: &CinderRuntime,
    pack_id: &str,
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
        .unwrap_or(current_room_id);
    let followed_actor_name = runtime
        .followed_actor_id()
        .map_err(|error| error.to_string())?
        .and_then(|id| runtime.actor_display_name(&id).ok().flatten());

    let (action_bar_actions, content_defined_bar) =
        if !content.ui_text.action_bar.actions.is_empty() {
            (
                content
                    .ui_text
                    .action_bar
                    .actions
                    .iter()
                    .map(|action| ActionBarAction {
                        id: action.id.clone(),
                        label: action.label.clone(),
                    })
                    .collect(),
                true,
            )
        } else {
            (
                vec![
                    ActionBarAction {
                        id: "look".into(),
                        label: "Look".into(),
                    },
                    ActionBarAction {
                        id: "move".into(),
                        label: "Move".into(),
                    },
                    ActionBarAction {
                        id: "follow".into(),
                        label: "Follow".into(),
                    },
                ],
                false,
            )
        };

    let look_options: Vec<LookOptionData> = runtime
        .current_room_look_options()
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|option: LookOptionItem| LookOptionData {
            id: option.id,
            title: option.label,
            command: option.command,
        })
        .collect();

    let talk_options: Vec<MenuOptionData> = runtime
        .current_room_talk_options()
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|option: LookOptionItem| MenuOptionData {
            id: option.id,
            title: option.label.clone(),
            menu_text: option.label,
        })
        .collect();

    let active_menu: Option<ActiveMenuData> = runtime
        .current_active_menu_info()
        .map_err(|error| error.to_string())?
        .map(|info: ActiveMenuInfo| ActiveMenuData {
            prompt: info.prompt,
            options: info
                .options
                .into_iter()
                .map(|option| MenuOptionData {
                    id: option.id,
                    title: option.title,
                    menu_text: option.menu_text,
                })
                .collect(),
        });

    let mut action_bar_actions = action_bar_actions;
    if !content_defined_bar
        && !talk_options.is_empty()
        && !action_bar_actions
            .iter()
            .any(|action| action.id == "speak" || action.id == "talk")
    {
        action_bar_actions.push(ActionBarAction {
            id: "talk".into(),
            label: "Talk".into(),
        });
    }

    let bar_ids: Vec<&str> = action_bar_actions
        .iter()
        .map(|action| action.id.as_str())
        .collect();
    let has_talk = bar_ids.contains(&"speak") || bar_ids.contains(&"talk");
    let modal_covered: Vec<&str> = vec!["inspect_feature", "inspect_actor"];
    let current_room_id = runtime.current_room_id().unwrap_or_default();
    let mut overflow_actions: Vec<OverflowAction> = content
        .commands
        .actions
        .iter()
        .filter(|command| {
            if !command.player_enabled || bar_ids.contains(&command.id.as_str()) {
                return false;
            }
            if modal_covered.contains(&command.id.as_str()) {
                return false;
            }
            if (command.id == "speak" || command.id == "talk") && has_talk {
                return false;
            }
            if !command.allowed_rooms.is_empty()
                && !command.allowed_rooms.contains(&current_room_id)
            {
                return false;
            }
            if let Some(item_id) = &command.consumes_item
                && !runtime.player_has_item(item_id).unwrap_or(false)
            {
                return false;
            }
            if !command.requires_any.is_empty() || !command.consumes_any.is_empty() {
                let has_any = command
                    .requires_any
                    .iter()
                    .chain(command.consumes_any.iter())
                    .any(|id| runtime.player_has_item(id).unwrap_or(false));
                if !has_any {
                    return false;
                }
            }
            if !command.available_during.is_empty() {
                let active_stages: Vec<String> = runtime.active_stage_ids().unwrap_or_default();
                let matches_stage = command
                    .available_during
                    .iter()
                    .any(|stage_id| active_stages.contains(stage_id));
                if !matches_stage {
                    return false;
                }
            }
            true
        })
        .map(|command| {
            let label = command
                .id
                .split('_')
                .map(|word| {
                    let mut chars = word.chars();
                    chars
                        .next()
                        .map(|first| first.to_uppercase().to_string() + chars.as_str())
                        .unwrap_or_default()
                })
                .collect::<Vec<_>>()
                .join(" ");
            let usage = command
                .player_command
                .as_ref()
                .map(|player_command| player_command.usage.clone())
                .unwrap_or_default();
            OverflowAction {
                id: command.id.clone(),
                label,
                group: command.group.clone(),
                usage,
            }
        })
        .collect();

    if let Ok(active_stages) = runtime.active_stage_ids() {
        for stage_id in &active_stages {
            let Some(menu) = content
                .menus
                .iter()
                .find(|menu| &menu.stage_id == stage_id && !menu.options.is_empty())
            else {
                continue;
            };
            for option in &menu.options {
                overflow_actions.push(OverflowAction {
                    id: option.id.clone(),
                    label: option.title.clone(),
                    group: "support".to_string(),
                    usage: String::new(),
                });
            }
        }
    }

    Ok(UiSnapshot {
        title: content.opening.title.clone(),
        time_label,
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
                .room_switch_options()
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
        talk_options,
        active_menu,
        ui_text: content.ui_text.clone(),
        session_feedback: response::session_feedback_data(runtime),
        inventory: runtime
            .inventory_items()
            .unwrap_or_default()
            .into_iter()
            .map(|(id, count)| {
                let label = content
                    .item(&id)
                    .map(|item| item.label.clone())
                    .unwrap_or_else(|| id.clone());
                InventoryItem { label, count }
            })
            .collect(),
    })
}

fn menu_option_data(options: Vec<MenuChoiceOption>) -> Vec<MenuOptionData> {
    options
        .into_iter()
        .map(|option| MenuOptionData {
            id: option.command,
            title: option.title,
            menu_text: option.menu_text,
        })
        .collect()
}
