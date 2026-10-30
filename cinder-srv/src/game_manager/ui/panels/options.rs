use super::super::{PanelOptionData, PanelOptionGroups, UiContext};
use super::{msg, msg_fmt, title_case};
use cinder_core::content::types::{
    ActionDefinition, CommandEffect, ContentPack, ItemStorageTarget, PanelDataSource,
};
use cinder_core::engine::runtime::{CinderRuntime, PanelOption};
use cinder_core::engine::state::{ActorStance, WorldState};
use std::collections::BTreeMap;

/// Actions whose panel options the UI builds generically instead of from the
/// authored panel config.
fn is_generic_item_action(id: &str) -> bool {
    matches!(id, "take" | "give" | "use")
}

pub(crate) fn build_panel_options(
    ctx: &UiContext,
    panels: PanelOptionGroups,
) -> Result<BTreeMap<String, Vec<PanelOptionData>>, String> {
    let (runtime, content, state) = (ctx.runtime, ctx.content, ctx.state);
    let PanelOptionGroups {
        take: take_panel_options,
        use_item: use_panel_options,
        give: give_panel_options,
        drop: drop_panel_options,
        equipment: equipment_panel_options,
    } = panels;
    let mut panel_options: BTreeMap<String, Vec<PanelOptionData>> = BTreeMap::new();
    for action in &content.actions {
        if is_generic_item_action(&action.id) {
            continue;
        }
        if let (Some(panel_name), Some(panel_config)) = (&action.ui.panel, &action.ui.panel_config)
        {
            let phrase = action
                .phrases
                .first()
                .map(|s| s.as_str())
                .unwrap_or(&action.command);
            let options = match &panel_config.data_source {
                PanelDataSource::ActorsInRoom => {
                    map_actor_target_options(runtime, state, action, phrase)?
                }
                PanelDataSource::Exits => map_exit_panel_options(runtime)?,
                PanelDataSource::CraftableItems => {
                    craftable_item_panel_options(content, state, action, phrase)
                }
                PanelDataSource::FollowActors | PanelDataSource::TeleportAnchors => {
                    map_actor_or_teleport_options(runtime, &panel_config.data_source)?
                }
                data_source => runtime
                    .panel_options(data_source)
                    .map_err(|error| error.to_string())?
                    .into_iter()
                    .map(PanelOptionData::from)
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

/// Whether an in-room actor is a legal attack target: never allies or followers.
fn is_hostile_target(state: &WorldState, option: &PanelOption) -> bool {
    let actor_id = option.id.strip_prefix("actor:").unwrap_or(&option.id);
    let relationship = state.relationship(actor_id);
    relationship.stance != ActorStance::Allied && !relationship.follows_player
}

/// Options for a panel backed by actors in the room. Attack panels additionally
/// hide allies and followers so the player can't strike their own party.
fn map_actor_target_options(
    runtime: &CinderRuntime,
    state: &WorldState,
    action: &ActionDefinition,
    phrase: &str,
) -> Result<Vec<PanelOptionData>, String> {
    let is_attack = action.has_effect(CommandEffect::AttackTarget);
    Ok(runtime
        .panel_options(&PanelDataSource::ActorsInRoom)
        .map_err(|error| error.to_string())?
        .into_iter()
        .filter(|option| !is_attack || is_hostile_target(state, option))
        .map(|option| {
            let actor_id = option
                .id
                .strip_prefix("actor:")
                .unwrap_or(&option.id)
                .to_string();
            PanelOptionData {
                id: actor_id.clone(),
                command: Some(format!("{phrase} {actor_id}")),
                ..option.into()
            }
        })
        .collect())
}

/// Options for a panel backed by room exits. When the authored exit label differs
/// from the destination room title, the destination room title becomes the subtitle.
pub(crate) fn map_exit_panel_options(
    runtime: &CinderRuntime,
) -> Result<Vec<PanelOptionData>, String> {
    Ok(runtime
        .panel_options(&PanelDataSource::Exits)
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|option| {
            let subtitle = if !option.menu_text.is_empty()
                && !option.menu_text.eq_ignore_ascii_case(&option.title)
            {
                Some(option.menu_text.clone())
            } else {
                None
            };
            PanelOptionData {
                subtitle,
                ..option.into()
            }
        })
        .collect())
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
            let mut options: Vec<PanelOptionData> = craftables
                .iter()
                .filter(|item_id| {
                    gates.and_then(|g| g.get(*item_id)).is_none_or(|when| {
                        cinder_core::engine::conditions::all_hold(
                            when,
                            &cinder_core::engine::conditions::ConditionContext::new(
                                content,
                                state,
                                &state.current_room_id,
                            ),
                        )
                    })
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
                    let trace_lvl = state.actor_skill_level(player_id, "trace");
                    let base_mp = item.map(|i| i.mp_cost).unwrap_or(0);
                    let mp_cost = if item.is_some_and(|i| i.trace_mark) {
                        cinder_core::engine::skills::effective_trace_mp_cost_for_pack(
                            content, base_mp, trace_lvl,
                        )
                    } else {
                        base_mp
                    };
                    let at_instance_limit = item.is_some_and(|i| {
                        i.max_active_instances.is_some_and(|max| {
                            !i.spawn_template_id.is_empty()
                                && state.active_spawned_actor_count(content, &i.spawn_template_id)
                                    >= max
                        })
                    });
                    let insufficient_mp = mp_cost > player_mp;
                    let disabled = already_traced || at_instance_limit || insufficient_mp;

                    let subtitle = if already_traced {
                        Some(content.ui_text.trace_mark_present_label.clone())
                    } else if at_instance_limit {
                        Some(msg(content, "ui.craft_max_summons", "Max summons active"))
                    } else if insufficient_mp {
                        Some(msg_fmt(
                            content,
                            "ui.craft_mp_of",
                            &format!("{mp_cost} MP (have {player_mp})"),
                            &[
                                ("mp", &mp_cost.to_string()),
                                ("mp_max", &player_mp.to_string()),
                            ],
                        ))
                    } else if mp_cost > 0 {
                        Some(msg_fmt(
                            content,
                            "ui.craft_mp",
                            &format!("{mp_cost} MP"),
                            &[("mp", &mp_cost.to_string())],
                        ))
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
                        group: None,
                        panel: None,
                    }
                })
                .collect();

            if action.id == "trace" || phrase == "trace" {
                let trace_in_room: Vec<String> = state
                    .loose_room_items(&state.current_room_id)
                    .into_iter()
                    .filter(|(item_id, count)| {
                        *count > 0 && content.item(item_id).is_some_and(|i| i.trace_mark)
                    })
                    .map(|(item_id, _)| item_id)
                    .collect();

                if !trace_in_room.is_empty() {
                    let title = if trace_in_room.len() == 1 {
                        msg_fmt(
                            content,
                            "ui.erase_sigil_one",
                            &format!(
                                "Erase {}",
                                title_case(content.item_label(&trace_in_room[0]))
                            ),
                            &[("item", &title_case(content.item_label(&trace_in_room[0])))],
                        )
                    } else {
                        msg(content, "ui.erase_sigils_many", "Erase Sigils")
                    };
                    options.push(PanelOptionData {
                        id: "erase".to_string(),
                        title,
                        subtitle: Some(msg(
                            content,
                            "ui.erase_sigil_help",
                            "Wipe chalk marks from this room",
                        )),
                        command: Some("erase".to_string()),
                        disabled: false,
                        selected: false,
                        group: None,
                        panel: None,
                    });
                } else {
                    options.push(PanelOptionData {
                        id: "erase".to_string(),
                        title: msg(content, "ui.erase_sigil_empty_title", "Erase Sigil"),
                        subtitle: Some(msg(
                            content,
                            "ui.erase_sigil_none",
                            "No sigils in this room",
                        )),
                        command: None,
                        disabled: true,
                        selected: false,
                        group: None,
                        panel: None,
                    });
                }
            }

            options
        })
        .unwrap_or_default()
}

fn map_actor_or_teleport_options(
    runtime: &CinderRuntime,
    data_source: &PanelDataSource,
) -> Result<Vec<PanelOptionData>, String> {
    let is_teleport = matches!(data_source, PanelDataSource::TeleportAnchors);
    let options = runtime
        .panel_options(data_source)
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|option| {
            let menu_text = option.menu_text.clone();
            let group = (is_teleport && !menu_text.is_empty()).then(|| menu_text.clone());
            let subtitle = (!is_teleport && !menu_text.is_empty()).then_some(menu_text);
            PanelOptionData {
                subtitle,
                group,
                ..option.into()
            }
        })
        .collect();
    Ok(options)
}
