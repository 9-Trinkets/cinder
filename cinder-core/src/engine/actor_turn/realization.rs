use crate::content::types::{
    ActorDefinition, CommandEffect, CommandInputMode, CommandOutcomeMode, CommandTargetMode,
    ContentPack,
};
use crate::engine::dialogue::{ActorTurnActionDecision, DialogueGenerator};
use crate::engine::events::{WorldEvent, render_actor_action_text};

use super::builder::ActorTurnRealizationContext;
use super::dialogue::{
    RoomSpeakDialogueTarget, actor_action_dialogue, actor_room_speak_dialogue,
    actor_to_actor_dialogue,
};
use super::movement::planned_move_target_room_id;
use super::targeting::{
    resolve_command_consumable_fields, resolve_inspect_actor_name, resolve_target_actor_name,
    validate_command_target_contract, validate_observe_feature_command,
};
use std::error::Error;

pub fn realize_actor_turn_action(
    content: &ContentPack,
    dialogue: &dyn DialogueGenerator,
    state: &crate::engine::state::WorldState,
    actor: &ActorDefinition,
    action: &ActorTurnActionDecision,
    realization_context: &ActorTurnRealizationContext,
    emit_trace: &mut dyn FnMut(&str, &str, serde_json::Value) -> Result<(), String>,
) -> Result<Vec<WorldEvent>, Box<dyn Error>> {
    match action.clone() {
        ActorTurnActionDecision::Move if realization_context.hide_move => {
            Err(Box::new(std::io::Error::other(
                "actor turn decider chose MOVE while hidden exploration actions are unavailable",
            )))
        }
        ActorTurnActionDecision::Move => {
            let Some(target_room_id) =
                planned_move_target_room_id(&realization_context.move_events)
            else {
                return Err(Box::new(std::io::Error::other(
                    "actor turn decider chose MOVE but movement planner returned no destination",
                )));
            };
            Ok(vec![WorldEvent::ActorCommandUsed {
                actor_id: actor.id.clone(),
                actor_name: actor.name.clone(),
                room_id: realization_context.current_room_id.to_string(),
                command_id: "move".to_string(),
                target_room_id: Some(target_room_id.to_string()),
                target_actor_id: None,
                target_actor_name: None,
                context_label: None,
                feature_id: None,
                consumable_id: None,
                freeform_text: None,
            }])
        }
        ActorTurnActionDecision::MoveTo { room_id } => {
            if realization_context.hide_move {
                return Err(Box::new(std::io::Error::other(
                    "actor turn decider chose MOVE while hidden exploration actions are unavailable",
                )));
            }
            let matches_move = realization_context.move_events.iter().any(|event| {
                matches!(
                    event,
                    WorldEvent::ActorMoved { to_room_id, .. } if to_room_id == &room_id
                )
            });
            if matches_move {
                Ok(vec![WorldEvent::ActorCommandUsed {
                    actor_id: actor.id.clone(),
                    actor_name: actor.name.clone(),
                    room_id: realization_context.current_room_id.to_string(),
                    command_id: "move".to_string(),
                    target_room_id: Some(room_id),
                    target_actor_id: None,
                    target_actor_name: None,
                    context_label: None,
                    feature_id: None,
                    consumable_id: None,
                    freeform_text: None,
                }])
            } else {
                Err(Box::new(std::io::Error::other(format!(
                    "actor turn decider chose move target '{room_id}' but movement planner returned a different destination"
                ))))
            }
        }
        ActorTurnActionDecision::Command {
            command_id,
            target_room_id,
            target_actor_id,
            feature_id,
            consumable_id,
            context_label,
            freeform_text,
        } => {
            let command = content.command(&command_id).ok_or_else(|| {
                Box::new(std::io::Error::other(format!(
                    "missing command '{command_id}'"
                ))) as Box<dyn Error>
            })?;
            if command.outcome_mode == CommandOutcomeMode::Dialogue {
                return match target_actor_id.as_deref() {
                    Some(target_actor_id) => actor_to_actor_dialogue(
                        content,
                        dialogue,
                        state,
                        actor,
                        &realization_context.current_room_id,
                        emit_trace,
                        realization_context
                            .talk_targets
                            .iter()
                            .find(|candidate| candidate.actor_id == target_actor_id)
                            .ok_or_else(|| format!("missing talk candidate '{target_actor_id}'"))?,
                    ),
                    None => actor_room_speak_dialogue(
                        content,
                        dialogue,
                        state,
                        actor,
                        &realization_context.current_room_id,
                        emit_trace,
                        RoomSpeakDialogueTarget {
                            audience: &realization_context.talk_targets,
                        },
                    ),
                };
            }
            let freeform_text = match freeform_text {
                Some(text) => {
                    if command.input_mode != CommandInputMode::FreeformText {
                        return Err(Box::new(std::io::Error::other(format!(
                            "actor turn decider returned freeform text for non-freeform command '{command_id}'"
                        ))));
                    }
                    let generated_text = actor_action_dialogue(
                        content,
                        dialogue,
                        state,
                        actor,
                        &realization_context.current_room_id,
                        emit_trace,
                    )
                    .unwrap_or(text);
                    let _ = render_actor_action_text(&actor.name, &generated_text).map_err(
                        |error| -> Box<dyn Error> { Box::new(std::io::Error::other(error)) },
                    )?;
                    Some(generated_text)
                }
                None => None,
            };
            validate_command_target_contract(
                command,
                target_room_id.as_deref(),
                target_actor_id.as_deref(),
                context_label.as_deref(),
                feature_id.as_deref(),
                consumable_id.as_deref(),
            )?;
            if command.has_effect(CommandEffect::MoveActor) {
                if realization_context.hide_move {
                    return Err(Box::new(std::io::Error::other(
                        "actor turn decider chose MOVE while hidden exploration actions are unavailable",
                    )));
                }
                let Some(target_room_id) = target_room_id else {
                    return Err(Box::new(std::io::Error::other(
                        "move command missing target room",
                    )));
                };
                let matches_move = realization_context.move_events.iter().any(|event| {
                    matches!(
                        event,
                        WorldEvent::ActorMoved { to_room_id, .. } if to_room_id == &target_room_id
                    )
                });
                if !matches_move {
                    return Err(Box::new(std::io::Error::other(format!(
                        "actor turn decider chose move target '{target_room_id}' but movement planner returned a different destination"
                    ))));
                }
                return Ok(vec![WorldEvent::ActorCommandUsed {
                    actor_id: actor.id.clone(),
                    actor_name: actor.name.clone(),
                    room_id: realization_context.current_room_id.to_string(),
                    command_id,
                    target_room_id: Some(target_room_id),
                    target_actor_id: None,
                    target_actor_name: None,
                    context_label: None,
                    feature_id: None,
                    consumable_id: None,
                    freeform_text: None,
                }]);
            }
            let target_actor_name = match command.target_mode {
                CommandTargetMode::Actor if command.has_effect(CommandEffect::ObserveActor) => {
                    if realization_context.hide_inspect_actor {
                        return Err(Box::new(std::io::Error::other(
                            "actor turn decider chose INSPECT ACTOR while hidden exploration actions are unavailable",
                        )));
                    }
                    resolve_inspect_actor_name(realization_context, target_actor_id.as_deref())?
                }
                CommandTargetMode::Actor | CommandTargetMode::ActorOptional => {
                    resolve_target_actor_name(realization_context, target_actor_id.as_deref())?
                }
                _ => None,
            };
            if command.target_mode == CommandTargetMode::Feature {
                validate_observe_feature_command(realization_context, feature_id.as_deref())?;
            }
            let (feature_id, consumable_id) =
                if command.target_mode == CommandTargetMode::Consumable {
                    resolve_command_consumable_fields(
                        content,
                        &realization_context.current_room_id,
                        feature_id,
                        consumable_id,
                    )?
                } else {
                    (feature_id, consumable_id)
                };
            Ok(vec![WorldEvent::ActorCommandUsed {
                actor_id: actor.id.clone(),
                actor_name: actor.name.clone(),
                room_id: realization_context.current_room_id.to_string(),
                command_id,
                target_room_id,
                target_actor_id,
                target_actor_name,
                context_label,
                feature_id,
                consumable_id,
                freeform_text,
            }])
        }
        ActorTurnActionDecision::Look
        | ActorTurnActionDecision::Help
        | ActorTurnActionDecision::Quit => Err(Box::new(std::io::Error::other(
            "actor turn decider returned a player-only action",
        ))),
    }
}
