use crate::content::types::{CommandInputMode, ContentPack};
use crate::engine::dialogue::{
    ActorTurnActionDecision, ActorTurnActionRequest, ActorTurnCommandInvocation,
};
use crate::engine::hook_ids;
use crate::engine::neuron::evaluate_symbolic_value;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::error::Error;

use super::decisions::{
    consume_decision_for_item, cook_decision, has_clearly_preferred_target,
    quiet_room_action_decision, rest_decision,
};
use super::policies::speech::{SpeechPolicyInput, decide_speech_action};

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
    pub has_food_consumable: bool,
    pub has_cook_affordance: bool,
    pub cooking_needed: bool,
    pub food_stock: usize,
    pub food_stock_deficit: i32,
    pub has_speak_room_affordance: bool,
    pub has_clearly_preferred_target: bool,
    pub candidates: Vec<SymbolicPlannerInputCandidate>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SymbolicPlannerBoolResult {
    #[serde(default)]
    pub value: bool,
    #[serde(default)]
    pub chance: Option<SymbolicPlannerChanceGate>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SymbolicPlannerChanceGate {
    pub path: String,
    #[serde(default = "default_chance_max")]
    pub max: u32,
}

fn default_chance_max() -> u32 {
    10
}

fn evaluate_speech_gate(
    config: Option<&serde_json::Value>,
    symbolic_input: &SymbolicPlannerInput,
    default_value: bool,
) -> Result<bool, Box<dyn Error>> {
    if let Some(config) = config {
        return evaluate_symbolic_boolean_rule(config.clone(), serde_json::to_value(symbolic_input)?);
    }
    Ok(default_value)
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
    let should_cook = content
        .hook(hook_ids::TURN_SHOULD_COOK)
        .map(|config| {
            evaluate_symbolic_boolean_rule(config.clone(), serde_json::to_value(symbolic_input)?)
        })
        .transpose()?
        .unwrap_or(false);
    if should_cook && let Some(decision) = cook_decision(request) {
        return Ok(decision);
    }
    if let Some((command_id, room_id)) =
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
    let should_speak = evaluate_speech_gate(
        content.speech.should_speak_when.as_ref(),
        symbolic_input,
        true,
    )?;
    let should_direct_speech = evaluate_speech_gate(
        content.speech.should_direct_speech_when.as_ref(),
        symbolic_input,
        true,
    )?;
    if let Some(decision) = decide_speech_action(
        request,
        SpeechPolicyInput {
            should_speak,
            should_direct_speech,
        },
    ) {
        return Ok(decision);
    }
    quiet_room_action_decision(request, "stays quiet for a moment, reading the room.")
}

pub fn decide_actor_turn_action(
    content: &ContentPack,
    request: &ActorTurnActionRequest,
    emit_trace: &mut dyn FnMut(&str, &str, serde_json::Value) -> Result<(), String>,
) -> Result<ActorTurnActionDecision, Box<dyn Error>> {
    let symbolic_input = build_symbolic_action_planner_input(request);
    let trace_backend = serde_json::json!({
        "backend": "symbolic",
        "planner_mode": "symbolic",
        "rule": "decision_table",
    });
    emit_trace(
        "actor_turn_decider",
        "model.request",
        serde_json::json!({
            "actor_id": request.actor_id,
            "actor_name": request.actor_name,
            "dialogue_request": request.clone(),
            "symbolic_input": symbolic_input,
            "backend": trace_backend.clone(),
        }),
    )
    .map_err(|error| -> Box<dyn Error> { Box::new(std::io::Error::other(error)) })?;
    let symbolic_action = select_symbolic_actor_turn_action(content, request, &symbolic_input)?;
    emit_trace(
        "actor_turn_decider",
        "model.response",
        serde_json::json!({
            "actor_id": request.actor_id,
            "actor_name": request.actor_name,
            "decision": actor_turn_decision_label(&symbolic_action),
            "backend": trace_backend,
        }),
    )
    .map_err(|error| -> Box<dyn Error> { Box::new(std::io::Error::other(error)) })?;
    Ok(symbolic_action)
}

pub fn actor_turn_decision_label(decision: &ActorTurnActionDecision) -> String {
    match decision {
        ActorTurnActionDecision::Move => "MOVE".to_string(),
        ActorTurnActionDecision::MoveTo { room_id } => format!("MOVE {room_id}"),
        ActorTurnActionDecision::Command {
            command_id,
            target_room_id,
            target_actor_id,
            feature_id,
            consumable_id,
            freeform_text,
            ..
        } => {
            let label = command_id.replace('_', " ").to_ascii_uppercase();
            if let Some(target_actor_id) = target_actor_id {
                return format!("{label} {target_actor_id}");
            }
            if let Some(target_room_id) = target_room_id {
                return format!("{label} {target_room_id}");
            }
            if let Some(consumable_id) = consumable_id {
                return format!("{label} {consumable_id}");
            }
            if let Some(feature_id) = feature_id {
                return format!("{label} {feature_id}");
            }
            match freeform_text {
                Some(text) => format!("{label} {text}"),
                None => label,
            }
        }
        ActorTurnActionDecision::Look => "LOOK".to_string(),
        ActorTurnActionDecision::Help => "HELP".to_string(),
        ActorTurnActionDecision::Quit => "QUIT".to_string(),
    }
}

pub fn evaluate_symbolic_boolean_rule(
    config: serde_json::Value,
    input: serde_json::Value,
) -> Result<bool, Box<dyn Error>> {
    let payload = evaluate_symbolic_value(&config, &input)
        .map_err(|error| -> Box<dyn Error> { Box::new(std::io::Error::other(error)) })?;
    let result: SymbolicPlannerBoolResult = serde_json::from_value(payload)?;
    if !result.value {
        return Ok(false);
    }
    if let Some(chance) = result.chance {
        let stat_value = input
            .get(&chance.path)
            .and_then(|value| value.as_i64())
            .unwrap_or_default();
        let roll = rand::thread_rng().gen_range(0..=chance.max);
        if stat_value < i64::from(roll) {
            return Ok(false);
        }
    }
    Ok(true)
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
        has_food_consumable: request.has_food_consumable,
        has_cook_affordance: request.has_cook_affordance,
        cooking_needed: request.cooking_needed,
        food_stock: request.food_stock,
        food_stock_deficit: request.actor_count as i32 - request.food_stock as i32,
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
