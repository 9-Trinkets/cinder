use crate::content::types::{CommandInputMode, ContentPack};
use crate::engine::dialogue::{
    ActorTurnActionDecision, ActorTurnActionRequest, ActorTurnCommandInvocation,
};
use crate::engine::hook_ids;
use crate::engine::neuron::evaluate_symbolic_value;
use serde::{Deserialize, Serialize};
use std::error::Error;

use super::decisions::{
    consume_decision_for_item, directly_addressed_target_actor_id, has_clearly_preferred_target,
    preferred_target_actor_id, quiet_room_action_decision, rest_decision,
};

#[derive(Debug, Clone, Serialize)]
pub struct SymbolicPlannerInputCandidate {
    pub actor_id: String,
    pub reply_now: bool,
    pub connection: i32,
    pub safety: i32,
    pub attraction: i32,
    pub target_score: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct SymbolicPlannerInput {
    pub is_directly_addressed: bool,
    pub has_recent_room_speech: bool,
    pub confidence: i32,
    pub stamina: i32,
    pub hunger: i32,
    pub has_rest_affordance: bool,
    pub has_hunger_recovery_consumable: bool,
    pub has_pending_movement_target: bool,
    pub has_move_affordance: bool,
    pub has_speak_room_affordance: bool,
    pub has_clearly_preferred_target: bool,
    pub candidates: Vec<SymbolicPlannerInputCandidate>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SymbolicPlannerBoolResult {
    #[serde(default)]
    pub value: bool,
}

pub fn select_symbolic_actor_turn_action(
    content: &ContentPack,
    request: &ActorTurnActionRequest,
    symbolic_input: &SymbolicPlannerInput,
) -> Result<ActorTurnActionDecision, Box<dyn Error>> {
    let should_rest = content
        .hook(hook_ids::TURN_SHOULD_REST)
        .map(|config| {
            evaluate_symbolic_boolean_rule(config.clone(), serde_json::to_value(symbolic_input)?)
        })
        .transpose()?
        .unwrap_or(false);
    if should_rest && let Some(decision) = rest_decision(request) {
        return Ok(decision);
    }
    let should_consume = content
        .hook(hook_ids::TURN_SHOULD_CONSUME)
        .map(|config| {
            evaluate_symbolic_boolean_rule(config.clone(), serde_json::to_value(symbolic_input)?)
        })
        .transpose()?
        .unwrap_or(false);
    if should_consume
        && let Some(item_id) = request.consume_target_item_id.as_deref()
        && let Some(decision) = consume_decision_for_item(request, item_id)
    {
        return Ok(decision);
    }
    let should_move = content
        .hook(hook_ids::TURN_SHOULD_MOVE)
        .map(|config| {
            evaluate_symbolic_boolean_rule(config.clone(), serde_json::to_value(symbolic_input)?)
        })
        .transpose()?
        .unwrap_or(false);
    if should_move
        && let Some((command_id, room_id)) =
            request
                .affordances
                .iter()
                .find_map(|affordance| match &affordance.invocation {
                    ActorTurnCommandInvocation::Command {
                        command_id,
                        target_room_id: Some(room_id),
                        input_mode: CommandInputMode::None,
                        ..
                    } => Some((command_id.clone(), room_id.clone())),
                    _ => None,
                })
    {
        return Ok(ActorTurnActionDecision::Command {
            command_id,
            target_room_id: Some(room_id),
            target_actor_id: None,
            feature_id: None,
            consumable_id: None,
            context_label: None,
            freeform_text: None,
        });
    }
    let should_speak = evaluate_symbolic_boolean_rule(
        symbolic_rule_config(content, hook_ids::TURN_SHOULD_SPEAK)?,
        serde_json::to_value(symbolic_input)?,
    )?;
    if !should_speak {
        return quiet_room_action_decision(request, "stays quiet for a moment, reading the room.");
    }
    let should_direct_speech = evaluate_symbolic_boolean_rule(
        symbolic_rule_config(content, hook_ids::TURN_SHOULD_DIRECT_SPEECH)?,
        serde_json::to_value(symbolic_input)?,
    )?;
    if should_direct_speech
        && let Some(target_actor_id) = directly_addressed_target_actor_id(request)
            .or_else(|| preferred_target_actor_id(request))
    {
        return Ok(ActorTurnActionDecision::Command {
            command_id: "speak".to_string(),
            target_room_id: None,
            target_actor_id: Some(target_actor_id),
            feature_id: None,
            consumable_id: None,
            context_label: None,
            freeform_text: None,
        });
    }
    if let Some(command_id) =
        request
            .affordances
            .iter()
            .find_map(|affordance| match &affordance.invocation {
                ActorTurnCommandInvocation::Command {
                    command_id,
                    target_actor_id: None,
                    ..
                } if command_id == "speak" => Some(command_id.clone()),
                _ => None,
            })
    {
        return Ok(ActorTurnActionDecision::Command {
            command_id,
            target_room_id: None,
            target_actor_id: None,
            feature_id: None,
            consumable_id: None,
            context_label: None,
            freeform_text: None,
        });
    }
    if let Some(target_actor_id) = preferred_target_actor_id(request) {
        return Ok(ActorTurnActionDecision::Command {
            command_id: "speak".to_string(),
            target_room_id: None,
            target_actor_id: Some(target_actor_id),
            feature_id: None,
            consumable_id: None,
            context_label: None,
            freeform_text: None,
        });
    }
    quiet_room_action_decision(request, "stays quiet for a moment, reading the room.")
}

pub fn evaluate_symbolic_boolean_rule(
    config: serde_json::Value,
    input: serde_json::Value,
) -> Result<bool, Box<dyn Error>> {
    let payload = evaluate_symbolic_value(&config, &input)
        .map_err(|error| -> Box<dyn Error> { Box::new(std::io::Error::other(error)) })?;
    let result: SymbolicPlannerBoolResult = serde_json::from_value(payload)?;
    Ok(result.value)
}

pub fn symbolic_rule_config(
    content: &ContentPack,
    hook_id: &str,
) -> Result<serde_json::Value, Box<dyn Error>> {
    content.hook(hook_id).cloned().ok_or_else(|| {
        Box::new(std::io::Error::other(format!(
            "missing symbolic hook '{hook_id}'"
        ))) as Box<dyn Error>
    })
}

pub fn build_symbolic_action_planner_input(
    request: &ActorTurnActionRequest,
) -> SymbolicPlannerInput {
    let candidates = request
        .speak_candidates
        .iter()
        .map(|candidate| {
            let connection = candidate
                .pair_stats
                .get("connection")
                .copied()
                .unwrap_or_default();
            let safety = candidate
                .pair_stats
                .get("safety")
                .copied()
                .unwrap_or_default();
            let attraction = candidate
                .pair_stats
                .get("attraction")
                .copied()
                .unwrap_or_default();
            SymbolicPlannerInputCandidate {
                actor_id: candidate.actor_id.clone(),
                reply_now: candidate.reply_now,
                connection,
                safety,
                attraction,
                target_score: connection + safety + attraction,
            }
        })
        .collect::<Vec<_>>();
    SymbolicPlannerInput {
        is_directly_addressed: request
            .speak_candidates
            .iter()
            .any(|candidate| candidate.reply_now),
        has_recent_room_speech: request.recent_memory.iter().any(|line| {
            line.kind == crate::engine::state::ConversationMemoryKind::Speech
                && line
                    .target_label
                    .as_deref()
                    .is_some_and(|label| label.eq_ignore_ascii_case("room"))
        }),
        confidence: request
            .actor_stats
            .get("confidence")
            .copied()
            .unwrap_or_default(),
        stamina: request
            .actor_stats
            .get("stamina")
            .copied()
            .unwrap_or_default(),
        hunger: request
            .actor_stats
            .get("hunger")
            .copied()
            .unwrap_or_default(),
        has_rest_affordance: request.has_rest_affordance,
        has_hunger_recovery_consumable: request.has_hunger_recovery_consumable,
        has_pending_movement_target: request.has_pending_movement_target,
        has_move_affordance: request.affordances.iter().any(|affordance| {
            matches!(
                affordance.invocation,
                ActorTurnCommandInvocation::Command {
                    target_room_id: Some(_),
                    ..
                }
            )
        }),
        has_speak_room_affordance: request.affordances.iter().any(|affordance| {
            matches!(
                &affordance.invocation,
                ActorTurnCommandInvocation::Command {
                    command_id,
                    target_actor_id: None,
                    ..
                } if command_id == "speak"
            )
        }),
        has_clearly_preferred_target: has_clearly_preferred_target(request),
        candidates,
    }
}
