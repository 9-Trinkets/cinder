use crate::content::types::{ActorDefinition, ActorMovementRulesDefinition, ContentPack};
use crate::engine::actor_tick::decide_movement;
use crate::engine::dialogue::{ActorTurnActionDecision, ActorTurnActionRequest};
use crate::engine::events::WorldEvent;
use crate::engine::state::WorldState;
use std::error::Error;
use std::sync::Arc;

use super::symbolic_planner::{
    build_symbolic_action_planner_input, select_symbolic_actor_turn_action,
};

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

/// Movement for non-autonomous packs (autonomous_actor_dialogue=false), which skip the
/// full affordance/dialogue pipeline in build_actor_turn and only ever move NPCs.
pub fn run_actor_turn(
    content: Arc<ContentPack>,
    state: &WorldState,
    actor: &ActorDefinition,
    rules: &ActorMovementRulesDefinition,
) -> Result<Vec<WorldEvent>, Box<dyn Error>> {
    let current_room_id = state.actor_room_id(&actor.id, &actor.room_id).to_string();
    decide_movement(content, state, actor, rules, &current_room_id, None)
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
