use super::super::types::PlannedTurn;
use super::PlanningContext;
use crate::content::types::{
    ActionDefinition, ActionItemConsumerTarget, ActionItemStorageTarget, ContentPack,
    ItemStorageTarget,
};
use crate::engine::commands::resolve_actor_reference_input;
use crate::engine::events::WorldEvent;
use crate::engine::turn_policies::{
    command_availability_issue, command_unavailable_message, story_var_is_truthy,
};
use std::collections::BTreeMap;

fn to_item_storage(storage: ActionItemStorageTarget) -> ItemStorageTarget {
    match storage {
        ActionItemStorageTarget::PlayerInventory => ItemStorageTarget::PlayerInventory,
        ActionItemStorageTarget::CurrentRoom => ItemStorageTarget::CurrentRoom,
    }
}

fn content_event_for_command(
    action: &ActionDefinition,
    payload: BTreeMap<String, String>,
) -> WorldEvent {
    let content_event = action
        .content_event
        .as_ref()
        .unwrap_or_else(|| panic!("action '{}' should define a content_event", action.id));
    WorldEvent::ContentEvent {
        event_id: content_event.id.clone(),
        payload,
    }
}

fn first_actor_in_room<'a>(
    content: &'a ContentPack,
    context: &PlanningContext<'_>,
) -> Option<&'a crate::content::types::ActorDefinition> {
    content.actors.iter().find(|actor| {
        !context
            .planner_state
            .actor_is_defeated(&actor.id, &content.settings.combat.health_stat_id)
            && context
                .planner_state
                .actor_room_id(&actor.id, &actor.room_id)
                == context.current_room_id
    })
}

fn resolved_created_item_id(
    content: &ContentPack,
    action: &ActionDefinition,
    input: Option<&str>,
    context: &PlanningContext<'_>,
) -> Option<String> {
    let item_creation = action.item_creation.as_ref()?;
    let item_id = &item_creation.creates_item;
    let craftable_unlocked = |craftable_id: &str| -> bool {
        match item_creation.craftable_item_gates.get(craftable_id) {
            None => true,
            Some(gate) if gate.is_empty() => true,
            Some(gate) => story_var_is_truthy(context.planner_state, gate),
        }
    };
    let craftable_available = |craftable_id: &str| {
        craftable_unlocked(craftable_id)
            && !trace_mark_already_in_room(
                content,
                context.planner_state,
                craftable_id,
                context.current_room_id,
            )
    };
    if !item_creation.craftable_items.is_empty() {
        if let Some(input_val) = input.map(str::trim).filter(|s| !s.is_empty()) {
            let input_lower = input_val.to_ascii_lowercase();
            if let Some(matched) = item_creation.craftable_items.iter().find(|craftable_id| {
                craftable_id.eq_ignore_ascii_case(input_val) && craftable_unlocked(craftable_id)
            }) {
                return Some(matched.clone());
            }
            if let Some(matched) = item_creation.craftable_items.iter().find(|craftable_id| {
                craftable_unlocked(craftable_id)
                    && content.item(craftable_id).is_some_and(|craftable_item| {
                        craftable_item.label.to_ascii_lowercase().replace(' ', "")
                            == input_lower.replace(' ', "")
                    })
            }) {
                return Some(matched.clone());
            }
        }
        return item_creation
            .craftable_items
            .iter()
            .find(|craftable_id| craftable_available(craftable_id))
            .cloned();
    }
    if !item_creation.creates_item_target_template.is_empty() {
        let input_val = input.unwrap_or_default().trim();
        return Some(
            resolve_actor_reference_input(
                content,
                context.planner_state,
                context.current_room_id,
                input_val,
            )
            .map(|resolved| item_creation.resolve_target_item_id(Some(&resolved.actor_id)))
            .unwrap_or_else(|| item_id.clone()),
        );
    }
    Some(
        (!item_creation.creates_item_story_var.is_empty())
            .then_some(item_creation.creates_item_story_var.as_str())
            .and_then(|var_key| context.planner_state.story_vars.get(var_key))
            .map(|value| value.to_string())
            .unwrap_or_else(|| item_id.clone()),
    )
}

fn trace_mark_already_in_room(
    content: &ContentPack,
    state: &crate::engine::state::WorldState,
    item_id: &str,
    room_id: &str,
) -> bool {
    content.item(item_id).is_some_and(|item| item.trace_mark)
        && state.has_item_in_storage(item_id, ItemStorageTarget::CurrentRoom, room_id)
}

pub(super) fn plan_content_command(
    content: &ContentPack,
    action: &ActionDefinition,
    input: Option<&str>,
    context: &PlanningContext<'_>,
    planned: &mut PlannedTurn,
) -> bool {
    if let Some(issue) = command_availability_issue(content, context.planner_state, action) {
        planned.events.push(WorldEvent::ActionRejected {
            message: command_unavailable_message(content, action, &issue),
        });
        return false;
    }

    // Check room restrictions
    if !action.available.allowed_rooms.is_empty()
        && !action
            .available
            .allowed_rooms
            .contains(&context.current_room_id.to_string())
    {
        let needed = action
            .available
            .allowed_rooms
            .first()
            .and_then(|id| content.room(id))
            .map(|r| r.title.as_str())
            .unwrap_or("another room");
        let command_lower = action.command.to_lowercase();
        planned.events.push(WorldEvent::ActionRejected {
            message: content
                .render_message(
                    "error.cannot_command_here",
                    &[("command", command_lower.as_str()), ("room", needed)],
                )
                .unwrap_or_default(),
        });
        return false;
    }

    // Check item requirement (consumes_item, consumes_any, or requires_any)
    if let Some(item_id) = &action.available.consumes_item {
        if !context.planner_state.has_item_in_storage(
            item_id,
            to_item_storage(action.available.consumes_item_storage.clone()),
            context.current_room_id,
        ) {
            let label = content
                .item(item_id)
                .map(|i| i.label.as_str())
                .unwrap_or(item_id);
            planned.events.push(WorldEvent::ActionRejected {
                message: content
                    .render_message("error.missing_item", &[("label", label)])
                    .unwrap_or_default(),
            });
            return false;
        }
        if action.item_consumer == ActionItemConsumerTarget::FirstActorInRoom
            && first_actor_in_room(content, context).is_none()
        {
            planned.events.push(WorldEvent::ActionRejected {
                message: content
                    .render_message("error.no_actor_to_serve", &[])
                    .unwrap_or_default(),
            });
            return false;
        }
    }
    if !action.available.requires_any.is_empty() || !action.available.consumes_any.is_empty() {
        let all_required: Vec<_> = action
            .available
            .requires_any
            .iter()
            .chain(action.available.consumes_any.iter())
            .collect();
        let has_any = all_required.iter().any(|id| {
            let storage = if action
                .available
                .consumes_any
                .iter()
                .any(|candidate| candidate == *id)
            {
                to_item_storage(action.available.consumes_any_storage.clone())
            } else {
                to_item_storage(action.available.requires_any_storage.clone())
            };
            context
                .planner_state
                .has_item_in_storage(id, storage, context.current_room_id)
        });
        if !has_any {
            planned.events.push(WorldEvent::ActionRejected {
                message: content
                    .render_message("error.nothing_to_consume", &[])
                    .unwrap_or_default(),
            });
            return false;
        }
        if !action.available.consumes_any.is_empty()
            && action.item_consumer == ActionItemConsumerTarget::FirstActorInRoom
            && first_actor_in_room(content, context).is_none()
        {
            planned.events.push(WorldEvent::ActionRejected {
                message: content
                    .render_message("error.no_actor_to_serve", &[])
                    .unwrap_or_default(),
            });
            return false;
        }
    }

    let metadata = action
        .player_command
        .as_ref()
        .unwrap_or_else(|| panic!("action '{}' should define player_command", action.id));
    let created_item_id = resolved_created_item_id(content, action, input, context);
    if action
        .item_creation
        .as_ref()
        .is_some_and(|creation| !creation.craftable_items.is_empty())
        && created_item_id.is_none()
    {
        planned.events.push(WorldEvent::ActionRejected {
            message: content
                .render_message("error.no_craftable_available", &[])
                .unwrap_or_default(),
        });
        return false;
    }
    if let Some(item_id) = created_item_id.as_deref()
        && action
            .item_creation
            .as_ref()
            .is_some_and(|creation| creation.storage == ActionItemStorageTarget::CurrentRoom)
        && trace_mark_already_in_room(
            content,
            context.planner_state,
            item_id,
            context.current_room_id,
        )
    {
        let label = content
            .item(item_id)
            .map(|item| item.label.as_str())
            .unwrap_or(item_id);
        planned.events.push(WorldEvent::ActionRejected {
            message: content
                .render_message("error.trace_mark_exists", &[("label", label)])
                .unwrap_or_default(),
        });
        return false;
    }
    let mut payload = BTreeMap::new();
    if let Some(input_metadata) = &metadata.input {
        let value = input.unwrap_or_default().trim();
        if input_metadata.required && value.is_empty() {
            planned.events.push(WorldEvent::ActionRejected {
                message: content.render_template(
                    &content.presentation.error_text.unknown_input,
                    &[
                        ("raw_input", context.raw_input),
                        ("available_commands", metadata.usage.as_str()),
                    ],
                ),
            });
            return false;
        }
        if !value.is_empty() {
            payload.insert(input_metadata.payload_key.clone(), value.to_string());
        }
    }

    // Content event (narrative) first, then item events
    planned
        .events
        .push(content_event_for_command(action, payload));
    if !action.sets_objective_progress.is_empty() || !action.clears_objective_progress.is_empty() {
        planned
            .events
            .push(WorldEvent::CommandObjectiveProgressApplied {
                command_id: action.id.clone(),
            });
    }

    if let Some(item_id) = created_item_id {
        planned.events.push(WorldEvent::ItemAcquired {
            item_id,
            storage: action
                .item_creation
                .as_ref()
                .map(|ic| to_item_storage(ic.storage.clone()))
                .unwrap_or_default(),
        });
    }
    if let Some(item_id) = &action.available.consumes_item {
        let (consumer_id, consumer_name) = match action.item_consumer {
            ActionItemConsumerTarget::None => (None, None),
            ActionItemConsumerTarget::Player => (
                Some(content.settings.combat.player_actor_id.clone()),
                Some("You".to_string()),
            ),
            ActionItemConsumerTarget::FirstActorInRoom => {
                let recipient =
                    first_actor_in_room(content, context).expect("actor should be in room");
                (Some(recipient.id.clone()), Some(recipient.name.clone()))
            }
        };
        planned.events.push(WorldEvent::ItemConsumed {
            item_id: item_id.clone(),
            storage: to_item_storage(action.available.consumes_item_storage.clone()),
            consumer_id,
            consumer_name,
        });
    }
    if !action.available.consumes_any.is_empty()
        && let Some(item_id) = action.available.consumes_any.iter().find(|id| {
            context.planner_state.has_item_in_storage(
                id,
                to_item_storage(action.available.consumes_any_storage.clone()),
                context.current_room_id,
            )
        })
    {
        let (consumer_id, consumer_name) = match action.item_consumer {
            ActionItemConsumerTarget::None => (None, None),
            ActionItemConsumerTarget::Player => (
                Some(content.settings.combat.player_actor_id.clone()),
                Some("You".to_string()),
            ),
            ActionItemConsumerTarget::FirstActorInRoom => {
                let recipient =
                    first_actor_in_room(content, context).expect("actor should be in room");
                (Some(recipient.id.clone()), Some(recipient.name.clone()))
            }
        };
        planned.events.push(WorldEvent::ItemConsumed {
            item_id: item_id.clone(),
            storage: to_item_storage(action.available.consumes_any_storage.clone()),
            consumer_id,
            consumer_name,
        });
    }

    metadata.advances_time
}
