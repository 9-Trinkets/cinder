mod dialogue_handler;
mod planner_handler;
mod planning;
mod types;

use self::planner_handler::{build_planned_turn, resolve_next_role};
use self::types::{
    CommandSignal, PlannedTurn, RouteEnvelope, SignalEnvelope, TurnRequest,
    extract_inbound_message, next_turn_id, parse_aggregated_turn,
};

use crate::content::types::ContentPack;
use crate::engine::commands::parse_command;
use crate::engine::conversation_memory::refresh_conversation_summaries;
use crate::engine::dialogue::DialogueGenerator;
use crate::engine::dialogue_grounding::build_grounded_dialogue_request;
use crate::engine::events::{TimestampedWorldEvent, WorldEvent};
use crate::engine::menus::{PendingMenuDialogue, menu_to_offer_for_pending_dialogue};
use crate::engine::neuron::{
    LocalWorkflowRunner, WorkflowDefinition, WorkflowRoleConfig, WorkflowTraceContext, run_workflow,
};
use crate::engine::reducer::{ReducerOutput, apply_events};
use crate::engine::roles::RoleHandler;
use crate::engine::state::{TurnOutcome, WorldSnapshot, WorldState};
use std::error::Error;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub(crate) fn run_turn(
    content: Arc<ContentPack>,
    dialogue: Arc<dyn DialogueGenerator>,
    state: Arc<Mutex<WorldState>>,
    workflow: WorkflowDefinition,
    trace_events: bool,
    trace_dir: &Path,
    raw_input: &str,
) -> Result<TurnOutcome, Box<dyn Error>> {
    let request = TurnRequest {
        raw_input: raw_input.to_string(),
    };
    let runner = CinderRoleRunner {
        content,
        dialogue,
        state,
        tracer: Arc::new(Mutex::new(None)),
        workflow: workflow.clone(),
    };
    let output = run_workflow(
        &workflow,
        &serde_json::to_string(&request)?,
        trace_events,
        trace_dir,
        runner,
    )?;
    Ok(serde_json::from_str(output.trim())?)
}

#[derive(Clone)]
struct CinderRoleRunner {
    content: Arc<ContentPack>,
    dialogue: Arc<dyn DialogueGenerator>,
    state: Arc<Mutex<WorldState>>,
    tracer: Arc<Mutex<Option<WorkflowTraceContext>>>,
    workflow: WorkflowDefinition,
}

impl LocalWorkflowRunner for CinderRoleRunner {
    fn run_role(
        &self,
        role_name: &str,
        prompt: &str,
        role_cfg: &WorkflowRoleConfig,
    ) -> Result<String, String> {
        let inbound = extract_inbound_message(prompt)?;
        let handler = role_cfg
            .handler
            .as_deref()
            .and_then(RoleHandler::parse)
            .ok_or_else(|| format!("role '{role_name}' is missing a valid Cinder handler"))?;
        let route = match handler {
            RoleHandler::Dispatch => Ok(RouteEnvelope {
                next: self.next_non_complete_role(role_name)?,
                message: inbound,
            }),
            RoleHandler::CommandParser => self.handle_command_parser(role_name, &inbound),
            RoleHandler::StateReader => self.handle_state_reader(role_name),
            RoleHandler::Planner => self.handle_turn_planner(role_name, &inbound),
            RoleHandler::MenuIntentClarifier => {
                self.handle_menu_intent_clarifier(role_name, &inbound)
            }
            RoleHandler::DialogueGrounder => self.handle_dialogue_grounder(role_name, &inbound),
            RoleHandler::ActorDialogue => self.handle_actor_dialogue(role_name, &inbound),
            RoleHandler::Reducer => self.handle_turn_reducer(role_name, &inbound),
            RoleHandler::Narrator => self.handle_turn_narrator(&inbound),
            RoleHandler::Aggregation => {
                return Err("turn_merge should be handled by Synapse aggregation".to_string());
            }
        }
        .map_err(|error| format!("role {role_name}: {error}"))?;
        serde_json::to_string(&route).map_err(|error| error.to_string())
    }

    fn set_trace_context(&self, tracer: WorkflowTraceContext) {
        if let Ok(mut slot) = self.tracer.lock() {
            *slot = Some(tracer);
        }
    }
}

impl CinderRoleRunner {
    fn handle_command_parser(
        &self,
        role_name: &str,
        inbound: &str,
    ) -> Result<RouteEnvelope, String> {
        let request: TurnRequest =
            serde_json::from_str(inbound).map_err(|error| error.to_string())?;
        let command = parse_command(self.content.as_ref(), &request.raw_input);
        Ok(RouteEnvelope {
            next: self.next_non_complete_role(role_name)?,
            message: serde_json::to_string(&SignalEnvelope {
                window_id: next_turn_id(&self.state)?.to_string(),
                signal_type: "command".to_string(),
                source: role_name.to_string(),
                payload: serde_json::to_value(CommandSignal {
                    raw_input: request.raw_input,
                    command,
                })
                .map_err(|error| error.to_string())?,
            })
            .map_err(|error| error.to_string())?,
        })
    }

    fn handle_state_reader(&self, role_name: &str) -> Result<RouteEnvelope, String> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock state_reader state".to_string())?;
        let snapshot = WorldSnapshot {
            turn_number: state.turn_number,
            current_room_id: state.current_room_id.clone(),
        };
        Ok(RouteEnvelope {
            next: self.next_non_complete_role(role_name)?,
            message: serde_json::to_string(&SignalEnvelope {
                window_id: (state.turn_number + 1).to_string(),
                signal_type: "world".to_string(),
                source: role_name.to_string(),
                payload: serde_json::to_value(snapshot).map_err(|error| error.to_string())?,
            })
            .map_err(|error| error.to_string())?,
        })
    }

    fn handle_turn_planner(&self, role_name: &str, inbound: &str) -> Result<RouteEnvelope, String> {
        let aggregated = parse_aggregated_turn(inbound)?;
        let planner_state = self
            .state
            .lock()
            .map_err(|_| "failed to lock planner state".to_string())?
            .clone();
        let channel_surfing_only = self.content.settings.channel_surfing_only;
        let turn_number = aggregated.world.turn_number + 1;
        let (planned, _) = build_planned_turn(
            self.content.as_ref(),
            aggregated,
            &planner_state,
            turn_number,
            channel_surfing_only,
        );
        resolve_next_role(
            &planned,
            || self.next_role_from(role_name, "menu_intent_clarifier"),
            || self.next_role_from(role_name, "turn_reducer"),
        )
    }

    fn handle_menu_intent_clarifier(
        &self,
        role_name: &str,
        inbound: &str,
    ) -> Result<RouteEnvelope, String> {
        let mut planned: PlannedTurn =
            serde_json::from_str(inbound).map_err(|error| error.to_string())?;
        let Some(pending) = planned.pending_dialogue.clone() else {
            return Ok(RouteEnvelope {
                next: self.next_non_complete_role(role_name)?,
                message: serde_json::to_string(&planned).map_err(|error| error.to_string())?,
            });
        };

        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock menu intent state".to_string())?
            .clone();
        if let Some(menu) = menu_to_offer_for_pending_dialogue(
            self.content.as_ref(),
            &state,
            self.dialogue.as_ref(),
            PendingMenuDialogue {
                actor_id: &pending.actor_id,
                current_room_id: &pending.current_room_id,
                other_person_message: pending.other_person_message.as_deref(),
            },
            |role, topic, payload| self.emit_trace(role, topic, payload),
        )? {
            planned.events.push(WorldEvent::ActorSpoke {
                actor_id: pending.actor_id.clone(),
                actor_name: self
                    .content
                    .actor(&pending.actor_id)
                    .map(|actor| actor.name.clone())
                    .unwrap_or_else(|| pending.actor_id.clone()),
                other_person_id: pending.other_person_id.clone(),
                other_person_name: pending.other_person_name.clone(),
                other_person_message: pending.other_person_message.clone(),
                room_id: pending.current_room_id.clone(),
                text: menu.proposal_line.clone(),
            });
            planned.events.push(WorldEvent::MenuOpened {
                menu_id: menu.id.clone(),
            });
            planned.pending_dialogue = None;
            return Ok(RouteEnvelope {
                next: self.next_role_from(role_name, "turn_reducer")?,
                message: serde_json::to_string(&planned).map_err(|error| error.to_string())?,
            });
        }

        Ok(RouteEnvelope {
            next: self.next_role_from(role_name, "dialogue_grounder")?,
            message: serde_json::to_string(&planned).map_err(|error| error.to_string())?,
        })
    }

    fn handle_dialogue_grounder(
        &self,
        role_name: &str,
        inbound: &str,
    ) -> Result<RouteEnvelope, String> {
        let mut planned: PlannedTurn =
            serde_json::from_str(inbound).map_err(|error| error.to_string())?;
        let pending = planned
            .pending_dialogue
            .take()
            .ok_or_else(|| "dialogue_grounder missing pending dialogue".to_string())?;
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock dialogue grounding state".to_string())?;
        planned.grounded_dialogue = Some(build_grounded_dialogue_request(
            self.content.as_ref(),
            &state,
            &pending.actor_id,
            &pending.current_room_id,
            pending.other_person_message,
        )?);
        Ok(RouteEnvelope {
            next: self.next_non_complete_role(role_name)?,
            message: serde_json::to_string(&planned).map_err(|error| error.to_string())?,
        })
    }

    fn handle_actor_dialogue(
        &self,
        role_name: &str,
        inbound: &str,
    ) -> Result<RouteEnvelope, String> {
        dialogue_handler::handle_actor_dialogue(
            self.dialogue.as_ref(),
            self.content.as_ref(),
            role_name,
            self.next_non_complete_role(role_name)?,
            inbound,
            |role, topic, payload| self.emit_trace(role, topic, payload),
        )
    }

    fn handle_turn_reducer(&self, role_name: &str, inbound: &str) -> Result<RouteEnvelope, String> {
        let planned: PlannedTurn =
            serde_json::from_str(inbound).map_err(|error| error.to_string())?;
        let logged_events = planned
            .events
            .into_iter()
            .map(TimestampedWorldEvent::now)
            .collect::<Vec<_>>();
        let mut state = self
            .state
            .lock()
            .map_err(|_| "failed to lock reducer state".to_string())?;
        let reduced = apply_events(&mut state, self.content.as_ref(), &logged_events);
        refresh_conversation_summaries(self.content.as_ref(), self.dialogue.as_ref(), &mut state)?;
        Ok(RouteEnvelope {
            next: self.next_non_complete_role(role_name)?,
            message: serde_json::to_string(&reduced).map_err(|error| error.to_string())?,
        })
    }

    fn handle_turn_narrator(&self, inbound: &str) -> Result<RouteEnvelope, String> {
        let reduced: ReducerOutput =
            serde_json::from_str(inbound).map_err(|error| error.to_string())?;
        Ok(RouteEnvelope {
            next: self.workflow.complete_target.clone(),
            message: serde_json::to_string(&TurnOutcome {
                text: reduced.lines.join("\n\n"),
                game_over: reduced.game_over,
            })
            .map_err(|error| error.to_string())?,
        })
    }

    fn next_non_complete_role(&self, role_name: &str) -> Result<String, String> {
        let role = self
            .workflow
            .roles
            .get(role_name)
            .ok_or_else(|| format!("workflow missing role '{role_name}'"))?;
        role.next_roles
            .iter()
            .find(|target| !target.eq_ignore_ascii_case(&self.workflow.complete_target))
            .cloned()
            .ok_or_else(|| format!("role '{role_name}' has no non-complete next role"))
    }

    fn next_role_from(&self, role_name: &str, expected_next: &str) -> Result<String, String> {
        let role = self
            .workflow
            .roles
            .get(role_name)
            .ok_or_else(|| format!("workflow missing role '{role_name}'"))?;
        let has_target = role.next_roles.iter().any(|target| target == expected_next);
        if has_target {
            Ok(expected_next.to_string())
        } else {
            Err(format!(
                "role '{role_name}' is not wired to expected next role '{expected_next}'"
            ))
        }
    }

    fn emit_trace(
        &self,
        role_name: &str,
        topic: &str,
        payload: serde_json::Value,
    ) -> Result<(), String> {
        let tracer = self
            .tracer
            .lock()
            .map_err(|_| "failed to lock cinder trace sink".to_string())?
            .clone();
        let Some(tracer) = tracer else {
            return Ok(());
        };
        tracer.emit(role_name, topic, payload)
    }
}
