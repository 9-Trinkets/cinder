use crate::content::types::{
    ActorDefinition, ActorMovementRulesDefinition, AutonomousHostilityMode, ContentPack,
    WanderDefinition, WanderMode,
};
use crate::engine::actor_turn::movement::required_movement_target_room_id;
use crate::engine::actor_turn::{
    build_actor_turn, decide_actor_turn_action, realize_actor_turn_action, run_actor_turn,
};
use crate::engine::behavior::should_hold;
use crate::engine::conversation_memory::refresh_conversation_summaries;
use crate::engine::dialogue::{
    ActorTurnActionDecision, DialogueGenerator, HostilityCandidate, HostilityPlanRequest,
};
use crate::engine::events::{TimestampedWorldEvent, WorldEvent};
use crate::engine::hostility::plan_rules_hostility;
use crate::engine::neuron::{
    LocalWorkflowRunner, WorkflowDefinition, WorkflowRoleConfig, run_workflow,
};
use crate::engine::reducer::apply_events;
use crate::engine::state::ActorStance;
use crate::engine::state::WorldState;
use rand::Rng;
use serde::{Deserialize, Serialize};
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

/// Workflow-local stage for the once-per-tick hostile strike planning pass.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(tag = "stage", content = "payload", rename_all = "snake_case")]
enum HostilityStageEnvelope {
    #[default]
    Idle,
    Decided {
        events: Vec<WorldEvent>,
    },
    Applied,
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
    state: &WorldState,
) -> Result<ActorTickExecution, ActorTickError> {
    // Only tick actors on the player's current board so separate levels don't
    // act in parallel (which also keeps per-tick workflow hops bounded).
    let reachable = content.reachable_room_ids(&state.current_room_id);
    let input = ActorTickWorkflowState {
        state: state.clone(),
        remaining_actor_ids: content
            .actors
            .iter()
            .filter(|actor| {
                !content.is_player_actor(&actor.id)
                    && reachable.contains(state.actor_room_id(&actor.id, &actor.room_id))
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

/// Deterministic wander pass for the tick: hostile, living actors with a
/// non-zero `wander` directive in `movement.json` (per-actor or the pack
/// default) move on the ticks where `current_time_minutes` is a multiple of
/// their `cadence_ticks`. So pawns (cadence 1) drift toward the player fast
/// while knights (2) and stronger pieces lag behind, giving the player early,
/// weaker encounters.
///
/// Whether an actor is *free* to move is decided by `behavior.json`'s `hold`
/// rule (an engaged hostile sharing the player's room holds and strikes rather
/// than wandering off); the destination is chosen from the `wander.mode` in
/// `movement.json`. Nothing here is hardcoded: cadence, destination and
/// hold-eligibility are all declared in content.
pub(crate) fn plan_wander_moves(content: &ContentPack, state: &WorldState) -> Vec<WorldEvent> {
    if state.phase != crate::engine::state::GamePhase::Active {
        return Vec::new();
    }
    // Only wander actors on the player's current board; actors on other levels
    // stay dormant until the player descends.
    let reachable = content.reachable_room_ids(&state.current_room_id);
    let mut events = Vec::new();
    for actor in &content.actors {
        if content.is_player_actor(&actor.id) {
            continue;
        }
        let Some(wander) = resolve_wander(content, &actor.id) else {
            continue;
        };
        if wander.cadence_ticks == 0 {
            continue;
        }
        if state.stance(&actor.id) != ActorStance::Hostile {
            continue;
        }
        if state.actor_stat(&actor.id, &content.settings.combat.health_stat_id) <= 0 {
            continue;
        }
        if !reachable.contains(state.actor_room_id(&actor.id, &actor.room_id)) {
            continue;
        }
        if !state
            .current_time_minutes
            .is_multiple_of(wander.cadence_ticks)
        {
            continue;
        }
        let current_room_id = state.actor_room_id(&actor.id, &actor.room_id);
        // A hostile held by its `behavior.json` `hold` rule (e.g. one sharing
        // the player's room mid-fight) stays put and strikes instead of
        // wandering; it resumes wandering once the player leaves.
        if should_hold(content, state, &actor.id) {
            continue;
        }
        let Some(to_room_id) = wander_destination(content, state, actor, current_room_id, &wander)
        else {
            continue;
        };
        if to_room_id == current_room_id {
            continue;
        }
        events.push(WorldEvent::ActorMoved {
            actor_id: actor.id.clone(),
            from_room_id: current_room_id.to_string(),
            to_room_id,
        });
    }
    events
}

/// Resolve the effective wander directive for an actor: per-actor override, or
/// the pack-wide default from `movement.json`. `None` means the actor has no
/// wander behavior at all.
fn resolve_wander(content: &ContentPack, actor_id: &str) -> Option<WanderDefinition> {
    content
        .movement
        .actors
        .get(actor_id)
        .and_then(|rules| rules.wander.clone())
        .or_else(|| content.movement.defaults.wander.clone())
}

/// Choose the destination room for a wandering actor per its `wander.mode`.
fn wander_destination(
    content: &ContentPack,
    state: &WorldState,
    actor: &ActorDefinition,
    current_room_id: &str,
    wander: &WanderDefinition,
) -> Option<String> {
    match wander.mode {
        WanderMode::RandomAdjacent => {
            let neighbors = content.adjacent_room_ids(current_room_id);
            if neighbors.is_empty() {
                return None;
            }
            let index = rand::thread_rng().gen_range(0..neighbors.len());
            Some(neighbors[index].clone())
        }
        WanderMode::TowardPlayer => {
            next_room_toward(content, current_room_id, &state.current_room_id)
        }
        WanderMode::Stay => None,
        WanderMode::To => {
            // Drift back toward the actor's home room unless a fixed
            // destination is given.
            let destination = if wander.room_id.is_empty() {
                actor.room_id.clone()
            } else {
                wander.room_id.clone()
            };
            next_room_toward(content, current_room_id, &destination)
        }
    }
}

pub(crate) fn decide_movement(
    content: Arc<ContentPack>,
    state: &WorldState,
    actor: &ActorDefinition,
    rules: &ActorMovementRulesDefinition,
    current_room_id: &str,
    preferred_target_room_id: Option<&str>,
) -> Result<Vec<WorldEvent>, Box<dyn Error>> {
    // Engaged: a hostile sharing the player's room is mid-fight and should
    // strike rather than move away. It resumes moving once the player leaves.
    if should_hold(&content, state, &actor.id) {
        return Ok(vec![]);
    }
    let target_room_id = required_movement_target_room_id(state, rules, current_room_id)
        .or_else(|| preferred_target_room_id.map(str::to_string));
    let Some(target_room_id) = target_room_id else {
        return Ok(vec![]);
    };
    if target_room_id == current_room_id {
        return Ok(vec![]);
    }
    Ok(next_room_toward(&content, current_room_id, &target_room_id)
        .map(|next_room_id| {
            vec![WorldEvent::ActorMoved {
                actor_id: actor.id.clone(),
                from_room_id: current_room_id.to_string(),
                to_room_id: next_room_id,
            }]
        })
        .unwrap_or_default())
}

#[derive(Clone)]
struct ActorTickRoleRunner {
    content: Arc<ContentPack>,
    dialogue: Arc<dyn DialogueGenerator>,
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
        if workflow_state.state.phase != crate::engine::state::GamePhase::Active {
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

    fn handle_hostility_decide(&self, prompt: &str) -> Result<String, String> {
        let inbound = extract_inbound_message(prompt)?;
        let mut workflow_state: ActorTickWorkflowState =
            serde_json::from_str(&inbound).map_err(|error| error.to_string())?;
        let eligible_events = plan_rules_hostility(self.content.as_ref(), &workflow_state.state);
        let events = if matches!(
            self.content.settings.autonomous_hostility_mode,
            AutonomousHostilityMode::Llm
        ) && !eligible_events.is_empty()
        {
            let request =
                build_hostility_plan_request(self.content.as_ref(), &workflow_state.state);
            let emit_trace =
                |role_name: &str, topic: &str, payload: serde_json::Value| -> Result<(), String> {
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
            emit_trace(
                "world_hostility",
                "plan.request",
                serde_json::to_value(&request).map_err(|error| error.to_string())?,
            )?;
            let decision = self
                .dialogue
                .plan_hostility_actions(&request)
                .map_err(|error| {
                    let _ = emit_trace(
                        "world_hostility",
                        "workflow.error",
                        serde_json::json!({ "message": error }),
                    );
                    error
                })?;
            emit_trace(
                "world_hostility",
                "plan.decision",
                serde_json::json!({ "strikes": decision.strikes }),
            )?;
            decision
                .strikes
                .into_iter()
                .map(|actor_id| WorldEvent::HostileStrike { actor_id })
                .collect()
        } else {
            eligible_events
        };
        workflow_state.hostility_stage = HostilityStageEnvelope::Decided { events };
        route_tick_workflow("world_hostility_apply", &workflow_state)
    }

    fn handle_hostility_apply(&self, prompt: &str) -> Result<String, String> {
        let inbound = extract_inbound_message(prompt)?;
        let mut workflow_state: ActorTickWorkflowState =
            serde_json::from_str(&inbound).map_err(|error| error.to_string())?;
        let events = match std::mem::replace(
            &mut workflow_state.hostility_stage,
            HostilityStageEnvelope::Idle,
        ) {
            HostilityStageEnvelope::Decided { events } => events,
            _ => {
                return Err(
                    "world_hostility_apply expected decided hostility stage envelope".to_string(),
                );
            }
        };
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
        workflow_state.hostility_stage = HostilityStageEnvelope::Applied;
        route_tick_workflow("npc_tick_orchestrator", &workflow_state)
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
        let rules = self.content.movement_rules(&actor_id);
        let emit_trace =
            |role_name: &str, topic: &str, payload: serde_json::Value| -> Result<(), String> {
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
                &workflow_state.state,
                &actor,
                &rules,
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
        let current_room_id = workflow_state
            .state
            .actor_room_id(&actor.id, &actor.room_id)
            .to_string();
        if required_movement_target_room_id(&workflow_state.state, &rules, &current_room_id)
            .is_some()
        {
            let events = run_actor_turn(
                Arc::clone(&self.content),
                &workflow_state.state,
                &actor,
                &rules,
            )
            .map_err(|error| {
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
            &workflow_state.state,
            &actor,
            &rules,
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
        let rules = self.content.movement_rules(&actor_id);
        let build = build_actor_turn(
            Arc::clone(&self.content),
            &workflow_state.state,
            &actor,
            &rules,
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
        let rules = self.content.movement_rules(&actor_id);
        let build = build_actor_turn(
            Arc::clone(&self.content),
            &workflow_state.state,
            &actor,
            &rules,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActorTickWorkflowState {
    state: WorldState,
    remaining_actor_ids: Vec<String>,
    current_actor_id: Option<String>,
    emitted_events: Vec<WorldEvent>,
    #[serde(default)]
    actor_turn_stage: ActorTurnStageEnvelope,
    #[serde(default)]
    hostility_stage: HostilityStageEnvelope,
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

/// Grounded planner snapshot for the LLM hostility mode. Candidates mirror the
/// rules-policy filter, so both modes choose from the same eligible set.
fn build_hostility_plan_request(content: &ContentPack, state: &WorldState) -> HostilityPlanRequest {
    let current_time_minutes = state.current_time_minutes;
    let candidates = state
        .relationships
        .iter()
        .filter(|(_, relationship)| {
            relationship.stance == crate::engine::state::ActorStance::Hostile
        })
        .filter(|(actor_id, _)| {
            state.actor_stat(actor_id, &content.settings.combat.health_stat_id) > 0
                && state.next_hostile_strike_at.contains_key(*actor_id)
                && {
                    let default_room_id = content
                        .actor(actor_id)
                        .map(|actor| actor.room_id.clone())
                        .unwrap_or_default();
                    state.actor_room_id(actor_id, &default_room_id) == state.current_room_id
                }
        })
        .map(|(actor_id, _)| {
            let actor = content.actor(actor_id);
            let due_at = *state
                .next_hostile_strike_at
                .get(actor_id)
                .unwrap_or(&current_time_minutes);
            let interval = actor
                .map(|actor| {
                    actor.attack_interval_minutes(
                        content.settings.combat.default_attack_interval_minutes,
                    )
                })
                .unwrap_or(content.settings.combat.default_attack_interval_minutes);
            HostilityCandidate {
                actor_id: actor_id.clone(),
                actor_name: actor
                    .map(|actor| actor.name.clone())
                    .unwrap_or_else(|| actor_id.clone()),
                room_id: state
                    .actor_room_id(
                        actor_id,
                        &actor.map(|actor| actor.room_id.clone()).unwrap_or_default(),
                    )
                    .to_string(),
                hp: state.actor_stat(actor_id, &content.settings.combat.health_stat_id),
                strength: state.actor_stat(actor_id, &content.settings.combat.attack_stat_id),
                minutes_since_last_strike: current_time_minutes
                    .saturating_sub(due_at.saturating_sub(interval)),
                attack_interval_minutes: interval,
            }
        })
        .collect();
    HostilityPlanRequest {
        player_room_id: state.current_room_id.to_string(),
        player_hp: state.actor_stat(
            &content.settings.combat.player_actor_id,
            &content.settings.combat.health_stat_id,
        ),
        candidates,
        system_prompt: content.system_text.hostility_planner_system_prompt.clone(),
    }
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

fn sanitize_json_control_chars(input: &str) -> String {
    input.chars().filter(|c| !c.is_control()).collect()
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
        let sanitized = sanitize_json_control_chars(inbound);
        serde_json::from_str(&sanitized).map_err(|error| error.to_string())
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
            let is_target = exit.room_id == target_room_id;
            if (!is_target && !content.room_is_reachable(&exit.room_id))
                || !visited.insert(exit.room_id.clone())
            {
                continue;
            }
            let candidate_first_step = first_step.clone().unwrap_or_else(|| exit.room_id.clone());
            if is_target {
                return Some(candidate_first_step);
            }
            queue.push_back((exit.room_id.clone(), Some(candidate_first_step)));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::test_fixtures::minimal_test_pack;

    #[test]
    fn engaged_hostile_does_not_wander_away_from_player_room() {
        let mut content = minimal_test_pack();
        assert!(content.actors.len() >= 2, "fixture needs two actors");
        content.actors[0].room_id = "lounge".to_string();
        content.actors[1].room_id = "kitchen".to_string();
        let engaged_id = content.actors[0].id.clone();
        let roaming_id = content.actors[1].id.clone();
        // Give both actors a wander cadence in movement.json, and a `behavior`
        // `hold` rule that engages any hostile sharing the player's room.
        content
            .movement
            .actors
            .entry(engaged_id.clone())
            .or_default()
            .wander = Some(crate::content::types::WanderDefinition {
            mode: crate::content::types::WanderMode::RandomAdjacent,
            cadence_ticks: 1,
            room_id: String::new(),
        });
        content
            .movement
            .actors
            .entry(roaming_id.clone())
            .or_default()
            .wander = Some(crate::content::types::WanderDefinition {
            mode: crate::content::types::WanderMode::RandomAdjacent,
            cadence_ticks: 1,
            room_id: String::new(),
        });
        let engaged_in_room_rule = serde_json::json!({
            "rule": "effect_table",
            "rule_config": {
                "cases_path": "rules",
                "next_on_match": "continue",
                "next_on_default": "continue",
                "default_payload_template": { "effects": [] }
            },
            "input_overlay": {
                "rules": [{
                    "conditions": [
                        { "path": "actor.stance", "operator": "equal", "value": "hostile" },
                        { "path": "actor.alive", "operator": "equal", "value": true },
                        { "path": "world.in_player_room", "operator": "equal", "value": true }
                    ],
                    "payload_template": { "kind": "hold" }
                }]
            }
        });
        content.behavior.defaults.hold = Some(engaged_in_room_rule);
        let mut state = WorldState::new(&content);
        state.current_room_id = "lounge".to_string();
        state.set_stance(&engaged_id, ActorStance::Hostile);
        state.set_stance(&roaming_id, ActorStance::Hostile);
        state
            .actor_stats
            .entry(engaged_id.clone())
            .or_default()
            .insert("hp".to_string(), 5);
        state
            .actor_stats
            .entry(roaming_id.clone())
            .or_default()
            .insert("hp".to_string(), 5);

        let events = plan_wander_moves(&content, &state);

        assert!(
            !events.iter().any(|ev| matches!(
                ev,
                WorldEvent::ActorMoved { actor_id, .. } if actor_id == &engaged_id
            )),
            "engaged hostile moved away from the player's room: {events:?}"
        );
        assert!(
            events.iter().any(|ev| matches!(
                ev,
                WorldEvent::ActorMoved { actor_id, .. } if actor_id == &roaming_id
            )),
            "roaming hostile should still wander: {events:?}"
        );
    }
}
