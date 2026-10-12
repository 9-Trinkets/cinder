use super::super::types::PlannedTurn;
use super::PlanningContext;
use crate::content::types::{
    ActionDefinition, ActionItemConsumerTarget, ActionItemStorageTarget, ContentPack,
    ItemStorageTarget,
};
use crate::engine::commands::resolve_actor_reference_input;
use crate::engine::events::WorldEvent;
use crate::engine::item_creation::{
    craftable_available, craftable_unlocked, item_creation_storage, plan_item_creation,
    render_item_creation_rejection,
};
use crate::engine::turn_policies::{command_availability_issue, command_unavailable_message};
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
    let event_id = action
        .content_event
        .as_ref()
        .map(|ce| ce.id.clone())
        .unwrap_or_else(|| action.id.clone());
    WorldEvent::ContentEvent { event_id, payload }
}

fn first_actor_in_room(
    content: &ContentPack,
    context: &PlanningContext<'_>,
) -> Option<(String, String)> {
    context
        .planner_state
        .actors(content)
        .find(|actor| {
            !context
                .planner_state
                .actor_is_defeated(&actor.id, &content.settings.combat.health_stat_id)
                && context
                    .planner_state
                    .actor_room_id(&actor.id, &actor.room_id)
                    == context.current_room_id
        })
        .map(|actor| (actor.id.clone(), actor.name.clone()))
}

fn craftable_matches_input(content: &ContentPack, craftable_id: &str, input_val: &str) -> bool {
    if craftable_id.eq_ignore_ascii_case(input_val) {
        return true;
    }
    if let Some(item) = content.item(craftable_id)
        && item.label.eq_ignore_ascii_case(input_val)
    {
        return true;
    }
    let input_lower = input_val.to_ascii_lowercase();
    let normalized = input_lower.replace(' ', "");
    if craftable_id
        .replace('-', "")
        .eq_ignore_ascii_case(&normalized)
        || craftable_id.to_ascii_lowercase().starts_with(&normalized)
    {
        return true;
    }
    if let Some(craftable_item) = content.item(craftable_id) {
        let label_norm = craftable_item.label.to_ascii_lowercase().replace(' ', "");
        if label_norm == normalized
            || label_norm.starts_with(&normalized)
            || craftable_item
                .label
                .to_ascii_lowercase()
                .split_whitespace()
                .any(|w| w == input_lower)
        {
            return true;
        }
    }
    false
}

fn resolved_created_item_id(
    content: &ContentPack,
    action: &ActionDefinition,
    input: Option<&str>,
    context: &PlanningContext<'_>,
) -> Option<String> {
    let item_creation = action.item_creation.as_ref()?;
    let item_id = &item_creation.creates_item;
    let state = context.planner_state;
    let room_id = context.current_room_id;
    if !item_creation.craftable_items.is_empty() {
        if let Some(input_val) = input.map(str::trim).filter(|s| !s.is_empty()) {
            return item_creation
                .craftable_items
                .iter()
                .find(|craftable_id| {
                    craftable_unlocked(state, item_creation, craftable_id)
                        && craftable_matches_input(content, craftable_id, input_val)
                })
                .cloned();
        }
        return item_creation
            .craftable_items
            .iter()
            .find(|craftable_id| {
                craftable_available(content, state, item_creation, craftable_id, room_id)
            })
            .cloned();
    }
    if !item_creation.creates_item_target_template.is_empty() {
        let input_val = input.unwrap_or_default().trim();
        return Some(
            resolve_actor_reference_input(content, state, room_id, input_val)
                .map(|resolved| item_creation.resolve_target_item_id(Some(&resolved.actor_id)))
                .unwrap_or_else(|| item_id.clone()),
        );
    }
    Some(
        (!item_creation.creates_item_story_var.is_empty())
            .then_some(item_creation.creates_item_story_var.as_str())
            .and_then(|var_key| state.story_vars.get(var_key))
            .map(|value| value.to_string())
            .unwrap_or_else(|| item_id.clone()),
    )
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

    let metadata = action
        .player_command
        .as_ref()
        .unwrap_or_else(|| panic!("action '{}' should define player_command", action.id));
    let plan = plan_item_creation(
        content,
        context.planner_state,
        action,
        resolved_created_item_id(content, action, input, context),
        context.current_room_id,
        &content.settings.combat.player_actor_id,
    );
    if let Some(rejection) = &plan.rejection {
        planned.events.push(WorldEvent::ActionRejected {
            message: render_item_creation_rejection(
                content,
                rejection,
                content.opening.title.as_str(),
            ),
        });
        return false;
    }
    let created_item_id = plan.resolved_id;
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
    if action.content_event.is_some() {
        planned
            .events
            .push(content_event_for_command(action, payload));
    }
    if !action.sets_objective_progress.is_empty() || !action.clears_objective_progress.is_empty() {
        planned
            .events
            .push(WorldEvent::CommandObjectiveProgressApplied {
                command_id: action.id.clone(),
            });
    }

    if let Some(item_id) = created_item_id {
        if let Some(item) = content.item(&item_id) {
            let trace_lvl = context
                .planner_state
                .actor_skill_level(&content.settings.combat.player_actor_id, "trace");
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
                planned.events.push(WorldEvent::ActorStatAdjusted {
                    actor_id: content.settings.combat.player_actor_id.clone(),
                    stat: "mp".to_string(),
                    delta: -(mp_cost as i32),
                });
            }
        }
        planned.events.push(WorldEvent::ItemAcquired {
            item_id,
            storage: item_creation_storage(action),
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
                let (recipient_id, recipient_name) =
                    first_actor_in_room(content, context).expect("actor should be in room");
                (Some(recipient_id), Some(recipient_name))
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
                let (recipient_id, recipient_name) =
                    first_actor_in_room(content, context).expect("actor should be in room");
                (Some(recipient_id), Some(recipient_name))
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
