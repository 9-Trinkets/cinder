//! Shared item-creation decision used by the turn planner, the reducer, and the
//! UI craftable panel. The three call sites differ only in how they resolve a
//! craftable id from player input and how they surface a rejection (a rejected
//! action, silent no-op, or narration), so the availability rules and the
//! validate-then-reject chain live here once.

use crate::content::types::{
    ActionDefinition, ActionItemCreation, ActionItemStorageTarget, ContentPack, ItemStorageTarget,
};
use crate::engine::state::WorldState;
use crate::engine::turn_policies::story_var_is_truthy;

/// The single reason a resolved item-creation request cannot proceed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemCreationRejection {
    /// The action offers craftable items but no craftable id resolved.
    NoCraftableAvailable,
    /// A trace-mark item is already present in the target room.
    TraceMarkExists { item_id: String },
    /// The item's active spawned-instance cap is reached.
    InstanceLimit { item_id: String, max: usize },
    /// The actor lacks the MP to create the item.
    InsufficientMp {
        item_id: String,
        mp_cost: u32,
        current_mp: u32,
    },
}

/// Outcome of [`plan_item_creation`]: the resolved craftable id (if any) and,
/// when present, the first rule that blocked creation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemCreationPlan {
    pub resolved_id: Option<String>,
    pub rejection: Option<ItemCreationRejection>,
}

/// Whether a craftable item id is unlocked by its story-var gate. Items with no
/// gate (or an empty gate) are always unlocked.
pub fn craftable_unlocked(
    state: &WorldState,
    creation: &ActionItemCreation,
    item_id: &str,
) -> bool {
    match creation.craftable_item_gates.get(item_id) {
        None => true,
        Some(gate) if gate.is_empty() => true,
        Some(gate) => story_var_is_truthy(state, gate),
    }
}

/// Whether a trace-mark item is already present in `room_id`.
pub fn trace_mark_already_in_room(
    content: &ContentPack,
    state: &WorldState,
    item_id: &str,
    room_id: &str,
) -> bool {
    content.item(item_id).is_some_and(|item| item.trace_mark)
        && state.has_item_in_storage(item_id, ItemStorageTarget::CurrentRoom, room_id)
}

/// Whether a craftable id can currently be offered: unlocked and not already
/// traced into the target room.
pub fn craftable_available(
    content: &ContentPack,
    state: &WorldState,
    creation: &ActionItemCreation,
    item_id: &str,
    room_id: &str,
) -> bool {
    craftable_unlocked(state, creation, item_id)
        && !(creation.storage == ActionItemStorageTarget::CurrentRoom
            && trace_mark_already_in_room(content, state, item_id, room_id))
}

fn item_storage(storage: ActionItemStorageTarget) -> ItemStorageTarget {
    match storage {
        ActionItemStorageTarget::PlayerInventory => ItemStorageTarget::PlayerInventory,
        ActionItemStorageTarget::CurrentRoom => ItemStorageTarget::CurrentRoom,
    }
}

/// Applies the shared item-creation validation chain to an already-resolved
/// craftable id. Callers resolve `resolved_id` from player input with their own
/// matching rules; everything after that (craftable presence, trace mark,
/// instance cap, MP cost) is identical across sites.
pub fn plan_item_creation(
    content: &ContentPack,
    state: &WorldState,
    action: &ActionDefinition,
    resolved_id: Option<String>,
    room_id: &str,
    actor_id: &str,
) -> ItemCreationPlan {
    let creation = action.item_creation.as_ref();
    if creation.is_some_and(|c| !c.craftable_items.is_empty()) && resolved_id.is_none() {
        return ItemCreationPlan {
            resolved_id: None,
            rejection: Some(ItemCreationRejection::NoCraftableAvailable),
        };
    }
    if let Some(item_id) = resolved_id.as_deref() {
        let storage = creation.map(|c| c.storage.clone()).unwrap_or_default();
        if storage == ActionItemStorageTarget::CurrentRoom
            && trace_mark_already_in_room(content, state, item_id, room_id)
        {
            return ItemCreationPlan {
                resolved_id: Some(item_id.to_string()),
                rejection: Some(ItemCreationRejection::TraceMarkExists {
                    item_id: item_id.to_string(),
                }),
            };
        }
        if let Some(item) = content.item(item_id) {
            if let Some(max) = item.max_active_instances
                && !item.spawn_template_id.is_empty()
                && state.active_spawned_actor_count(content, &item.spawn_template_id) >= max
            {
                return ItemCreationPlan {
                    resolved_id: Some(item_id.to_string()),
                    rejection: Some(ItemCreationRejection::InstanceLimit {
                        item_id: item_id.to_string(),
                        max,
                    }),
                };
            }
            let trace_lvl = state.actor_skill_level(actor_id, "trace");
            let mp_cost = if item.trace_mark {
                crate::engine::skills::effective_trace_mp_cost_for_pack(
                    content,
                    item.mp_cost,
                    trace_lvl,
                )
            } else {
                item.mp_cost
            };
            if mp_cost > 0 {
                let current_mp = state.actor_stat_u32(actor_id, "mp");
                if current_mp < mp_cost {
                    return ItemCreationPlan {
                        resolved_id: Some(item_id.to_string()),
                        rejection: Some(ItemCreationRejection::InsufficientMp {
                            item_id: item_id.to_string(),
                            mp_cost,
                            current_mp,
                        }),
                    };
                }
            }
        }
    }
    ItemCreationPlan {
        resolved_id,
        rejection: None,
    }
}

/// Renders the player-facing message for an item-creation rejection.
pub fn render_item_creation_rejection(
    content: &ContentPack,
    rejection: &ItemCreationRejection,
    actor_name: &str,
) -> String {
    match rejection {
        ItemCreationRejection::NoCraftableAvailable => content
            .render_message("error.no_craftable_available", &[])
            .unwrap_or_default(),
        ItemCreationRejection::TraceMarkExists { item_id } => {
            let label = content.item_label(item_id);
            content
                .render_message("error.trace_mark_exists", &[("label", label)])
                .unwrap_or_default()
        }
        ItemCreationRejection::InstanceLimit { item_id, max } => {
            let item = content.item(item_id);
            let template_id = item
                .map(|i| i.spawn_template_id.as_str())
                .unwrap_or_default();
            let template_name = content
                .actor(template_id)
                .map(|a| a.name.as_str())
                .unwrap_or(template_id);
            let msg_key = item
                .map(|i| i.max_instances_message.as_str())
                .filter(|key| !key.is_empty())
                .unwrap_or("sigil.spawn_limit");
            let max_str = max.to_string();
            content
                .render_message(
                    msg_key,
                    &[("actor", template_name), ("max", max_str.as_str())],
                )
                .unwrap_or_else(|| format!("You cannot sustain more than {max} active summons."))
        }
        ItemCreationRejection::InsufficientMp {
            item_id,
            mp_cost,
            current_mp,
        } => {
            let cost_str = mp_cost.to_string();
            let current_str = current_mp.to_string();
            content
                .render_message(
                    "magic.insufficient_mp",
                    &[
                        ("actor", actor_name),
                        ("item", content.item_label(item_id)),
                        ("mp_cost", cost_str.as_str()),
                        ("mp", current_str.as_str()),
                    ],
                )
                .unwrap_or_else(|| {
                    format!(
                        "You do not have enough magic to trace that (needs {mp_cost} MP, have {current_mp})."
                    )
                })
        }
    }
}

/// Per-craftable UI state for the item-creation panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CraftableOptionState {
    pub item_id: String,
    pub already_traced: bool,
    pub at_instance_limit: bool,
    pub insufficient_mp: bool,
    pub mp_cost: u32,
    pub actor_mp: u32,
}

impl CraftableOptionState {
    /// Whether the option should be shown disabled.
    pub fn disabled(&self) -> bool {
        self.already_traced || self.at_instance_limit || self.insufficient_mp
    }
}

/// Lists the unlocked craftable items for an action along with the flags the
/// UI needs to render and disable each option.
pub fn craftable_option_states(
    content: &ContentPack,
    state: &WorldState,
    action: &ActionDefinition,
    room_id: &str,
    actor_id: &str,
) -> Vec<CraftableOptionState> {
    let Some(creation) = action.item_creation.as_ref() else {
        return Vec::new();
    };
    let actor_mp = state.actor_stat_u32(actor_id, "mp");
    let trace_lvl = state.actor_skill_level(actor_id, "trace");
    creation
        .craftable_items
        .iter()
        .filter(|item_id| craftable_unlocked(state, creation, item_id))
        .map(|item_id| {
            let item = content.item(item_id);
            let already_traced = trace_mark_already_in_room(content, state, item_id, room_id);
            let at_spawn_limit = item.is_some_and(|i| {
                i.max_active_instances.is_some_and(|max| {
                    !i.spawn_template_id.is_empty()
                        && state.active_spawned_actor_count(content, &i.spawn_template_id) >= max
                })
            });
            let at_instance_limit = at_spawn_limit;
            let base_mp = item.map(|i| i.mp_cost).unwrap_or(0);
            let mp_cost = if item.is_some_and(|i| i.trace_mark) {
                crate::engine::skills::effective_trace_mp_cost_for_pack(content, base_mp, trace_lvl)
            } else {
                base_mp
            };
            CraftableOptionState {
                item_id: item_id.clone(),
                already_traced,
                at_instance_limit,
                insufficient_mp: mp_cost > actor_mp,
                mp_cost,
                actor_mp,
            }
        })
        .collect()
}

/// Converts an action's declared storage target to the runtime storage enum.
/// Shared so callers applying an [`ItemCreationPlan`] use one mapping.
pub fn item_creation_storage(action: &ActionDefinition) -> ItemStorageTarget {
    action
        .item_creation
        .as_ref()
        .map(|creation| item_storage(creation.storage.clone()))
        .unwrap_or_default()
}
