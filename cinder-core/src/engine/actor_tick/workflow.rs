use super::{
    ActorTickError, ActorTickExecution, ActorTraceRecord, ActorTurnStageEnvelope,
    HostilityStageEnvelope, room_is_in_tick_scope, tick_scope_room_ids,
};
use crate::content::types::ContentPack;
use crate::engine::conversation_memory::refresh_conversation_summaries;
use crate::engine::dialogue::DialogueGenerator;
use crate::engine::events::{TimestampedWorldEvent, WorldEvent};
use crate::engine::neuron::{
    LocalWorkflowRunner, WorkflowDefinition, WorkflowRoleConfig, run_workflow,
};
use crate::engine::reducer::apply_events;
use crate::engine::state::{GamePhase, WorldState};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, Mutex};

pub(crate) fn run_actor_tick(
    content: Arc<ContentPack>,
    dialogue: Arc<dyn DialogueGenerator>,
    tick_workflow: &WorkflowDefinition,
    state: &WorldState,
) -> Result<ActorTickExecution, ActorTickError> {
    let scope_room_ids = tick_scope_room_ids(content.as_ref(), state);
    let input = ActorTickWorkflowState {
        state: state.clone(),
        remaining_actor_ids: content
            .onstage_actors()
            .filter(|actor| {
                !content.is_player_actor(&actor.id)
                    && room_is_in_tick_scope(
                        &scope_room_ids,
                        state.actor_room_id(&actor.id, &actor.room_id),
                    )
            })
            .map(|actor| actor.id.clone())
            .collect(),
        current_actor_id: None,
        emitted_events: Vec::new(),
        actor_turn_stage: ActorTurnStageEnvelope::Idle,
        hostility_stage: HostilityStageEnvelope::Idle,
    };
    let trace_records = Arc::new(Mutex::new(Vec::new()));
    let output = run_workflow(
        tick_workflow,
        &serde_json::to_string(&input).map_err(|error| ActorTickError {
            message: error.to_string(),
            trace_records: Vec::new(),
        })?,
        false,
        Path::new("."),
        ActorTickRoleRunner {
            content,
            dialogue,
            trace_records: Arc::clone(&trace_records),
        },
    );
    let trace_records = Arc::try_unwrap(trace_records)
        .map_err(|_| ActorTickError {
            message: "failed to unwrap npc tick trace records".to_string(),
            trace_records: Vec::new(),
        })?
        .into_inner()
        .map_err(|_| ActorTickError {
            message: "failed to unlock npc tick trace records".to_string(),
            trace_records: Vec::new(),
        })?;
    let output = output.map_err(|error| ActorTickError {
        message: error.to_string(),
        trace_records: trace_records.clone(),
    })?;
    let result: ActorTickResult =
        serde_json::from_str(output.trim()).map_err(|error| ActorTickError {
            message: error.to_string(),
            trace_records: trace_records.clone(),
        })?;
    Ok(ActorTickExecution {
        events: result.events,
        trace_records,
    })
}

#[derive(Clone)]
pub(super) struct ActorTickRoleRunner {
    pub(super) content: Arc<ContentPack>,
    pub(super) dialogue: Arc<dyn DialogueGenerator>,
    trace_records: Arc<Mutex<Vec<ActorTraceRecord>>>,
}

impl LocalWorkflowRunner for ActorTickRoleRunner {
    fn run_role(
        &self,
        role_name: &str,
        prompt: &str,
        _role_cfg: &WorkflowRoleConfig,
    ) -> Result<String, String> {
        match role_name {
            "npc_tick_orchestrator" => self.handle_tick_orchestrator(prompt),
            "world_hostility_decide" => self.handle_hostility_decide(prompt),
            "world_hostility_apply" => self.handle_hostility_apply(prompt),
            "npc_actor_turn_build_actions" => self.handle_actor_turn_build_actions(prompt),
            "npc_actor_turn_decide_action" => self.handle_actor_turn_decide_action(prompt),
            "npc_actor_turn_write_dialogue" => self.handle_actor_turn_write_dialogue(prompt),
            "npc_actor_turn_apply" => self.handle_actor_turn_apply(prompt),
            _ => Err(format!("unknown cinder npc tick role '{role_name}'")),
        }
    }

    fn run_symbolic_role(
        &self,
        role_name: &str,
        prompt: &str,
        _role_cfg: &WorkflowRoleConfig,
    ) -> Result<String, String> {
        match role_name {
            "world_hostility_decide" => self.handle_hostility_decide(prompt),
            "npc_actor_turn_decide_action" => self.handle_actor_turn_decide_action(prompt),
            _ => Err(format!(
                "unknown cinder npc tick symbolic role '{role_name}'"
            )),
        }
    }
}

impl ActorTickRoleRunner {
    fn handle_tick_orchestrator(&self, prompt: &str) -> Result<String, String> {
        let inbound = extract_inbound_message(prompt)?;
        let mut workflow_state: ActorTickWorkflowState =
            serde_json::from_str(&inbound).map_err(|error| error.to_string())?;
        workflow_state.current_actor_id = None;
        if workflow_state.state.phase != GamePhase::Active {
            return complete_tick_workflow(&workflow_state.emitted_events);
        }
        match workflow_state.hostility_stage {
            HostilityStageEnvelope::Idle => {
                route_tick_workflow("world_hostility_decide", &workflow_state)
            }
            HostilityStageEnvelope::Applied => {
                if let Some(actor_id) = workflow_state.remaining_actor_ids.first().cloned() {
                    workflow_state.remaining_actor_ids.remove(0);
                    workflow_state.current_actor_id = Some(actor_id);
                    workflow_state.actor_turn_stage = ActorTurnStageEnvelope::Idle;
                    return route_tick_workflow("npc_actor_turn_build_actions", &workflow_state);
                }
                complete_tick_workflow(&workflow_state.emitted_events)
            }
            HostilityStageEnvelope::Decided { .. } => {
                Err("npc tick orchestrator received undecided hostility stage envelope".to_string())
            }
        }
    }

    pub(super) fn emit_trace(
        &self,
        role_name: &str,
        topic: &str,
        payload: serde_json::Value,
    ) -> Result<(), String> {
        self.trace_records
            .lock()
            .map_err(|_| "failed to lock npc tick trace records".to_string())?
            .push(ActorTraceRecord {
                role_name: role_name.to_string(),
                topic: topic.to_string(),
                payload,
            });
        Ok(())
    }

    pub(super) fn apply_and_refresh(
        &self,
        state: &mut WorldState,
        events: &[WorldEvent],
    ) -> Result<(), String> {
        if events.is_empty() {
            return Ok(());
        }
        let timestamped = events
            .iter()
            .cloned()
            .map(TimestampedWorldEvent::now)
            .collect::<Vec<_>>();
        apply_events(state, self.content.as_ref(), &timestamped);
        refresh_conversation_summaries(self.content.as_ref(), self.dialogue.as_ref(), state)
            .map_err(|error| error.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ActorTickWorkflowState {
    pub(super) state: WorldState,
    pub(super) remaining_actor_ids: Vec<String>,
    pub(super) current_actor_id: Option<String>,
    pub(super) emitted_events: Vec<WorldEvent>,
    #[serde(default)]
    pub(super) actor_turn_stage: ActorTurnStageEnvelope,
    #[serde(default)]
    pub(super) hostility_stage: HostilityStageEnvelope,
}

#[derive(Debug, Serialize, Deserialize)]
struct ActorTickResult {
    #[serde(default)]
    events: Vec<WorldEvent>,
}

#[derive(Debug, Serialize)]
struct RouteEnvelope {
    next: String,
    message: String,
}

pub(super) fn route_tick_workflow(
    next: &str,
    state: &ActorTickWorkflowState,
) -> Result<String, String> {
    serde_json::to_string(&RouteEnvelope {
        next: next.to_string(),
        message: serde_json::to_string(state).map_err(|error| error.to_string())?,
    })
    .map_err(|error| error.to_string())
}

fn complete_tick_workflow(events: &[WorldEvent]) -> Result<String, String> {
    serde_json::to_string(&RouteEnvelope {
        next: "complete".to_string(),
        message: serde_json::to_string(&ActorTickResult {
            events: events.to_vec(),
        })
        .map_err(|error| error.to_string())?,
    })
    .map_err(|error| error.to_string())
}

pub(super) fn extract_inbound_message(prompt: &str) -> Result<String, String> {
    let (marker, json_encoded) = if prompt.contains("INBOUND_MESSAGE_JSON:\n") {
        ("INBOUND_MESSAGE_JSON:\n", true)
    } else {
        ("INBOUND_MESSAGE:\n", false)
    };
    let start = prompt
        .find(marker)
        .ok_or_else(|| "missing INBOUND_MESSAGE block".to_string())?
        + marker.len();
    let rest = &prompt[start..];
    let end = rest
        .find("\n\nROUTING_PROTOCOL:")
        .ok_or_else(|| "missing ROUTING_PROTOCOL block".to_string())?;
    let inbound = &rest[..end];
    if json_encoded {
        let sanitized: String = inbound.chars().filter(|c| !c.is_control()).collect();
        serde_json::from_str(&sanitized).map_err(|error| error.to_string())
    } else {
        Ok(inbound.to_string())
    }
}
