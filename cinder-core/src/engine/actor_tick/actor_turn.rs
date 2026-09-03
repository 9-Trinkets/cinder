use super::ActorTurnStageEnvelope;
use super::workflow::{
    ActorTickRoleRunner, ActorTickWorkflowState, extract_inbound_message, route_tick_workflow,
};
use crate::engine::actor_turn::movement::required_movement_target_room_id;
use crate::engine::actor_turn::{
    build_actor_turn, decide_actor_turn_action, realize_actor_turn_action, run_actor_turn,
};

impl ActorTickRoleRunner {
    pub(super) fn handle_actor_turn_build_actions(&self, prompt: &str) -> Result<String, String> {
        let inbound = extract_inbound_message(prompt)?;
        let mut workflow_state: ActorTickWorkflowState =
            serde_json::from_str(&inbound).map_err(|error| error.to_string())?;
        let actor_id = workflow_state.current_actor_id.clone().ok_or_else(|| {
            "npc_actor_turn_build_actions is missing current_actor_id".to_string()
        })?;
        let actor = self
            .content
            .actor(&actor_id)
            .cloned()
            .ok_or_else(|| format!("missing actor '{actor_id}'"))?;
        let rules = self.content.movement_rules(&actor_id);
        if !self.content.settings.autonomous_actor_dialogue {
            let events =
                run_actor_turn(self.content.clone(), &workflow_state.state, &actor, &rules)
                    .map_err(|error| {
                        let current_room_id = workflow_state
                            .state
                            .actor_room_id(&actor.id, &actor.room_id);
                        let _ = self.emit_trace(
                            "npc_actor_turn",
                            "workflow.error",
                            serde_json::json!({
                                "actor_id": actor.id,
                                "actor_name": actor.name,
                                "current_room_id": current_room_id,
                                "message": error.to_string(),
                            }),
                        );
                        error.to_string()
                    })?;
            workflow_state.actor_turn_stage = ActorTurnStageEnvelope::Realized { actor_id, events };
            return route_tick_workflow("npc_actor_turn_apply", &workflow_state);
        }
        let current_room_id = workflow_state
            .state
            .actor_room_id(&actor.id, &actor.room_id)
            .to_string();
        if required_movement_target_room_id(&workflow_state.state, &rules, &current_room_id)
            .is_some()
        {
            let events =
                run_actor_turn(self.content.clone(), &workflow_state.state, &actor, &rules)
                    .map_err(|error| {
                        let _ = self.emit_trace(
                            "npc_actor_turn",
                            "workflow.error",
                            serde_json::json!({
                                "actor_id": actor.id,
                                "actor_name": actor.name,
                                "current_room_id": current_room_id,
                                "message": error.to_string(),
                            }),
                        );
                        error.to_string()
                    })?;
            workflow_state.actor_turn_stage = ActorTurnStageEnvelope::Realized { actor_id, events };
            return route_tick_workflow("npc_actor_turn_apply", &workflow_state);
        }
        let _ = build_actor_turn(self.content.clone(), &workflow_state.state, &actor, &rules)
            .map_err(|error| {
                let current_room_id = workflow_state
                    .state
                    .actor_room_id(&actor.id, &actor.room_id);
                let _ = self.emit_trace(
                    "npc_actor_turn",
                    "workflow.error",
                    serde_json::json!({
                        "actor_id": actor.id,
                        "actor_name": actor.name,
                        "current_room_id": current_room_id,
                        "message": error.to_string(),
                    }),
                );
                error.to_string()
            })?;
        workflow_state.actor_turn_stage = ActorTurnStageEnvelope::Built { actor_id };
        route_tick_workflow("npc_actor_turn_decide_action", &workflow_state)
    }

    pub(super) fn handle_actor_turn_decide_action(&self, prompt: &str) -> Result<String, String> {
        let inbound = extract_inbound_message(prompt)?;
        let mut workflow_state: ActorTickWorkflowState =
            serde_json::from_str(&inbound).map_err(|error| error.to_string())?;
        let actor_id = workflow_state.current_actor_id.clone().ok_or_else(|| {
            "npc_actor_turn_decide_action is missing current_actor_id".to_string()
        })?;
        let stage_actor_id = match std::mem::replace(
            &mut workflow_state.actor_turn_stage,
            ActorTurnStageEnvelope::Idle,
        ) {
            ActorTurnStageEnvelope::Built { actor_id } => actor_id,
            _ => {
                return Err(
                    "npc_actor_turn_decide_action expected built actor turn stage envelope"
                        .to_string(),
                );
            }
        };
        if stage_actor_id != actor_id {
            return Err(format!(
                "npc_actor_turn_decide_action stage actor mismatch '{stage_actor_id}' != '{actor_id}'"
            ));
        }
        let actor = self
            .content
            .actor(&actor_id)
            .cloned()
            .ok_or_else(|| format!("missing actor '{actor_id}'"))?;
        let rules = self.content.movement_rules(&actor_id);
        let build = build_actor_turn(self.content.clone(), &workflow_state.state, &actor, &rules)
            .map_err(|error| error.to_string())?;
        let mut emit_trace = |role_name: &str, topic: &str, payload: serde_json::Value| {
            self.emit_trace(role_name, topic, payload)
        };
        let decision =
            decide_actor_turn_action(self.dialogue.as_ref(), &build.request, &mut emit_trace)
                .map_err(|error| {
                    let _ = emit_trace(
                        "npc_actor_turn",
                        "workflow.error",
                        serde_json::json!({
                            "actor_id": actor_id,
                            "actor_name": build.request.actor_name,
                            "message": error.to_string(),
                        }),
                    );
                    error.to_string()
                })?;
        workflow_state.actor_turn_stage = ActorTurnStageEnvelope::Decided { actor_id, decision };
        route_tick_workflow("npc_actor_turn_write_dialogue", &workflow_state)
    }

    pub(super) fn handle_actor_turn_write_dialogue(&self, prompt: &str) -> Result<String, String> {
        let inbound = extract_inbound_message(prompt)?;
        let mut workflow_state: ActorTickWorkflowState =
            serde_json::from_str(&inbound).map_err(|error| error.to_string())?;
        let actor_id = workflow_state.current_actor_id.clone().ok_or_else(|| {
            "npc_actor_turn_write_dialogue is missing current_actor_id".to_string()
        })?;
        let (stage_actor_id, decision) = match std::mem::replace(
            &mut workflow_state.actor_turn_stage,
            ActorTurnStageEnvelope::Idle,
        ) {
            ActorTurnStageEnvelope::Decided { actor_id, decision } => (actor_id, decision),
            _ => {
                return Err(
                    "npc_actor_turn_write_dialogue expected decided actor turn stage envelope"
                        .to_string(),
                );
            }
        };
        if stage_actor_id != actor_id {
            return Err(format!(
                "npc_actor_turn_write_dialogue stage actor mismatch '{stage_actor_id}' != '{actor_id}'"
            ));
        }
        let actor = self
            .content
            .actor(&actor_id)
            .cloned()
            .ok_or_else(|| format!("missing actor '{actor_id}'"))?;
        let rules = self.content.movement_rules(&actor_id);
        let build = build_actor_turn(self.content.clone(), &workflow_state.state, &actor, &rules)
            .map_err(|error| error.to_string())?;
        let mut emit_trace = |role_name: &str, topic: &str, payload: serde_json::Value| {
            self.emit_trace(role_name, topic, payload)
        };
        let events = realize_actor_turn_action(
            self.content.as_ref(),
            self.dialogue.as_ref(),
            &workflow_state.state,
            &actor,
            &decision,
            &build.realization_context,
            &mut emit_trace,
        )
        .map_err(|error| {
            let current_room_id = workflow_state
                .state
                .actor_room_id(&actor.id, &actor.room_id);
            let _ = emit_trace(
                "npc_actor_turn",
                "workflow.error",
                serde_json::json!({
                    "actor_id": actor.id,
                    "actor_name": actor.name,
                    "current_room_id": current_room_id,
                    "message": error.to_string(),
                }),
            );
            error.to_string()
        })?;
        workflow_state.actor_turn_stage = ActorTurnStageEnvelope::Realized { actor_id, events };
        route_tick_workflow("npc_actor_turn_apply", &workflow_state)
    }

    pub(super) fn handle_actor_turn_apply(&self, prompt: &str) -> Result<String, String> {
        let inbound = extract_inbound_message(prompt)?;
        let mut workflow_state: ActorTickWorkflowState =
            serde_json::from_str(&inbound).map_err(|error| error.to_string())?;
        let actor_id = workflow_state
            .current_actor_id
            .clone()
            .ok_or_else(|| "npc_actor_turn_apply is missing current_actor_id".to_string())?;
        let (stage_actor_id, events) = match std::mem::replace(
            &mut workflow_state.actor_turn_stage,
            ActorTurnStageEnvelope::Idle,
        ) {
            ActorTurnStageEnvelope::Realized { actor_id, events } => (actor_id, events),
            _ => {
                return Err(
                    "npc_actor_turn_apply expected realized actor turn stage envelope".to_string(),
                );
            }
        };
        if stage_actor_id != actor_id {
            return Err(format!(
                "npc_actor_turn_apply stage actor mismatch '{stage_actor_id}' != '{actor_id}'"
            ));
        }
        self.apply_and_refresh(&mut workflow_state.state, &events)?;
        workflow_state.emitted_events.extend(events);
        workflow_state.current_actor_id = None;
        workflow_state.actor_turn_stage = ActorTurnStageEnvelope::Idle;
        route_tick_workflow("npc_tick_orchestrator", &workflow_state)
    }
}
