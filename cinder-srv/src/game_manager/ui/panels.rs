mod equipment;

use cinder_core::content::types::{
    ActionDefinition, CommandEffect, ContentPack, PanelConfig, PanelDataSource, PanelSelectAction,
};
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::{ActorStance, WorldState};
use cinder_core::engine::turn_policies::action_is_available;
use std::collections::BTreeMap;

use super::{
    ActionBarAction, ActiveMenuData, LookOptionData, MenuOptionData, OverflowAction,
    PanelConfigData, PanelOptionData, PartyMember, droppable_inventory_items,
};
pub(super) use equipment::build_equipment_panel_options;

/// Builds the action bar actions along with options for the dynamic
/// `take` and `give` panels.
///
/// - `take`: Surfaces when loose room items or companion items exist.
/// - `give`: Surfaces when companions are present and the player has inventory items.
pub(super) fn build_action_bar_items(
    content: &ContentPack,
    state: &WorldState,
    party: &[PartyMember],
) -> (Vec<ActionBarAction>, Vec<PanelOptionData>, Vec<PanelOptionData>) {
    let mut action_bar_actions: Vec<ActionBarAction> = if !content.actions.is_empty() {
        content
            .actions
            .iter()
            .filter(|a| {
                // 'take' and 'give' are dynamically placed when applicable
                if a.id == "take" || a.id == "give" {
                    return false;
                }
                a.ui.bar && action_is_available(content, state, a, &state.current_room_id)
            })
            .map(|a| ActionBarAction {
                id: a.id.clone(),
                label: a.label.clone(),
                panel: a.ui.panel.clone(),
                panel_config: a.ui.panel_config.as_ref().map(panel_config_data),
            })
            .collect()
    } else {
        vec![]
    };

    // 1. Take items: from ground and/or companion packs/equipment
    let take_panel_options = if !content.player_can_take_items() {
        vec![]
    } else {
        let loose = takeable_loose_items(content, state);
        let has_companion_items = party
            .iter()
            .any(|m| !m.inventory.is_empty() || !m.equipped_items.is_empty());

        let mut options = Vec::new();
        // Ground items
        for (item_id, _) in &loose {
            let title = title_case(content.item_label(item_id));
            let subtitle = if has_companion_items {
                Some("On the ground".to_string())
            } else {
                None
            };
            options.push(PanelOptionData {
                id: format!("ground:{item_id}"),
                title,
                subtitle,
                command: Some(format!("take {item_id}")),
                disabled: false,
                selected: false,
            });
        }
        // Companion items
        for member in party {
            for item in &member.inventory {
                let item_ref = item.id.as_deref().unwrap_or(&item.label);
                options.push(PanelOptionData {
                    id: format!("member_inv:{}:{item_ref}", member.id),
                    title: title_case(&item.label),
                    subtitle: Some(format!("From {}", member.label)),
                    command: Some(format!("take {item_ref} from {}", member.id)),
                    disabled: false,
                    selected: false,
                });
            }
            for item in &member.equipped_items {
                let item_ref = item.id.as_deref().unwrap_or(&item.label);
                options.push(PanelOptionData {
                    id: format!("member_equip:{}:{item_ref}", member.id),
                    title: title_case(&item.label),
                    subtitle: Some(format!("From {} ({})", member.label, item.slot)),
                    command: Some(format!("take {item_ref} from {}", member.id)),
                    disabled: false,
                    selected: false,
                });
            }
        }

        if !options.is_empty() {
            action_bar_actions.push(ActionBarAction {
                id: "take".to_string(),
                label: content.ui_text.take_label.clone(),
                panel: Some("take".to_string()),
                panel_config: Some(PanelConfigData {
                    title: if loose.is_empty() {
                        "Take from Companion".to_string()
                    } else {
                        content.ui_text.room_items_sidebar_label.clone()
                    },
                    prompt: if has_companion_items && !loose.is_empty() {
                        "Choose an item to take from the room or companions".to_string()
                    } else {
                        String::new()
                    },
                    data_source: PanelDataSource::LooseRoomItems,
                    on_select: PanelSelectAction::ExecuteCommand,
                }),
            });
        }
        options
    };

    // 2. Give items: to party members
    // Step 1: list droppable items. If 1 companion exists, auto-gives to them.
    // If multiple companions exist, UI prompts for recipient in step 2.
    let give_panel_options = if party.is_empty() {
        vec![]
    } else {
        let droppable = droppable_inventory_items(state);
        if droppable.is_empty() {
            vec![]
        } else {
            let mut options = Vec::new();
            for item_id in &droppable {
                let command = if party.len() == 1 {
                    Some(format!("give {item_id} to {}", party[0].id))
                } else {
                    None
                };
                options.push(PanelOptionData {
                    id: item_id.clone(),
                    title: title_case(content.item_label(item_id)),
                    subtitle: None,
                    command,
                    disabled: false,
                    selected: false,
                });
            }

            if !options.is_empty() {
                let (give_title, give_prompt) = if party.len() == 1 {
                    (
                        format!("Give to {}", party[0].label),
                        "Choose an item to give".to_string(),
                    )
                } else {
                    (
                        "Give to Companion".to_string(),
                        "Choose an item to give".to_string(),
                    )
                };
                action_bar_actions.push(ActionBarAction {
                    id: "give".to_string(),
                    label: "Give".to_string(),
                    panel: Some("give".to_string()),
                    panel_config: Some(PanelConfigData {
                        title: give_title,
                        prompt: give_prompt,
                        data_source: PanelDataSource::InventoryItems,
                        on_select: PanelSelectAction::ExecuteCommand,
                    }),
                });
            }
            options
        }
    };

    (action_bar_actions, take_panel_options, give_panel_options)
}

#[cfg(test)]
pub(super) fn build_action_bar_and_take(
    content: &ContentPack,
    state: &WorldState,
) -> (Vec<ActionBarAction>, Vec<PanelOptionData>) {
    let (actions, take, _) = build_action_bar_items(content, state, &[]);
    (actions, take)
}

fn takeable_loose_items(content: &ContentPack, state: &WorldState) -> Vec<(String, u32)> {
    state
        .loose_room_items(&state.current_room_id)
        .into_iter()
        .filter(|(item_id, _)| content.item(item_id).is_none_or(|item| item.is_takeable()))
        .collect()
}

/// Option rows for the generic `drop <item>` overflow action.
pub(super) fn build_drop_panel_options(
    content: &ContentPack,
    state: &WorldState,
) -> Vec<PanelOptionData> {
    if !content.player_can_drop_items() {
        return vec![];
    }
    droppable_inventory_items(state)
        .into_iter()
        .map(|item_id| PanelOptionData {
            id: item_id.clone(),
            title: title_case(content.item_label(&item_id)),
            subtitle: None,
            command: Some(format!("drop {item_id}")),
            disabled: false,
            selected: false,
        })
        .collect()
}

pub(super) fn build_look_options(runtime: &CinderRuntime) -> Result<Vec<LookOptionData>, String> {
    Ok(runtime
        .room_interactable_options()
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|option| LookOptionData {
            id: option.id,
            title: option.title,
            command: option.command,
        })
        .collect())
}

pub(super) fn build_talk_options(runtime: &CinderRuntime) -> Result<Vec<MenuOptionData>, String> {
    Ok(runtime
        .panel_options(&PanelDataSource::ActorsInRoom)
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|option| MenuOptionData {
            id: option.id,
            title: option.title.clone(),
            menu_text: option.title,
        })
        .collect())
}

pub(super) fn build_active_menu(runtime: &CinderRuntime) -> Result<Option<ActiveMenuData>, String> {
    Ok(runtime
        .current_active_menu_info()
        .map_err(|error| error.to_string())?
        .map(
            |info: cinder_core::engine::runtime::ActiveMenuInfo| ActiveMenuData {
                prompt: info.prompt,
                max_selections: info.max_selections,
                min_selections: info.min_selections,
                selected_ids: info.selected_ids,
                options: info
                    .options
                    .into_iter()
                    .map(|option| MenuOptionData {
                        id: option.id,
                        title: option.title,
                        menu_text: option.menu_text,
                    })
                    .collect(),
            },
        ))
}

/// Names the transcript can highlight as interactable: actors, features, and
/// items present here (everything except the room itself).
pub(super) fn build_interactable_labels(look_options: &[LookOptionData]) -> Vec<String> {
    let mut labels: Vec<String> = look_options
        .iter()
        .filter(|option| option.id != "__room__")
        .map(|option| option.title.clone())
        .collect();
    labels.sort();
    labels.dedup();
    labels
}

/// Display title for an overflow action button: the authored `label` (e.g.
/// "Use Moss Poultice") when present, otherwise a title-cased form of the
/// action id as a fallback for legacy actions without a label.
fn overflow_action_title(action: &ActionDefinition) -> String {
    if !action.label.is_empty() {
        return action.label.clone();
    }
    action
        .id
        .split('_')
        .map(|word| {
            let mut chars = word.chars();
            chars
                .next()
                .map(|first: char| first.to_uppercase().to_string() + chars.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn build_overflow_actions(
    runtime: &CinderRuntime,
    content: &ContentPack,
    state: &WorldState,
    bar_ids: &[&str],
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
            action_is_available(content, state, a, &current_room_id)
        })
        .map(|a| {
            let label = overflow_action_title(a);
            let usage = a
                .player_command
                .as_ref()
                .map(|player_command| player_command.usage.clone())
                .unwrap_or_default();
            OverflowAction {
                id: a.id.clone(),
                label,
                group: a.ui.group.clone(),
                usage,
                panel: a.ui.panel.clone().unwrap_or_default(),
                panel_config: a.ui.panel_config.as_ref().map(panel_config_data),
            }
        })
        .collect();

    if let Ok(active_stages) = runtime.active_stage_ids() {
        super::append_stage_menu_overflow_actions(&mut overflow_actions, content, &active_stages);
    }

    // Surface the generic `drop <item>` overflow action when the player holds
    // something droppable. Structure mirrors authored panel actions so moving
    // it to the main bar later is a placement-only change.
    if !drop_panel_options.is_empty() && !overflow_actions.iter().any(|a| a.id == "drop") {
        overflow_actions.push(OverflowAction {
            id: "drop".to_string(),
            label: content.ui_text.drop_label.clone(),
            group: String::new(),
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

pub(super) fn build_panel_options(
    runtime: &CinderRuntime,
    content: &ContentPack,
    state: &WorldState,
    take_panel_options: Vec<PanelOptionData>,
    give_panel_options: Vec<PanelOptionData>,
    drop_panel_options: Vec<PanelOptionData>,
    equipment_panel_options: Vec<PanelOptionData>,
) -> Result<BTreeMap<String, Vec<PanelOptionData>>, String> {
    let mut panel_options: BTreeMap<String, Vec<PanelOptionData>> = BTreeMap::new();
    for action in &content.actions {
        if action.id == "take" || action.id == "give" {
            continue;
        }
        if let (Some(panel_name), Some(panel_config)) = (&action.ui.panel, &action.ui.panel_config)
        {
            let phrase = action
                .phrases
                .first()
                .map(|s| s.as_str())
                .unwrap_or(&action.command);
            let options = match panel_config.data_source {
                PanelDataSource::ActorsInRoom => {
                    let is_attack = action.has_effect(CommandEffect::AttackTarget);
                    runtime
                        .panel_options(&PanelDataSource::ActorsInRoom)
                        .map_err(|error| error.to_string())?
                        .into_iter()
                        .filter(|opt| {
                            if !is_attack {
                                return true;
                            }
                            let actor_id = opt.id.strip_prefix("actor:").unwrap_or(&opt.id);
                            // Never offer allies or followers as attack targets so
                            // the player can't accidentally strike their own party.
                            let relationship = state.relationship(actor_id);
                            relationship.stance != ActorStance::Allied
                                && !relationship.follows_player
                        })
                        .map(|opt| {
                            let actor_id = opt.id.strip_prefix("actor:").unwrap_or(&opt.id);
                            PanelOptionData {
                                id: actor_id.to_string(),
                                title: opt.title.clone(),
                                subtitle: None,
                                command: Some(format!("{} {}", phrase, actor_id)),
                                disabled: false,
                                selected: false,
                            }
                        })
                        .collect()
                }
                PanelDataSource::Exits => runtime
                    .panel_options(&PanelDataSource::Exits)
                    .map_err(|error| error.to_string())?
                    .into_iter()
                    .map(|opt| PanelOptionData {
                        id: opt.id.clone(),
                        title: opt.title.clone(),
                        subtitle: if opt.menu_text.is_empty() {
                            None
                        } else {
                            Some(opt.menu_text)
                        },
                        command: Some(opt.command.clone()),
                        disabled: false,
                        selected: false,
                    })
                    .collect(),
                PanelDataSource::Features => runtime
                    .panel_options(&PanelDataSource::Features)
                    .map_err(|error| error.to_string())?
                    .into_iter()
                    .map(|opt| PanelOptionData {
                        id: opt.id.clone(),
                        title: opt.title.clone(),
                        subtitle: None,
                        command: Some(opt.command.clone()),
                        disabled: false,
                        selected: false,
                    })
                    .collect(),
                PanelDataSource::CraftableItems => {
                    craftable_item_panel_options(content, state, action, phrase)
                }
                PanelDataSource::LooseRoomItems => runtime
                    .panel_options(&PanelDataSource::LooseRoomItems)
                    .map_err(|error| error.to_string())?
                    .into_iter()
                    .map(|opt| PanelOptionData {
                        id: opt.id.clone(),
                        title: opt.title.clone(),
                        subtitle: None,
                        command: Some(opt.command.clone()),
                        disabled: false,
                        selected: false,
                    })
                    .collect(),
                PanelDataSource::InventoryItems => runtime
                    .panel_options(&PanelDataSource::InventoryItems)
                    .map_err(|error| error.to_string())?
                    .into_iter()
                    .map(|opt| PanelOptionData {
                        id: opt.id.clone(),
                        title: opt.title.clone(),
                        subtitle: None,
                        command: Some(opt.command.clone()),
                        disabled: false,
                        selected: false,
                    })
                    .collect(),
                PanelDataSource::FollowActors => runtime
                    .panel_options(&PanelDataSource::FollowActors)
                    .map_err(|error| error.to_string())?
                    .into_iter()
                    .map(|opt| PanelOptionData {
                        id: opt.id.clone(),
                        title: opt.title.clone(),
                        subtitle: if opt.menu_text.is_empty() {
                            None
                        } else {
                            Some(opt.menu_text)
                        },
                        command: Some(opt.command.clone()),
                        disabled: false,
                        selected: false,
                    })
                    .collect(),
            };
            panel_options.insert(panel_name.clone(), options);
        }
    }
    panel_options.insert("take".to_string(), take_panel_options);
    panel_options.insert("give".to_string(), give_panel_options);
    panel_options.insert("drop".to_string(), drop_panel_options);
    panel_options.insert("equipment".to_string(), equipment_panel_options);
    Ok(panel_options)
}

fn craftable_item_panel_options(
    content: &ContentPack,
    state: &WorldState,
    action: &cinder_core::content::types::ActionDefinition,
    phrase: &str,
) -> Vec<PanelOptionData> {
    let gates = action
        .item_creation
        .as_ref()
        .map(|ic| &ic.craftable_item_gates);
    action
        .item_creation
        .as_ref()
        .map(|ic| &ic.craftable_items)
        .filter(|craftables| !craftables.is_empty())
        .map(|craftables| {
            craftables
                .iter()
                .filter(|item_id| {
                    let unlocked = || -> bool {
                        let Some(gate) = gates.and_then(|g| g.get(*item_id)) else {
                            return true;
                        };
                        if gate.is_empty() {
                            return true;
                        }
                        !matches!(
                            state
                                .story_vars
                                .get(gate)
                                .map(str::trim)
                                .unwrap_or("")
                                .to_ascii_lowercase()
                                .as_str(),
                            "" | "false" | "0"
                        )
                    };
                    unlocked()
                })
                .map(|item_id| {
                    let already_traced = content.item(item_id).is_some_and(|item| item.trace_mark)
                        && state.has_item_in_storage(
                            item_id,
                            cinder_core::content::types::ItemStorageTarget::CurrentRoom,
                            &state.current_room_id,
                        );
                    let title = title_case(content.item_label(item_id));
                    PanelOptionData {
                        id: item_id.clone(),
                        title,
                        subtitle: already_traced
                            .then(|| content.ui_text.trace_mark_present_label.clone()),
                        command: (!already_traced).then(|| format!("{} {}", phrase, item_id)),
                        disabled: already_traced,
                        selected: false,
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn panel_config_data(pc: &PanelConfig) -> PanelConfigData {
    PanelConfigData {
        title: pc.title.clone(),
        prompt: pc.prompt.clone(),
        data_source: pc.data_source.clone(),
        on_select: pc.on_select.clone(),
    }
}


pub(crate) fn title_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut capitalize_next = true;
    for c in s.chars() {
        if c.is_alphanumeric() {
            if capitalize_next {
                for uc in c.to_uppercase() {
                    result.push(uc);
                }
                capitalize_next = false;
            } else {
                result.push(c);
            }
        } else {
            result.push(c);
            if c != '\'' {
                capitalize_next = true;
            }
        }
    }
    result
}

#[cfg(test)]
#[path = "panels/panel_tests.rs"]
mod tests;
