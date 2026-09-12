use crate::content::types::{
    ActionDefinition, ActionItemStorageTarget, CommandEffect, ContentPack, ItemStorageTarget,
    PanelDataSource,
};
use crate::engine::state::{ActorStance, WorldState};

use super::objectives::{objective_progress_is_met, objective_progress_label};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CommandAvailabilityIssue {
    StageInactive,
    MissingObjectiveProgress(Vec<String>),
    BlockedByObjectiveProgress(Vec<String>),
    /// A room-item condition is unmet (`requires_room_item` / `requires_room_without_item`).
    RoomItemCondition(String),
}

pub(crate) fn command_availability_issue(
    content: &ContentPack,
    state: &WorldState,
    action: &ActionDefinition,
) -> Option<CommandAvailabilityIssue> {
    let a = &action.available;
    if !a.available_during.is_empty()
        && !a
            .available_during
            .iter()
            .any(|stage_id| state.active_objective_stage_ids.contains(stage_id))
    {
        return Some(CommandAvailabilityIssue::StageInactive);
    }

    let missing = a
        .required_objective_progress
        .iter()
        .filter(|progress| !objective_progress_is_met(content, state, progress))
        .map(|progress| objective_progress_label(content, progress))
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Some(CommandAvailabilityIssue::MissingObjectiveProgress(missing));
    }

    let blocked = a
        .blocked_by_objective_progress
        .iter()
        .filter(|progress| objective_progress_is_met(content, state, progress))
        .map(|progress| objective_progress_label(content, progress))
        .collect::<Vec<_>>();
    if !blocked.is_empty() {
        return Some(CommandAvailabilityIssue::BlockedByObjectiveProgress(blocked));
    }

    if !a.requires_room_item.is_empty()
        && !state.has_item_in_storage(
            &a.requires_room_item,
            ItemStorageTarget::CurrentRoom,
            &state.current_room_id,
        )
    {
        return Some(CommandAvailabilityIssue::RoomItemCondition(
            "required".to_string(),
        ));
    }
    if !a.requires_room_without_item.is_empty()
        && state.has_item_in_storage(
            &a.requires_room_without_item,
            ItemStorageTarget::CurrentRoom,
            &state.current_room_id,
        )
    {
        return Some(CommandAvailabilityIssue::RoomItemCondition(
            "forbidden".to_string(),
        ));
    }
    if !a.requires_story_var.is_empty() && !story_var_is_truthy(state, &a.requires_story_var) {
        return Some(CommandAvailabilityIssue::StageInactive);
    }

    None
}

/// A story variable counts as set/truthy when it exists and isn't a falsy
/// string ("", "false", "0").
pub(crate) fn story_var_is_truthy(state: &WorldState, key: &str) -> bool {
    match state.story_vars.get(key) {
        Some(value) => !matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "" | "false" | "0"
        ),
        None => false,
    }
}

pub(crate) fn command_unavailable_message(
    content: &ContentPack,
    action: &ActionDefinition,
    issue: &CommandAvailabilityIssue,
) -> String {
    let verb = action.command.to_ascii_lowercase();
    match issue {
        CommandAvailabilityIssue::StageInactive => content
            .render_message("error.command_not_now", &[("verb", verb.as_str())])
            .unwrap_or_default(),
        CommandAvailabilityIssue::MissingObjectiveProgress(labels) => {
            let labels_text = labels.join(", ");
            content
                .render_message(
                    "error.command_needs_more",
                    &[("verb", verb.as_str()), ("labels", labels_text.as_str())],
                )
                .unwrap_or_default()
        }
        CommandAvailabilityIssue::BlockedByObjectiveProgress(labels) => {
            let labels_text = labels.join(", ");
            content
                .render_message(
                    "error.command_blocked_by_progress",
                    &[("verb", verb.as_str()), ("labels", labels_text.as_str())],
                )
                .unwrap_or_default()
        }
        CommandAvailabilityIssue::RoomItemCondition(_) => content
            .render_message("error.command_not_now", &[("verb", verb.as_str())])
            .unwrap_or_default(),
    }
}

fn to_item_storage(storage: ActionItemStorageTarget) -> ItemStorageTarget {
    match storage {
        ActionItemStorageTarget::PlayerInventory => ItemStorageTarget::PlayerInventory,
        ActionItemStorageTarget::CurrentRoom => ItemStorageTarget::CurrentRoom,
    }
}

pub fn action_is_available(
    content: &ContentPack,
    state: &WorldState,
    action: &ActionDefinition,
    context_room_id: &str,
) -> bool {
    let a = &action.available;

    if !a.allowed_rooms.is_empty() && !a.allowed_rooms.contains(&context_room_id.to_string()) {
        return false;
    }

    if !a.requires_room_item.is_empty()
        && !state.has_item_in_storage(
            &a.requires_room_item,
            ItemStorageTarget::CurrentRoom,
            context_room_id,
        )
    {
        return false;
    }
    if !a.requires_room_without_item.is_empty()
        && state.has_item_in_storage(
            &a.requires_room_without_item,
            ItemStorageTarget::CurrentRoom,
            context_room_id,
        )
    {
        return false;
    }
    if !a.requires_story_var.is_empty() && !story_var_is_truthy(state, &a.requires_story_var) {
        return false;
    }

    if !a.available_during.is_empty()
        && !a
            .available_during
            .iter()
            .any(|stage_id| state.active_objective_stage_ids.contains(stage_id))
    {
        return false;
    }

    // Item-possession gating so the bar hides actions whose items you lack.
    if let Some(item_id) = &a.consumes_item
        && !state.has_item_in_storage(
            item_id,
            to_item_storage(a.consumes_item_storage.clone()),
            context_room_id,
        )
    {
        return false;
    }
    if !a.requires_any.is_empty() || !a.consumes_any.is_empty() {
        let all_required: Vec<_> = a.requires_any.iter().chain(a.consumes_any.iter()).collect();
        let has_any = all_required.iter().any(|id| {
            let storage = if a.consumes_any.iter().any(|c| c == *id) {
                to_item_storage(a.consumes_any_storage.clone())
            } else {
                to_item_storage(a.requires_any_storage.clone())
            };
            state.has_item_in_storage(id, storage, context_room_id)
        });
        if !has_any {
            return false;
        }
    }
    // Item-state gating derived from the effect, so content never has to restate
    // the obvious: equipping/using needs the item in the inventory, unequipping
    // needs it worn.
    if action.has_effect(CommandEffect::EquipItem) && !state.has_item(&action.item_id) {
        return false;
    }
    if action.has_effect(CommandEffect::UseItem) && !state.has_item(&action.item_id) {
        return false;
    }
    if action.has_effect(CommandEffect::UnequipItem) {
        let equipped = content
            .item(&action.item_id)
            .is_some_and(|item| state.item_is_equipped(item));
        if !equipped {
            return false;
        }
    }

    for progress in &a.required_objective_progress {
        if !objective_progress_is_met(content, state, progress) {
            return false;
        }
    }

    for progress in &a.blocked_by_objective_progress {
        if objective_progress_is_met(content, state, progress) {
            return false;
        }
    }

    if !action_has_available_target(content, state, action, context_room_id) {
        return false;
    }

    true
}

/// Whether a target-selection action currently has at least one eligible
/// target. This is the generic "hide when there are no targets" rule: an
/// action whose panel selects a target (actors in the room, or craftable
/// items) is hidden while its target list is empty. Informational panels
/// (`features`/look, `exits`/move) are never hidden here.
fn action_has_available_target(
    content: &ContentPack,
    state: &WorldState,
    action: &ActionDefinition,
    room_id: &str,
) -> bool {
    let Some(panel_config) = &action.ui.panel_config else {
        return true;
    };
    match panel_config.data_source {
        PanelDataSource::ActorsInRoom => {
            let is_attack = action.has_effect(CommandEffect::AttackTarget);
            content.actors.iter().any(|actor| {
                if content.is_player_actor(&actor.id)
                    || !state.actor_is_in_room(content, &actor.id, room_id)
                    || state.actor_is_defeated(&actor.id, &content.settings.combat.health_stat_id)
                {
                    return false;
                }
                if is_attack {
                    let relationship = state.relationship(&actor.id);
                    // Attack never targets party members (allies or followers).
                    relationship.stance != ActorStance::Allied && !relationship.follows_player
                } else {
                    true
                }
            })
        }
        PanelDataSource::CraftableItems => action.item_creation.as_ref().is_some_and(|item_creation| {
            item_creation.craftable_items.iter().any(|item_id| {
                let unlocked = match item_creation.craftable_item_gates.get(item_id) {
                    None => true,
                    Some(gate) => gate.is_empty() || story_var_is_truthy(state, gate),
                };
                let already_traced = content.item(item_id).is_some_and(|item| item.trace_mark)
                    && state.has_item_in_storage(
                        item_id,
                        crate::content::types::ItemStorageTarget::CurrentRoom,
                        room_id,
                    );
                unlocked && !already_traced
            })
        }),
        PanelDataSource::Exits | PanelDataSource::Features => true,
        PanelDataSource::LooseRoomItems => state.loose_room_items(room_id).iter().any(|(item_id, _)| {
            content.item(item_id).is_none_or(|item| item.is_takeable())
        }),
        PanelDataSource::InventoryItems => state
            .player_inventory
            .iter()
            .any(|(item_id, count)| {
                *count > 0 && !state.equipment.values().any(|equipped| equipped == item_id)
            }),
    }
}
