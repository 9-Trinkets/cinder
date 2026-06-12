use crate::content::types::{
    ActorDefinition, ActorMovementRulesDefinition, ActorMovementTargetRuleDefinition, ContentPack,
};
use crate::engine::actor_turn::{
    build_actor_turn, decide_actor_turn_action, realize_actor_turn_action, run_actor_turn,
};
use crate::engine::conversation_memory::refresh_conversation_summaries;
use crate::engine::dialogue::{ActorTurnActionDecision, DialogueGenerator};
use crate::engine::events::{TimestampedWorldEvent, WorldEvent};
use crate::engine::neuron::{
    LocalWorkflowRunner, WorkflowDefinition, WorkflowRoleConfig, evaluate_symbolic_role,
    run_workflow,
};
use crate::engine::reducer::apply_events;
use crate::engine::state::WorldState;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct ActorTickExecution {
    pub events: Vec<WorldEvent>,
    pub trace_records: Vec<ActorTraceRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(tag = "stage", content = "payload", rename_all = "snake_case")]
enum ActorTurnStageEnvelope {
    #[default]
    Idle,
    Built {
        actor_id: String,
    },
    Decided {
        actor_id: String,
        decision: ActorTurnActionDecision,
    },
    Realized {
        actor_id: String,
        events: Vec<WorldEvent>,
    },
}

#[derive(Debug, Clone)]
pub struct ActorTraceRecord {
    pub role_name: String,
    pub topic: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct ActorTickError {
    pub message: String,
    pub trace_records: Vec<ActorTraceRecord>,
}

impl fmt::Display for ActorTickError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for ActorTickError {}

pub(crate) fn run_actor_tick(
    content: Arc<ContentPack>,
    dialogue: Arc<dyn DialogueGenerator>,
    tick_workflow: &WorkflowDefinition,
    movement_workflow: &WorkflowDefinition,
    state: &WorldState,
) -> Result<ActorTickExecution, ActorTickError> {
    let input = ActorTickWorkflowState {
        state: state.clone(),
        remaining_actor_ids: content
            .actors
            .iter()
            .map(|actor| actor.id.clone())
            .collect(),
        current_actor_id: None,
        emitted_events: Vec::new(),
        actor_turn_stage: ActorTurnStageEnvelope::Idle,
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
            movement_workflow: movement_workflow.clone(),
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

pub(crate) fn decide_movement(
    content: Arc<ContentPack>,
    workflow: &WorkflowDefinition,
    state: &WorldState,
    actor: &ActorDefinition,
    rules: &ActorMovementRulesDefinition,
    current_room_id: &str,
    preferred_target_room_id: Option<&str>,
) -> Result<Vec<WorldEvent>, Box<dyn Error>> {
    let input = ActorTurnWorkflowInput {
        actor_id: actor.id.clone(),
        current_room_id: current_room_id.to_string(),
        player_room_id: state.current_room_id.clone(),
        active_stage_ids: state.active_objective_stage_ids.clone(),
        story_vars: state.story_vars.clone(),
        default_target_room_id: preferred_target_room_id
            .unwrap_or(rules.default_target_room_id.as_str())
            .to_string(),
        rules: build_decision_cases(actor, current_room_id, &rules.target_rules),
    };
    let output = run_workflow(
        workflow,
        &serde_json::to_string(&input)?,
        false,
        Path::new("."),
        ActorMoveRoleRunner { content },
    )?;
    let result: ActorTurnResult = serde_json::from_str(output.trim())?;
    Ok(result.events)
}

#[derive(Clone)]
struct ActorMoveRoleRunner {
    content: Arc<ContentPack>,
}

#[derive(Clone)]
struct ActorTickRoleRunner {
    content: Arc<ContentPack>,
    dialogue: Arc<dyn DialogueGenerator>,
    movement_workflow: WorkflowDefinition,
    trace_records: Arc<Mutex<Vec<ActorTraceRecord>>>,
}

impl LocalWorkflowRunner for ActorMoveRoleRunner {
    fn run_role(
        &self,
        role_name: &str,
        prompt: &str,
        _role_cfg: &WorkflowRoleConfig,
    ) -> Result<String, String> {
        match role_name {
            "npc_apply" => self.handle_apply(prompt),
            _ => Err(format!("unknown cinder npc role '{role_name}'")),
        }
    }

    fn run_symbolic_role(
        &self,
        role_name: &str,
        prompt: &str,
        role_cfg: &WorkflowRoleConfig,
    ) -> Result<String, String> {
        match role_name {
            "npc_rule_gate" => self.handle_symbolic_rule(prompt, role_cfg),
            _ => Err(format!("unknown cinder npc symbolic role '{role_name}'")),
        }
    }
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
            "npc_actor_turn_decide_action" => self.handle_actor_turn_decide_action(prompt),
            _ => Err(format!(
                "unknown cinder npc tick symbolic role '{role_name}'"
            )),
        }
    }
}

impl ActorMoveRoleRunner {
    fn handle_apply(&self, prompt: &str) -> Result<String, String> {
        let inbound = extract_inbound_message(prompt)?;
        let decision: ActorMoveDecision =
            serde_json::from_str(&inbound).map_err(|error| error.to_string())?;
        let events = next_room_toward(
            self.content.as_ref(),
            &decision.current_room_id,
            &decision.target_room_id,
        )
        .map(|next_room_id| {
            vec![WorldEvent::ActorMoved {
                actor_id: decision.actor_id,
                from_room_id: decision.current_room_id,
                to_room_id: next_room_id,
            }]
        })
        .unwrap_or_default();
        serde_json::to_string(&RouteEnvelope {
            next: "complete".to_string(),
            message: serde_json::to_string(&ActorTurnResult { events })
                .map_err(|error| error.to_string())?,
        })
        .map_err(|error| error.to_string())
    }

    fn handle_symbolic_rule(
        &self,
        prompt: &str,
        role_cfg: &WorkflowRoleConfig,
    ) -> Result<String, String> {
        evaluate_symbolic_role(prompt, role_cfg, "cinder-npc-rule-gate")
    }
}

impl ActorTickRoleRunner {
    fn handle_tick_orchestrator(&self, prompt: &str) -> Result<String, String> {
        let inbound = extract_inbound_message(prompt)?;
        let mut workflow_state: ActorTickWorkflowState =
            serde_json::from_str(&inbound).map_err(|error| error.to_string())?;
        workflow_state.current_actor_id = None;
        if workflow_state.state.game_over {
            return complete_tick_workflow(&workflow_state.emitted_events);
        }
        while let Some(actor_id) = workflow_state.remaining_actor_ids.first().cloned() {
            workflow_state.remaining_actor_ids.remove(0);
            if self.content.movement_rules(&actor_id).is_none() {
                continue;
            }
            workflow_state.current_actor_id = Some(actor_id);
            workflow_state.actor_turn_stage = ActorTurnStageEnvelope::Idle;
            return route_tick_workflow("npc_actor_turn_build_actions", &workflow_state);
        }
        complete_tick_workflow(&workflow_state.emitted_events)
    }
    fn handle_actor_turn_build_actions(&self, prompt: &str) -> Result<String, String> {
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
        let rules = self
            .content
            .movement_rules(&actor_id)
            .ok_or_else(|| format!("missing movement rules for '{actor_id}'"))?;
        let mut emit_trace = |role_name: &str, topic: &str, payload: serde_json::Value| {
            self.trace_records
                .lock()
                .map_err(|_| "failed to lock npc tick trace records".to_string())?
                .push(ActorTraceRecord {
                    role_name: role_name.to_string(),
                    topic: topic.to_string(),
                    payload,
                });
            Ok(())
        };
        if !self.content.settings.autonomous_actor_dialogue {
            let events = run_actor_turn(
                Arc::clone(&self.content),
                Arc::clone(&self.dialogue),
                &self.movement_workflow,
                &workflow_state.state,
                &actor,
                rules,
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
            return route_tick_workflow("npc_actor_turn_apply", &workflow_state);
        }
        let _ = build_actor_turn(
            Arc::clone(&self.content),
            &self.movement_workflow,
            &workflow_state.state,
            &actor,
            rules,
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
        workflow_state.actor_turn_stage = ActorTurnStageEnvelope::Built { actor_id };
        route_tick_workflow("npc_actor_turn_decide_action", &workflow_state)
    }

    fn handle_actor_turn_decide_action(&self, prompt: &str) -> Result<String, String> {
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
        let rules = self
            .content
            .movement_rules(&actor_id)
            .ok_or_else(|| format!("missing movement rules for '{actor_id}'"))?;
        let build = build_actor_turn(
            Arc::clone(&self.content),
            &self.movement_workflow,
            &workflow_state.state,
            &actor,
            rules,
        )
        .map_err(|error| error.to_string())?;
        let mut emit_trace = |role_name: &str, topic: &str, payload: serde_json::Value| {
            self.trace_records
                .lock()
                .map_err(|_| "failed to lock npc tick trace records".to_string())?
                .push(ActorTraceRecord {
                    role_name: role_name.to_string(),
                    topic: topic.to_string(),
                    payload,
                });
            Ok(())
        };
        let decision =
            decide_actor_turn_action(self.content.as_ref(), &build.request, &mut emit_trace)
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

    fn handle_actor_turn_write_dialogue(&self, prompt: &str) -> Result<String, String> {
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
        let rules = self
            .content
            .movement_rules(&actor_id)
            .ok_or_else(|| format!("missing movement rules for '{actor_id}'"))?;
        let build = build_actor_turn(
            Arc::clone(&self.content),
            &self.movement_workflow,
            &workflow_state.state,
            &actor,
            rules,
        )
        .map_err(|error| error.to_string())?;
        let mut emit_trace = |role_name: &str, topic: &str, payload: serde_json::Value| {
            self.trace_records
                .lock()
                .map_err(|_| "failed to lock npc tick trace records".to_string())?
                .push(ActorTraceRecord {
                    role_name: role_name.to_string(),
                    topic: topic.to_string(),
                    payload,
                });
            Ok(())
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

    fn handle_actor_turn_apply(&self, prompt: &str) -> Result<String, String> {
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
        if !events.is_empty() {
            let timestamped = events
                .iter()
                .cloned()
                .map(TimestampedWorldEvent::now)
                .collect::<Vec<_>>();
            apply_events(
                &mut workflow_state.state,
                self.content.as_ref(),
                &timestamped,
            );
            refresh_conversation_summaries(
                self.content.as_ref(),
                self.dialogue.as_ref(),
                &mut workflow_state.state,
            )
            .map_err(|error| error.to_string())?;
        }
        workflow_state.emitted_events.extend(events);
        workflow_state.current_actor_id = None;
        workflow_state.actor_turn_stage = ActorTurnStageEnvelope::Idle;
        route_tick_workflow("npc_tick_orchestrator", &workflow_state)
    }
}

#[derive(Debug, Clone, Serialize)]
struct ActorTurnWorkflowInput {
    actor_id: String,
    current_room_id: String,
    player_room_id: String,
    active_stage_ids: Vec<String>,
    story_vars: BTreeMap<String, String>,
    default_target_room_id: String,
    rules: Vec<ActorDecisionCase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActorTickWorkflowState {
    state: WorldState,
    remaining_actor_ids: Vec<String>,
    current_actor_id: Option<String>,
    emitted_events: Vec<WorldEvent>,
    #[serde(default)]
    actor_turn_stage: ActorTurnStageEnvelope,
}

#[derive(Debug, Clone, Serialize)]
struct ActorDecisionCase {
    conditions: Vec<ActorDecisionCondition>,
    payload_template: Value,
}

#[derive(Debug, Clone, Serialize)]
struct ActorDecisionCondition {
    path: String,
    operator: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ActorMoveDecision {
    actor_id: String,
    current_room_id: String,
    target_room_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ActorTurnResult {
    #[serde(default)]
    events: Vec<WorldEvent>,
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

fn route_tick_workflow(next: &str, state: &ActorTickWorkflowState) -> Result<String, String> {
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

fn build_decision_cases(
    actor: &ActorDefinition,
    current_room_id: &str,
    target_rules: &[ActorMovementTargetRuleDefinition],
) -> Vec<ActorDecisionCase> {
    target_rules
        .iter()
        .map(|rule| ActorDecisionCase {
            conditions: build_conditions(rule),
            payload_template: json!({
                "actor_id": actor.id,
                "current_room_id": current_room_id,
                "target_room_id": rule.target_room_id,
            }),
        })
        .collect()
}

fn build_conditions(rule: &ActorMovementTargetRuleDefinition) -> Vec<ActorDecisionCondition> {
    let mut conditions = Vec::new();
    if !rule.when_player_room_id.is_empty() {
        conditions.push(ActorDecisionCondition {
            path: "player_room_id".to_string(),
            operator: "equal",
            value: Some(Value::String(rule.when_player_room_id.clone())),
        });
    }
    if !rule.required_story_var.is_empty() {
        conditions.push(ActorDecisionCondition {
            path: format!("story_vars.{}", rule.required_story_var),
            operator: "exists",
            value: None,
        });
    }
    if !rule.any_active_stage_ids.is_empty() {
        conditions.push(ActorDecisionCondition {
            path: "active_stage_ids".to_string(),
            operator: "array_contains_any",
            value: Some(Value::Array(
                rule.any_active_stage_ids
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect(),
            )),
        });
    }
    conditions
}

fn extract_inbound_message(prompt: &str) -> Result<String, String> {
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
        serde_json::from_str(inbound).map_err(|error| error.to_string())
    } else {
        Ok(inbound.to_string())
    }
}

fn next_room_toward(
    content: &ContentPack,
    current_room_id: &str,
    target_room_id: &str,
) -> Option<String> {
    if current_room_id.is_empty() || target_room_id.is_empty() || current_room_id == target_room_id
    {
        return None;
    }
    let mut queue =
        std::collections::VecDeque::from([(current_room_id.to_string(), None::<String>)]);
    let mut visited = std::collections::BTreeSet::from([current_room_id.to_string()]);

    while let Some((room_id, first_step)) = queue.pop_front() {
        let room = content.room(&room_id)?;
        for exit in &room.exits {
            if !visited.insert(exit.room_id.clone()) {
                continue;
            }
            let candidate_first_step = first_step.clone().unwrap_or_else(|| exit.room_id.clone());
            if exit.room_id == target_room_id {
                return Some(candidate_first_step);
            }
            queue.push_back((exit.room_id.clone(), Some(candidate_first_step)));
        }
    }
    None
}
