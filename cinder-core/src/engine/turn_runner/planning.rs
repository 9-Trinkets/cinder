use super::types::{PendingDialogue, PlannedTurn};
use crate::content::types::{
    CommandDefinition, CommandEffect, CommandOutcomeMode, ContentPack, PlayerCommandTargetMode,
};
use crate::engine::commands::{resolve_actor_reference_input, unknown_target_token};
use crate::engine::dialogue_grounding::viewer_participant_id;
use crate::engine::events::{ObservationMode, WorldEvent};
use crate::engine::state::{display_actor_name, WorldState};
use std::collections::BTreeMap;

pub(super) struct PlanningContext<'a> {
    pub(super) raw_input: &'a str,
    pub(super) current_room_id: &'a str,
    pub(super) planner_state: &'a WorldState,
    pub(super) channel_surfing_only: bool,
    pub(super) turn_number: u32,
}

fn pending_dialogue_for(
    content: &ContentPack,
    context: &PlanningContext<'_>,
    actor_id: String,
    other_person_message: Option<String>,
) -> PendingDialogue {
    PendingDialogue {
        actor_id,
        current_room_id: context.current_room_id.to_string(),
        raw_input: context.raw_input.to_string(),
        other_person_id: viewer_participant_id(content),
        other_person_name: content.opening.title.clone(),
        other_person_message,
        turn_number: context.turn_number,
    }
}

fn content_event_for_command(
    command: &CommandDefinition,
    payload: BTreeMap<String, String>,
) -> WorldEvent {
    let content_event = command
        .content_event
        .as_ref()
        .unwrap_or_else(|| panic!("command '{}' should define a content_event", command.id));
    WorldEvent::ContentEvent {
        event_id: content_event.id.clone(),
        payload,
    }
}

pub(super) fn plan_content_command(
    content: &ContentPack,
    command: &CommandDefinition,
    input: Option<&str>,
    context: &PlanningContext<'_>,
    planned: &mut PlannedTurn,
) -> bool {
    // Check room restrictions
    if !command.allowed_rooms.is_empty()
        && !command.allowed_rooms.contains(&context.current_room_id.to_string())
    {
        let needed = command
            .allowed_rooms
            .first()
            .and_then(|id| content.room(id))
            .map(|r| r.title.as_str())
            .unwrap_or("another room");
        planned.events.push(WorldEvent::ActionRejected {
            message: format!(
                "You can't {} here. Head to the {} first.",
                command.command.to_lowercase(),
                needed,
            ),
        });
        return false;
    }

    // Check item requirement (consumes_item, consumes_any, or requires_any)
    if let Some(item_id) = &command.consumes_item {
        if !context.planner_state.has_item(item_id) {
            let label = content
                .item(item_id)
                .map(|i| i.label.as_str())
                .unwrap_or(item_id);
            planned.events.push(WorldEvent::ActionRejected {
                message: format!("You don't have any {label} to serve."),
            });
            return false;
        }
        // Check target actor is in the same room
        let actors_here: Vec<_> = content
            .actors
            .iter()
            .filter(|actor| {
                context
                    .planner_state
                    .actor_room_id(&actor.id, &actor.room_id)
                    == context.current_room_id
            })
            .collect();
        if actors_here.is_empty() {
            planned.events.push(WorldEvent::ActionRejected {
                message: "There is no one here to serve.".to_string(),
            });
            return false;
        }
    }
    if !command.requires_any.is_empty() || !command.consumes_any.is_empty() {
        let all_required: Vec<_> = command
            .requires_any
            .iter()
            .chain(command.consumes_any.iter())
            .collect();
        let has_any = all_required
            .iter()
            .any(|id| context.planner_state.has_item(id));
        if !has_any {
            planned.events.push(WorldEvent::ActionRejected {
                message: "You don't have anything to consume.".to_string(),
            });
            return false;
        }
    }

    let metadata = command
        .player_command
        .as_ref()
        .unwrap_or_else(|| panic!("command '{}' should define player_command", command.id));
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
        .push(content_event_for_command(command, payload));

    if let Some(item_id) = &command.creates_item {
        planned.events.push(WorldEvent::ItemAcquired {
            item_id: item_id.clone(),
        });
    }
    if let Some(item_id) = &command.consumes_item {
        // Use the first actor in the room as the recipient
        let recipient = content
            .actors
            .iter()
            .find(|actor| {
                context
                    .planner_state
                    .actor_room_id(&actor.id, &actor.room_id)
                    == context.current_room_id
            })
            .expect("actor should be in room");
        planned.events.push(WorldEvent::ItemConsumed {
            item_id: item_id.clone(),
            consumer_id: recipient.id.clone(),
            consumer_name: recipient.name.clone(),
        });
    }
    if !command.consumes_any.is_empty() {
        if let Some(item_id) = command
            .consumes_any
            .iter()
            .find(|id| context.planner_state.has_item(id))
        {
            planned.events.push(WorldEvent::ItemConsumed {
                item_id: item_id.clone(),
                consumer_id: "player".to_string(),
                consumer_name: "You".to_string(),
            });
        }
    }

    metadata.advances_time
}

pub(super) fn plan_observe_room(context: &PlanningContext<'_>, planned: &mut PlannedTurn) -> bool {
    planned.events.push(WorldEvent::CurrentRoomObserved {
        room_id: context.current_room_id.to_string(),
        mode: ObservationMode::Detailed,
    });
    false
}

pub(super) fn plan_observe_target(
    content: &ContentPack,
    target: &str,
    context: &PlanningContext<'_>,
    planned: &mut PlannedTurn,
) -> bool {
    if let Some(actor) = content.resolve_actor(target).or_else(|| {
        content
            .actors
            .iter()
            .find(|actor| display_actor_name(context.planner_state, actor).eq_ignore_ascii_case(target))
    }) {
        let actor_name = display_actor_name(context.planner_state, actor);
        if context
            .planner_state
            .actor_room_id(&actor.id, &actor.room_id)
            == context.current_room_id
        {
            planned.events.push(WorldEvent::ActorObserved {
                actor_id: actor.id.clone(),
            });
        } else {
            planned.events.push(WorldEvent::ActionRejected {
                message: content.render_template(
                    &content.presentation.error_text.actor_not_here,
                    &[("actor_name", actor_name.as_str())],
                ),
            });
        }
    } else if let Some(feature) = content.resolve_feature_in_room(context.current_room_id, target) {
        planned.events.push(WorldEvent::FeatureObserved {
            room_id: context.current_room_id.to_string(),
            feature_id: feature.id.clone(),
        });
    } else if let Some(item) = content.resolve_item_in_inventory(context.planner_state, target) {
        planned.events.push(WorldEvent::ItemObserved {
            item_id: item.id.clone(),
        });
    } else {
        planned.events.push(WorldEvent::ActionRejected {
            message: content.render_template(
                &content.presentation.error_text.feature_unknown,
                &[("target", target)],
            ),
        });
    }
    false
}

pub(super) fn plan_move_to_room_target(
    content: &ContentPack,
    target: &str,
    advances_time: bool,
    context: &PlanningContext<'_>,
    planned: &mut PlannedTurn,
) -> bool {
    if let Some(exit) = content.resolve_exit(context.current_room_id, target) {
        planned.events.push(WorldEvent::PlayerMoved {
            from_room_id: context.current_room_id.to_string(),
            to_room_id: exit.room_id.clone(),
        });
        planned.events.push(WorldEvent::CurrentRoomObserved {
            room_id: exit.room_id.clone(),
            mode: ObservationMode::Summary,
        });
        advances_time
    } else {
        planned.events.push(WorldEvent::ActionRejected {
            message: content.render_template(
                &content.presentation.error_text.cannot_go,
                &[("target", target)],
            ),
        });
        false
    }
}

pub(super) fn plan_targeted_state_command(
    content: &ContentPack,
    command: &CommandDefinition,
    input: Option<&str>,
    context: &PlanningContext<'_>,
    planned: &mut PlannedTurn,
) -> bool {
    let metadata = command
        .player_command
        .as_ref()
        .unwrap_or_else(|| panic!("command '{}' should define player_command", command.id));
    match metadata.target_mode {
        PlayerCommandTargetMode::RoomReference => plan_move_to_room_target(
            content,
            input.unwrap_or_default().trim(),
            metadata.advances_time,
            context,
            planned,
        ),
        PlayerCommandTargetMode::ActorOrFeatureReference => {
            plan_observe_target(content, input.unwrap_or_default().trim(), context, planned)
        }
        other => panic!(
            "stateful player command '{}' has unsupported target_mode '{other:?}'",
            command.id
        ),
    }
}

pub(super) fn plan_command_effects(
    content: &ContentPack,
    command: &CommandDefinition,
    input: Option<&str>,
    context: &PlanningContext<'_>,
    planned: &mut PlannedTurn,
) -> bool {
    if command.has_effect(CommandEffect::ObserveRoom) {
        let target = input.unwrap_or_default().trim();
        if target.is_empty() {
            plan_observe_room(context, planned)
        } else {
            plan_observe_target(content, target, context, planned)
        }
    } else if command.has_any_effect(&[
        CommandEffect::ObserveFeature,
        CommandEffect::ObserveActor,
        CommandEffect::MoveActor,
    ]) {
        plan_targeted_state_command(content, command, input, context, planned)
    } else {
        panic!(
            "player command '{}' uses command effects without a supported planner effect",
            command.id
        )
    }
}

pub(super) fn plan_dialogue_command(
    content: &ContentPack,
    command: &CommandDefinition,
    input: Option<&str>,
    context: &PlanningContext<'_>,
    planned: &mut PlannedTurn,
) -> bool {
    let metadata = command
        .player_command
        .as_ref()
        .unwrap_or_else(|| panic!("command '{}' should define player_command", command.id));
    if context.channel_surfing_only {
        planned.events.push(WorldEvent::UnknownInput {
            raw_input: context.raw_input.to_string(),
        });
        return false;
    }

    match metadata.target_mode {
        PlayerCommandTargetMode::ActorReference => {
            let remainder = input.unwrap_or_default();
            if let Some(resolved) = resolve_actor_reference_input(
                content,
                context.planner_state,
                context.current_room_id,
                remainder,
            ) {
                if resolved.actor_in_room {
                    planned.pending_dialogue = Some(pending_dialogue_for(
                        content,
                        context,
                        resolved.actor_id,
                        resolved.player_message,
                    ));
                    metadata.advances_time
                } else {
                    planned.events.push(WorldEvent::ActionRejected {
                        message: content.render_template(
                            &content.presentation.error_text.actor_not_here,
                            &[("actor_name", resolved.actor_name.as_str())],
                        ),
                    });
                    false
                }
            } else {
                planned.events.push(WorldEvent::ActionRejected {
                    message: content.render_template(
                        &content.presentation.error_text.actor_unknown,
                        &[("target", unknown_target_token(remainder).as_str())],
                    ),
                });
                false
            }
        }
        PlayerCommandTargetMode::FirstActorInRoom => {
            let actors_here: Vec<_> = content
                .actors
                .iter()
                .filter(|actor| {
                    context
                        .planner_state
                        .actor_room_id(&actor.id, &actor.room_id)
                        == context.current_room_id
                })
                .collect();
            if let Some(actor) = actors_here.first() {
                planned.pending_dialogue = Some(pending_dialogue_for(
                    content,
                    context,
                    actor.id.clone(),
                    None,
                ));
                metadata.advances_time
            } else {
                planned.events.push(WorldEvent::ActionRejected {
                    message: "There is no one here to listen to.".to_string(),
                });
                false
            }
        }
        other => {
            panic!(
                "dialogue player command '{}' has unsupported target_mode '{other:?}'",
                command.id,
            )
        }
    }
}

pub(super) fn plan_authored_command(
    content: &ContentPack,
    command_id: &str,
    input: Option<&str>,
    context: PlanningContext<'_>,
    planned: &mut PlannedTurn,
) -> bool {
    let command = content
        .command(command_id)
        .unwrap_or_else(|| panic!("missing command definition '{command_id}'"));
    if command.outcome_mode == CommandOutcomeMode::Dialogue {
        plan_dialogue_command(content, command, input, &context, planned)
    } else if !command.effects.is_empty() {
        plan_command_effects(content, command, input, &context, planned)
    } else {
        plan_content_command(content, command, input, &context, planned)
    }
}
