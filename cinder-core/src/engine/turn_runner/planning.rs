use super::types::{PendingDialogue, PlannedTurn};
use crate::content::types::{
    CommandDefinition, CommandEffect, CommandOutcomeMode, ContentPack, PlayerCommandTargetMode,
};
use crate::engine::commands::{resolve_actor_reference_input, unknown_target_token};
use crate::engine::dialogue_grounding::viewer_participant_id;
use crate::engine::events::{ObservationMode, WorldEvent};
use crate::engine::state::WorldState;
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
    raw_input: &str,
    planned: &mut PlannedTurn,
) -> bool {
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
                        ("raw_input", raw_input),
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
    planned
        .events
        .push(content_event_for_command(command, payload));
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
    if let Some(actor) = content.resolve_actor(target) {
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
                    &[("actor_name", actor.name.as_str())],
                ),
            });
        }
    } else if let Some(feature) = content.resolve_feature_in_room(context.current_room_id, target) {
        planned.events.push(WorldEvent::FeatureObserved {
            room_id: context.current_room_id.to_string(),
            feature_id: feature.id.clone(),
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
        plan_content_command(content, command, input, context.raw_input, planned)
    }
}
