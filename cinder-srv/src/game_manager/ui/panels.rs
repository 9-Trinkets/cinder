use cinder_core::content::types::{CommandEffect, ContentPack, PanelConfig, PanelDataSource, PanelSelectAction};
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::{ActorStance, WorldState};
use cinder_core::engine::turn_policies::action_is_available;
use std::collections::BTreeMap;

use super::{
    droppable_inventory_items, ActionBarAction, ActiveMenuData, LookOptionData, MenuOptionData,
    OverflowAction, PanelConfigData, PanelOptionData,
};

/// Builds the action bar, and also computes the option rows for the generic
/// `take <item>` picker that is only surfaced when a loose item lies in the
/// current room. Because the take button is appended to the bar itself, both
/// are produced together.
pub(super) fn build_action_bar_and_take(
    content: &ContentPack,
    state: &WorldState,
) -> (Vec<ActionBarAction>, Vec<PanelOptionData>) {
    let mut action_bar_actions: Vec<ActionBarAction> = if !content.actions.is_empty() {
        content
            .actions
            .iter()
            .filter(|a| a.ui.bar && action_is_available(content, state, a, &state.current_room_id))
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

    // The generic `take <item>` command surfaces as a single action-bar button
    // whenever a loose item lies in the current room. It reuses the same panel
    // model as authored content actions: the button opens a picker listing each
    // item (auto-selecting when only one is present), dispatching `take <id>`.
    let take_panel_options: Vec<PanelOptionData> = {
        let loose = takeable_loose_items(content, state);
        if loose.is_empty() {
            vec![]
        } else {
            action_bar_actions.push(ActionBarAction {
                id: "take".to_string(),
                label: content.ui_text.take_label.clone(),
                panel: Some("take".to_string()),
                panel_config: Some(PanelConfigData {
                    title: content.ui_text.room_items_sidebar_label.clone(),
                    prompt: String::new(),
                    data_source: PanelDataSource::LooseRoomItems,
                    on_select: PanelSelectAction::ExecuteCommand,
                }),
            });
            loose
                .into_iter()
                .map(|(item_id, _count)| loose_item_option(content, &item_id))
                .collect()
        }
    };

    (action_bar_actions, take_panel_options)
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
    droppable_inventory_items(state)
        .into_iter()
        .map(|item_id| PanelOptionData {
            id: item_id.clone(),
            title: content
                .item(&item_id)
                .map(|item| item.label.clone())
                .unwrap_or_else(|| item_id.clone()),
            subtitle: None,
            command: Some(format!("drop {item_id}")),
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
        .map(|info: cinder_core::engine::runtime::ActiveMenuInfo| ActiveMenuData {
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
        }))
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

pub(super) fn build_overflow_actions(
    runtime: &CinderRuntime,
    content: &ContentPack,
    state: &WorldState,
    bar_ids: &[&str],
    drop_panel_options: &[PanelOptionData],
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
            let label = a
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
                .join(" ");
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
    if !drop_panel_options.is_empty() {
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

    Ok(overflow_actions)
}

pub(super) fn build_panel_options(
    runtime: &CinderRuntime,
    content: &ContentPack,
    state: &WorldState,
    take_panel_options: Vec<PanelOptionData>,
    drop_panel_options: Vec<PanelOptionData>,
) -> Result<BTreeMap<String, Vec<PanelOptionData>>, String> {
    let mut panel_options: BTreeMap<String, Vec<PanelOptionData>> = BTreeMap::new();
    for action in &content.actions {
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
                    })
                    .collect(),
                PanelDataSource::CraftableItems => craftable_item_panel_options(
                    content, state, action, phrase,
                ),
                PanelDataSource::LooseRoomItems => runtime
                    .panel_options(&PanelDataSource::LooseRoomItems)
                    .map_err(|error| error.to_string())?
                    .into_iter()
                    .map(|opt| PanelOptionData {
                        id: opt.id.clone(),
                        title: opt.title.clone(),
                        subtitle: None,
                        command: Some(opt.command.clone()),
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
                    })
                    .collect(),
            };
            panel_options.insert(panel_name.clone(), options);
        }
    }
    panel_options.insert("take".to_string(), take_panel_options);
    panel_options.insert("drop".to_string(), drop_panel_options);
    Ok(panel_options)
}

fn craftable_item_panel_options(
    content: &ContentPack,
    state: &WorldState,
    action: &cinder_core::content::types::ActionDefinition,
    phrase: &str,
) -> Vec<PanelOptionData> {
    let gates = action.item_creation.as_ref().map(|ic| &ic.craftable_item_gates);
    action
        .item_creation
        .as_ref()
        .map(|ic| &ic.craftable_items)
        .filter(|craftables| !craftables.is_empty())
        .map(|craftables| {
            craftables
                .iter()
                .filter(|item_id| {
                    if content.item(item_id).is_some_and(|item| item.trace_mark)
                        && state.has_item_in_storage(
                            item_id,
                            cinder_core::content::types::ItemStorageTarget::CurrentRoom,
                            &state.current_room_id,
                        )
                    {
                        return false;
                    }
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
                    let title = content
                        .item(item_id)
                        .map(|item| item.label.clone())
                        .unwrap_or_else(|| item_id.clone());
                    PanelOptionData {
                        id: item_id.clone(),
                        title,
                        subtitle: None,
                        command: Some(format!("{} {}", phrase, item_id)),
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

fn loose_item_option(content: &ContentPack, item_id: &str) -> PanelOptionData {
    PanelOptionData {
        id: item_id.to_string(),
        title: content
            .item(item_id)
            .map(|item| item.label.clone())
            .unwrap_or_else(|| item_id.to_string()),
        subtitle: None,
        command: Some(format!("take {item_id}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cinder_core::content::types::{ItemDefinition, ItemStorageTarget};
    use cinder_core::engine::test_fixtures::minimal_test_pack;

    #[test]
    fn takeable_items_exclude_trace_marks() {
        let mut content = minimal_test_pack();
        content.items.extend([
            ItemDefinition {
                id: "sigil".to_string(),
                label: "sigil".to_string(),
                description: "A fixed mark.".to_string(),
                trace_mark: true,
                ..ItemDefinition::default()
            },
            ItemDefinition {
                id: "scroll".to_string(),
                label: "scroll".to_string(),
                description: "A portable scroll.".to_string(),
                ..ItemDefinition::default()
            },
        ]);
        let mut state = WorldState::new(&content);
        let room_id = state.current_room_id.clone();
        state.add_item_to_storage("sigil", ItemStorageTarget::CurrentRoom, &room_id);

        assert!(takeable_loose_items(&content, &state).is_empty());

        state.add_item_to_storage("scroll", ItemStorageTarget::CurrentRoom, &room_id);
        assert_eq!(
            takeable_loose_items(&content, &state),
            vec![("scroll".to_string(), 1)]
        );
    }
}
