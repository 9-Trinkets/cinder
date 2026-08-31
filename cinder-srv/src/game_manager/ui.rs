use cinder_core::content::loader;
use cinder_core::content::types::{
    CommandEffect, PanelDataSource, PanelSelectAction, UiTextDefinition,
};
use cinder_core::engine::runtime::{
    ActClosure, ActiveMenuInfo, CinderRuntime, LookOptionItem, MenuChoiceOption,
};
use cinder_core::engine::state::{ActorStance, WorldState};
use cinder_core::engine::turn_policies::action_is_available;
use serde::Serialize;
use std::collections::BTreeMap;

use super::response;

/// The player's progress toward the next level. XP/level are per-actor now:
/// `actor_xp` holds progress toward the *next* level (reset past each
/// threshold on level-up), so `xp_max` is the threshold at the player's own
/// current level. A missing threshold means the actor is at max level
/// (xp_max == 0).
fn xp_progress(
    state: &WorldState,
    content: &cinder_core::content::types::ContentPack,
) -> (u32, u32, u32) {
    let player_id = &content.settings.combat.player_actor_id;
    let level = state.actor_level(player_id);
    let xp = state.actor_xp(player_id);
    let xp_max = content
        .xp_required_for_next_level(player_id, level)
        .unwrap_or(0);
    (level, xp, xp_max)
}

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
    pub on_select: PanelSelectAction,
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
        .unwrap_or(current_room_id);
    let followed_actor_name = runtime
        .followed_actor_id()
        .map_err(|error| error.to_string())?
        .and_then(|id| runtime.actor_display_name(&id).ok().flatten());
    let crafted_consumable_ids = content.crafted_current_room_item_ids();
    let highlighted_consumable =
        |consumable: &cinder_core::content::types::ConsumableDefinition| {
            crafted_consumable_ids.contains(&consumable.id) || consumable.initial_stock > 0
        };

    let state = runtime.export_state().map_err(|e| e.to_string())?;
    let action_bar_actions: Vec<ActionBarAction> = if !content.actions.is_empty() {
        content
            .actions
            .iter()
            .filter(|a| a.ui.bar && action_is_available(content, &state, a, &state.current_room_id))
            .map(|a| ActionBarAction {
                id: a.id.clone(),
                label: a.label.clone(),
                panel: a.ui.panel.clone(),
                panel_config: a.ui.panel_config.as_ref().map(|pc| PanelConfigData {
                    title: pc.title.clone(),
                    prompt: pc.prompt.clone(),
                    data_source: pc.data_source.clone(),
                    on_select: pc.on_select.clone(),
                }),
            })
            .collect()
    } else {
        vec![]
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

    // Names the transcript can highlight as interactable: actors, features,
    // and items present here (everything except the room itself).
    let interactable_labels: Vec<String> = {
        let mut labels: Vec<String> = look_options
            .iter()
            .filter(|option| option.id != "__room__")
            .map(|option| option.title.clone())
            .collect();
        labels.sort();
        labels.dedup();
        labels
    };

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
        });

    let bar_ids: Vec<&str> = action_bar_actions
        .iter()
        .map(|action| action.id.as_str())
        .collect();
    let has_talk = bar_ids.contains(&"speak") || bar_ids.contains(&"talk");
    let modal_covered: Vec<&str> = vec!["inspect_feature", "inspect_actor"];
    let current_room_id = runtime.current_room_id().unwrap_or_default();
    let mut overflow_actions: Vec<OverflowAction> = {
        content
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
                action_is_available(content, &state, a, &current_room_id)
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
                    panel_config: a.ui.panel_config.as_ref().map(|pc| PanelConfigData {
                        title: pc.title.clone(),
                        prompt: pc.prompt.clone(),
                        data_source: pc.data_source.clone(),
                        on_select: pc.on_select.clone(),
                    }),
                }
            })
            .collect()
    };

    if let Ok(active_stages) = runtime.active_stage_ids() {
        append_stage_menu_overflow_actions(&mut overflow_actions, content, &active_stages);
    }

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
                        .current_room_talk_options()
                        .map_err(|error| error.to_string())?
                        .into_iter()
                        .filter(|opt| {
                            if !is_attack {
                                return true;
                            }
                            let actor_id = opt.id.strip_prefix("actor:").unwrap_or(&opt.id);
                            // Never offer allies as attack targets so the
                            // player can't accidentally strike their own party.
                            state.stance(actor_id) != ActorStance::Allied
                        })
                        .map(|opt| {
                            let actor_id = opt.id.strip_prefix("actor:").unwrap_or(&opt.id);
                            PanelOptionData {
                                id: actor_id.to_string(),
                                title: opt.label.clone(),
                                subtitle: None,
                                command: Some(format!("{} {}", phrase, actor_id)),
                            }
                        })
                        .collect()
                }
                PanelDataSource::Exits => runtime
                    .room_switch_options()
                    .map_err(|error| error.to_string())?
                    .into_iter()
                    .map(|opt| PanelOptionData {
                        id: opt.command.clone(),
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
                    .current_room_look_options()
                    .map_err(|error| error.to_string())?
                    .into_iter()
                    .filter(|opt| opt.id.starts_with("feature:"))
                    .map(|opt| PanelOptionData {
                        id: opt.id.clone(),
                        title: opt.label.clone(),
                        subtitle: None,
                        command: Some(opt.command.clone()),
                    })
                    .collect(),
                PanelDataSource::CraftableItems => {
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
                                        let Some(gate) =
                                            gates.and_then(|g| g.get(*item_id))
                                        else {
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
            };
            panel_options.insert(panel_name.clone(), options);
        }
    }

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
        party: {
            let mut members: Vec<PartyMember> = Vec::new();
            for actor_id in state
                .relationships
                .iter()
                .filter_map(|(actor_id, relationship)| {
                    (relationship.follows_player
                        && actor_id.as_str() != content.settings.combat.player_actor_id)
                        .then_some(actor_id)
                })
            {
                let label = runtime
                    .actor_display_name(actor_id)
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| actor_id.clone());
                let level = state.actor_level(actor_id);
                if let Some(member) = members.iter_mut().find(|m| m.label == label) {
                    member.count += 1;
                } else {
                    members.push(PartyMember {
                        label,
                        count: 1,
                        level,
                    });
                }
            }
            members.sort_by(|a, b| a.label.cmp(&b.label));
            members
        },
        player: {
            let player_id = &content.settings.combat.player_actor_id;
            let health_stat = &content.settings.combat.health_stat_id;
            let hp = state
                .effective_actor_stat(content, player_id, health_stat)
                .max(0) as u32;
            let hp_max = content
                .stats
                .actor
                .get(health_stat)
                .and_then(|stat| stat.max)
                .map(|max| max.max(0) as u32)
                .unwrap_or(hp);
            let mut stats = content
                .stats
                .actor
                .iter()
                .filter(|(stat_id, _)| *stat_id != health_stat)
                .map(|(stat_id, _)| StatValue {
                    id: stat_id.clone(),
                    value: state.effective_actor_stat(content, player_id, stat_id),
                })
                .collect::<Vec<_>>();
            stats.sort_by(|a, b| a.id.cmp(&b.id));
            let (level, xp, xp_max) = xp_progress(&state, content);
            PlayerStatus {
                hp,
                hp_max,
                stats,
                level,
                xp,
                xp_max,
            }
        },
        levels_revealed: content.levels_revealed_for_room(&current_room_id),
        current_room_items: state
            .loose_room_items(&current_room_id)
            .into_iter()
            .map(|(item_id, count)| {
                let label = content
                    .item(&item_id)
                    .map(|item| item.label.clone())
                    .unwrap_or_else(|| item_id.clone());
                InventoryItem { label, count }
            })
            .collect(),
        equipped_items: {
            let mut items: Vec<EquippedItem> = state
                .equipment
                .iter()
                .map(|(slot, item_id)| {
                    let label = content
                        .item(item_id)
                        .map(|item| item.label.clone())
                        .unwrap_or_else(|| item_id.clone());
                    EquippedItem {
                        slot: slot.clone(),
                        label,
                    }
                })
                .collect();
            items.sort_by(|left, right| left.slot.cmp(&right.slot));
            items
        },
        inventory: {
            let mut inventory = runtime
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
                .collect::<Vec<_>>();
            inventory.sort_by(|left, right| left.label.cmp(&right.label));
            inventory
        },
        room_consumables: {
            content
                .room_consumables(&current_room_id)
                .into_iter()
                .filter(|c| {
                    runtime
                        .current_room_item_count(&c.consumable.id)
                        .map(|count| {
                            count
                                + runtime
                                    .feature_consumable_count(
                                        &current_room_id,
                                        &c.feature.id,
                                        &c.consumable.id,
                                    )
                                    .unwrap_or(0)
                        })
                        .unwrap_or(0)
                        > 0
                })
                .fold(Vec::<RoomConsumableGroup>::new(), |mut groups, c| {
                    let remaining = runtime
                        .current_room_item_count(&c.consumable.id)
                        .unwrap_or(0)
                        + runtime
                            .feature_consumable_count(
                                &current_room_id,
                                &c.feature.id,
                                &c.consumable.id,
                            )
                            .unwrap_or(0);
                    if let Some(group) = groups
                        .iter_mut()
                        .find(|g| g.feature_label == c.feature.label)
                    {
                        group.items.push(ConsumableInfo {
                            id: c.consumable.id.clone(),
                            label: c.consumable.label.clone(),
                            kind: format!("{:?}", c.consumable.kind).to_lowercase(),
                            stock: remaining,
                            is_crafted: highlighted_consumable(c.consumable),
                        });
                    } else {
                        groups.push(RoomConsumableGroup {
                            feature_label: c.feature.label.clone(),
                            items: vec![ConsumableInfo {
                                id: c.consumable.id.clone(),
                                label: c.consumable.label.clone(),
                                kind: format!("{:?}", c.consumable.kind).to_lowercase(),
                                stock: remaining,
                                is_crafted: highlighted_consumable(c.consumable),
                            }],
                        });
                    }
                    groups
                })
        },
        crafted_consumable_labels: content
            .room_consumables(&current_room_id)
            .into_iter()
            .filter(|c| highlighted_consumable(c.consumable))
            .map(|c| c.consumable.label.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect(),
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
