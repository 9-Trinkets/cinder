use super::title_case;
use super::super::PanelOptionData;
use cinder_core::content::types::{
    ActionDefinition, CommandEffect, ContentPack, ItemStorageTarget, PanelDataSource,
};
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::{ActorStance, WorldState};
use std::collections::BTreeMap;

pub(crate) fn build_panel_options(
    runtime: &CinderRuntime,
    content: &ContentPack,
    state: &WorldState,
    take_panel_options: Vec<PanelOptionData>,
    use_panel_options: Vec<PanelOptionData>,
    give_panel_options: Vec<PanelOptionData>,
    drop_panel_options: Vec<PanelOptionData>,
    equipment_panel_options: Vec<PanelOptionData>,
) -> Result<BTreeMap<String, Vec<PanelOptionData>>, String> {
    let mut panel_options: BTreeMap<String, Vec<PanelOptionData>> = BTreeMap::new();
    for action in &content.actions {
        if action.id == "take" || action.id == "give" || action.id == "use" {
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
    panel_options.insert("use".to_string(), use_panel_options);
    panel_options.insert("give".to_string(), give_panel_options);
    panel_options.insert("drop".to_string(), drop_panel_options);
    panel_options.insert("equipment".to_string(), equipment_panel_options);
    Ok(panel_options)
}

pub(crate) fn craftable_item_panel_options(
    content: &ContentPack,
    state: &WorldState,
    action: &ActionDefinition,
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
                    let item = content.item(item_id);
                    let already_traced = item.is_some_and(|i| i.trace_mark)
                        && state.has_item_in_storage(
                            item_id,
                            ItemStorageTarget::CurrentRoom,
                            &state.current_room_id,
                        );
                    let player_id = &content.settings.combat.player_actor_id;
                    let player_mp = state.actor_stat_u32(player_id, "mp");
                    let mp_cost = item.map(|i| i.mp_cost).unwrap_or(0);
                    let at_instance_limit = item.is_some_and(|i| {
                        i.max_active_instances.is_some_and(|max| {
                            !i.spawn_template_id.is_empty()
                                && state.active_spawned_actor_count(content, &i.spawn_template_id) >= max
                        })
                    });
                    let insufficient_mp = mp_cost > player_mp;
                    let disabled = already_traced || at_instance_limit || insufficient_mp;

                    let subtitle = if already_traced {
                        Some(content.ui_text.trace_mark_present_label.clone())
                    } else if at_instance_limit {
                        Some("Max summons active".to_string())
                    } else if insufficient_mp {
                        Some(format!("{mp_cost} MP (have {player_mp})"))
                    } else if mp_cost > 0 {
                        Some(format!("{mp_cost} MP"))
                    } else {
                        None
                    };

                    let title = title_case(content.item_label(item_id));
                    PanelOptionData {
                        id: item_id.clone(),
                        title,
                        subtitle,
                        command: (!disabled).then(|| format!("{} {}", phrase, item_id)),
                        disabled,
                        selected: false,
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}