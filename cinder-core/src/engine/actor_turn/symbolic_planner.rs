use crate::content::types::CommandInputMode;
use crate::engine::dialogue::{
    ActorTurnActionDecision, ActorTurnActionRequest, ActorTurnCommandInvocation, DialogueGenerator,
};
use crate::engine::neuron::evaluate_symbolic_value;
use rand::Rng;
use serde::Deserialize;
use std::error::Error;

use super::decisions::quiet_room_action_decision;

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

pub fn select_fallback_actor_turn_action(
    request: &ActorTurnActionRequest,
) -> Result<ActorTurnActionDecision, Box<dyn Error>> {
    for affordance in &request.affordances {
        let ActorTurnCommandInvocation::Command { input_mode, .. } = &affordance.invocation;
        if *input_mode == CommandInputMode::None
            && let Ok(decision) = affordance.invocation.clone().into_decision(None)
        {
            return Ok(decision);
        }
    }
    quiet_room_action_decision(request, "stays quiet for a moment, reading the room.")
}

pub fn decide_actor_turn_action(
    dialogue: &dyn DialogueGenerator,
    request: &ActorTurnActionRequest,
    emit_trace: &mut dyn FnMut(&str, &str, serde_json::Value) -> Result<(), String>,
) -> Result<ActorTurnActionDecision, Box<dyn Error>> {
    let prompt = dialogue.build_actor_turn_action_prompt(request);
    let model_backend = dialogue.trace_metadata("actor_turn_decider");
    let trace_backend = serde_json::json!({
        "backend": "symbolic_fallback",
        "planner_mode": "affordance_first",
    });
    emit_trace(
        "actor_turn_decider",
        "model.request",
        serde_json::json!({
            "actor_id": request.actor_id,
            "actor_name": request.actor_name,
            "dialogue_request": request.clone(),
            "prompt": prompt,
            "backend": model_backend,
        }),
    )
    .map_err(|error| -> Box<dyn Error> { Box::new(std::io::Error::other(error)) })?;
    match dialogue.choose_actor_turn_action(request) {
        Ok(model_action) => {
            emit_trace(
                "actor_turn_decider",
                "model.response",
                serde_json::json!({
                    "actor_id": request.actor_id,
                    "actor_name": request.actor_name,
                    "decision": actor_turn_decision_label(&model_action),
                    "backend": dialogue.trace_metadata("actor_turn_decider"),
                }),
            )
            .map_err(|error| -> Box<dyn Error> { Box::new(std::io::Error::other(error)) })?;
            Ok(model_action)
        }
        Err(error) => {
            let fallback_action = select_fallback_actor_turn_action(request)?;
            emit_trace(
                "actor_turn_decider",
                "model.response",
                serde_json::json!({
                    "actor_id": request.actor_id,
                    "actor_name": request.actor_name,
                    "error": error,
                    "decision": actor_turn_decision_label(&fallback_action),
                    "backend": trace_backend,
                }),
            )
            .map_err(|error| -> Box<dyn Error> { Box::new(std::io::Error::other(error)) })?;
            Ok(fallback_action)
        }
    }
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
