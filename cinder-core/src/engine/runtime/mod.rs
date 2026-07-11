use crate::content::types::{ContentPack, OpeningMenuOptionDefinition, OpeningMovieDefinition};
use crate::engine::actor_tick::{ActorTickError, run_actor_tick};
use crate::engine::commands::player_command_help_text;
use crate::engine::conversation_memory::refresh_conversation_summaries;
use crate::engine::dialogue::{
    DialogueGenerator, StageAssignment, StageAssignmentCandidate, StageAssignmentRequest,
    StageAssignmentScore, SynapseDialogueGenerator,
};
use crate::engine::dialogue_grounding::render_story_text;
use crate::engine::events::{TimestampedWorldEvent, WorldEvent};
use crate::engine::neuron::{WorkflowDefinition, WorkflowTraceContext, load_workflow};
use crate::engine::reducer::apply_events;
use crate::engine::state::{
    AppointmentFeedbackSummary, TurnOutcome, WorldState, advance_to_next_appointment,
    current_appointment_intro, current_patient_name, display_actor_name,
    initialize_appointment_state,
};
use crate::engine::turn_runner;
use crate::engine::workflows::{
    cinder_npc_tick_workflow_path, cinder_npc_turn_workflow_path, workflow_path_for_id,
};
use serde::Serialize;
use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct CinderRuntime {
    content: Arc<ContentPack>,
    dialogue: Arc<dyn DialogueGenerator>,
    state: Arc<Mutex<WorldState>>,
    workflow: WorkflowDefinition,
    actor_tick_workflow: WorkflowDefinition,
    actor_move_workflow: WorkflowDefinition,
    trace_events: bool,
    trace_dir: PathBuf,
    session_closure: Arc<Mutex<Option<SessionClosure>>>,
}

impl Clone for CinderRuntime {
    fn clone(&self) -> Self {
        Self {
            content: Arc::clone(&self.content),
            dialogue: Arc::clone(&self.dialogue),
            state: Arc::clone(&self.state),
            workflow: self.workflow.clone(),
            actor_tick_workflow: self.actor_tick_workflow.clone(),
            actor_move_workflow: self.actor_move_workflow.clone(),
            trace_events: self.trace_events,
            trace_dir: self.trace_dir.clone(),
            session_closure: Arc::new(Mutex::new(
                self.session_closure
                    .lock()
                    .map(|opt| opt.clone())
                    .unwrap_or(None),
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LookOptionItem {
    pub id: String,
    pub label: String,
    pub command: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActiveMenuInfo {
    pub prompt: String,
    pub options: Vec<OpeningMenuOptionDefinition>,
}

#[derive(Debug, Clone)]
pub struct MenuChoiceOption {
    pub prompt: String,
    pub title: String,
    pub menu_text: String,
    pub command: String,
    pub transcript_label: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionClosure {
    pub title: String,
    pub subtitle: Option<String>,
    pub sections: Vec<SessionClosureSection>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SessionClosureSection {
    Text { title: String, body: String },
    Rating { title: String, value: u32, max: u32 },
}

impl CinderRuntime {
    pub fn new(content: ContentPack, trace_events: bool) -> Result<Self, Box<dyn Error>> {
        let workflow_id = if content.settings.workflow_id.is_empty() {
            "cinder_turn".to_string()
        } else {
            content.settings.workflow_id.clone()
        };
        let workflow = load_workflow(&workflow_path_for_id(&workflow_id))?;
        let dialogue = Arc::new(
            SynapseDialogueGenerator::new(workflow.clone())
                .map_err(|error| format!("failed to configure dialogue roles: {error}"))?,
        );
        let state = WorldState::new(&content);
        Self::new_with_dialogue_generator_and_workflows(
            content,
            state,
            trace_events,
            dialogue,
            workflow,
            load_workflow(&cinder_npc_tick_workflow_path())?,
            load_workflow(&cinder_npc_turn_workflow_path())?,
            PathBuf::from(env!("CINDER_PROJECT_DIR")).join(".cinder-state"),
        )
    }

    pub fn from_state(
        content: ContentPack,
        state: WorldState,
        trace_events: bool,
    ) -> Result<Self, Box<dyn Error>> {
        let workflow_id = if content.settings.workflow_id.is_empty() {
            "cinder_turn".to_string()
        } else {
            content.settings.workflow_id.clone()
        };
        let workflow = load_workflow(&workflow_path_for_id(&workflow_id))?;
        let dialogue = Arc::new(
            SynapseDialogueGenerator::new(workflow.clone())
                .map_err(|error| format!("failed to configure dialogue roles: {error}"))?,
        );
        Self::new_with_dialogue_generator_and_workflows(
            content,
            state,
            trace_events,
            dialogue,
            workflow,
            load_workflow(&cinder_npc_tick_workflow_path())?,
            load_workflow(&cinder_npc_turn_workflow_path())?,
            PathBuf::from(env!("CINDER_PROJECT_DIR")).join(".cinder-state"),
        )
    }

    pub fn export_state(&self) -> Result<WorldState, Box<dyn Error>> {
        self.state
            .lock()
            .map_err(|_| "failed to lock runtime state for export".into())
            .map(|state| state.clone())
    }

    fn new_with_dialogue_generator_and_workflows(
        content: ContentPack,
        state: WorldState,
        trace_events: bool,
        dialogue: Arc<dyn DialogueGenerator>,
        workflow: WorkflowDefinition,
        actor_tick_workflow: WorkflowDefinition,
        actor_move_workflow: WorkflowDefinition,
        trace_dir: PathBuf,
    ) -> Result<Self, Box<dyn Error>> {
        let mut state = state;
        initialize_appointment_state(&content, &mut state);
        Ok(Self {
            state: Arc::new(Mutex::new(state)),
            content: Arc::new(content),
            dialogue,
            workflow,
            actor_tick_workflow,
            actor_move_workflow,
            trace_events,
            trace_dir,
            session_closure: Arc::new(Mutex::new(None)),
        })
    }

    pub fn run_turn(&self, raw_input: &str) -> Result<TurnOutcome, Box<dyn Error>> {
        let outcome = turn_runner::run_turn(
            Arc::clone(&self.content),
            Arc::clone(&self.dialogue),
            Arc::clone(&self.state),
            self.workflow.clone(),
            self.trace_events,
            &self.trace_dir,
            raw_input,
        )?;
        self.apply_stage_assignments(outcome)
    }

    pub fn run_tick(&self) -> Result<TurnOutcome, Box<dyn Error>> {
        let outcome = match self.run_actor_turns() {
            Ok((text, game_over)) => TurnOutcome { text, game_over },
            Err(error) => {
                if let Some(actor_tick_error) = error.downcast_ref::<ActorTickError>() {
                    TurnOutcome {
                        text: self.actor_tick_soft_error_text(actor_tick_error),
                        game_over: false,
                    }
                } else {
                    return Err(error);
                }
            }
        };
        let outcome = self.continue_after_game_over(outcome)?;
        let outcome = self.apply_stage_assignments(outcome)?;
        if !outcome.text.is_empty() {
            self.push_transcript_line(&outcome.text).ok();
        }
        Ok(outcome)
    }

    pub fn continue_after_game_over(
        &self,
        outcome: TurnOutcome,
    ) -> Result<TurnOutcome, Box<dyn Error>> {
        if !outcome.game_over {
            return Ok(outcome);
        }
        let Some(intro_text) = self.advance_appointment_if_needed()? else {
            return Ok(outcome);
        };
        let text = if outcome.text.is_empty() {
            intro_text
        } else {
            format!("{}\n\n{}", outcome.text, intro_text)
        };
        Ok(TurnOutcome {
            text,
            game_over: false,
        })
    }

    pub fn current_intro_text(&self) -> Result<String, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for intro text")?;
        Ok(current_appointment_intro(&state)
            .unwrap_or_else(|| self.content.opening.intro_text.clone()))
    }

    pub fn actor_display_name(&self, actor_id: &str) -> Result<Option<String>, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for actor display name")?;
        Ok(self
            .content
            .actor(actor_id)
            .map(|actor| display_actor_name(&state, actor)))
    }

    pub fn current_patient_name(&self) -> Result<Option<String>, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for patient name")?;
        Ok(current_patient_name(&state))
    }

    fn apply_stage_assignments(&self, outcome: TurnOutcome) -> Result<TurnOutcome, Box<dyn Error>> {
        let Some((request, config)) = self.stage_assignment_request()? else {
            return Ok(outcome);
        };
        let assignment = self
            .dialogue
            .assign_stage_participants(&request)
            .unwrap_or_else(|error| {
                eprintln!(
                    "[cinder] stage assignment failed for '{}': {error}, using deterministic fallback",
                    request.stage_id
                );
                self.fallback_stage_assignment(&request)
            });
        let summary = self.commit_stage_assignment(&request, &assignment, &config)?;
        if summary.is_empty() {
            return Ok(outcome);
        }
        let text = if outcome.text.is_empty() {
            summary
        } else {
            format!("{}\n\n{}", outcome.text, summary)
        };
        Ok(TurnOutcome {
            text,
            game_over: outcome.game_over,
        })
    }

    fn stage_assignment_request(
        &self,
    ) -> Result<
        Option<(
            StageAssignmentRequest,
            crate::content::types::StageAssignmentDefinition,
        )>,
        Box<dyn Error>,
    > {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for stage assignment")?;
        for stage_id in &state.active_objective_stage_ids {
            let Some(stage) = self
                .content
                .beats
                .stages
                .iter()
                .find(|stage| &stage.id == stage_id)
            else {
                continue;
            };
            let Some(config) = stage.stage_assignment.as_ref() else {
                continue;
            };
            if config.initiator_actor_id.trim().is_empty()
                || config.selected_room_id.trim().is_empty()
                || config.remaining_room_id.trim().is_empty()
            {
                continue;
            }
            let applied_flag = format!("stage_assignment_applied:{}", stage.id);
            if state
                .story_vars
                .get(&applied_flag)
                .is_some_and(|value| value == "true")
            {
                continue;
            }
            let Some(initiator) = self.content.actor(&config.initiator_actor_id) else {
                continue;
            };
            let selected_room_title = self
                .content
                .room(&config.selected_room_id)
                .map(|room| room.title.clone())
                .unwrap_or_else(|| config.selected_room_id.clone());
            let remaining_room_title = self
                .content
                .room(&config.remaining_room_id)
                .map(|room| room.title.clone())
                .unwrap_or_else(|| config.remaining_room_id.clone());
            let candidates = self
                .content
                .actors
                .iter()
                .filter(|actor| actor.id != initiator.id)
                .map(|actor| StageAssignmentCandidate {
                    actor_id: actor.id.clone(),
                    actor_name: display_actor_name(&state, actor),
                    current_room_id: state.actor_room_id(&actor.id, &actor.room_id).to_string(),
                    current_room_title: self
                        .content
                        .room(state.actor_room_id(&actor.id, &actor.room_id))
                        .map(|room| room.title.clone())
                        .unwrap_or_else(|| {
                            state.actor_room_id(&actor.id, &actor.room_id).to_string()
                        }),
                    actor_stats: state.actor_stats_snapshot(&actor.id),
                    pair_stats_with_initiator: state.pair_stats_snapshot(&actor.id, &initiator.id),
                })
                .collect::<Vec<_>>();
            if candidates.is_empty() {
                continue;
            }
            let request = StageAssignmentRequest {
                locale: self.content.locale.clone(),
                system_text: self.content.system_text.clone(),
                stage_id: stage.id.clone(),
                selection_label: if config.selection_label.trim().is_empty() {
                    stage.summary.clone()
                } else {
                    config.selection_label.clone()
                },
                prompt_instructions: config.prompt_instructions.clone(),
                initiator_actor_id: initiator.id.clone(),
                initiator_actor_name: display_actor_name(&state, initiator),
                selected_room_id: config.selected_room_id.clone(),
                selected_room_title,
                remaining_room_id: config.remaining_room_id.clone(),
                remaining_room_title,
                beat_note: stage.beat_note.clone(),
                candidates,
            };
            return Ok(Some((request, config.clone())));
        }
        Ok(None)
    }

    fn fallback_stage_assignment(&self, request: &StageAssignmentRequest) -> StageAssignment {
        let assignments = request
            .candidates
            .iter()
            .enumerate()
            .map(|(index, candidate)| StageAssignmentScore {
                actor_id: candidate.actor_id.clone(),
                selection_score: if candidate.current_room_id == request.selected_room_id {
                    100
                } else {
                    100 - index as i32
                },
                rationale: "deterministic fallback".to_string(),
            })
            .collect();
        StageAssignment { assignments }
    }

    fn commit_stage_assignment(
        &self,
        request: &StageAssignmentRequest,
        assignment: &StageAssignment,
        config: &crate::content::types::StageAssignmentDefinition,
    ) -> Result<String, Box<dyn Error>> {
        let mut scored = assignment.assignments.clone();
        scored.sort_by(|left, right| {
            right
                .selection_score
                .cmp(&left.selection_score)
                .then_with(|| left.actor_id.cmp(&right.actor_id))
        });
        let selected_count = scored
            .iter()
            .filter(|entry| entry.selection_score >= config.score_threshold)
            .count()
            .max(config.min_selected_actors)
            .min(config.max_selected_actors)
            .min(scored.len());
        let chosen = scored
            .into_iter()
            .take(selected_count)
            .map(|entry| entry.actor_id)
            .collect::<std::collections::BTreeSet<_>>();

        let mut state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state to commit stage assignment")?;
        let applied_flag = format!("stage_assignment_applied:{}", request.stage_id);
        if state
            .story_vars
            .get(&applied_flag)
            .is_some_and(|value| value == "true")
        {
            return Ok(String::new());
        }
        state.actor_room_overrides.insert(
            request.initiator_actor_id.clone(),
            request.selected_room_id.clone(),
        );
        for candidate in &request.candidates {
            let room_id = if chosen.contains(&candidate.actor_id) {
                &request.selected_room_id
            } else {
                &request.remaining_room_id
            };
            state
                .actor_room_overrides
                .insert(candidate.actor_id.clone(), room_id.clone());
        }
        state.story_vars.insert(applied_flag, "true".to_string());

        let selected_names = request
            .candidates
            .iter()
            .filter(|candidate| chosen.contains(&candidate.actor_id))
            .map(|candidate| candidate.actor_name.clone())
            .collect::<Vec<_>>();
        let remaining_names = request
            .candidates
            .iter()
            .filter(|candidate| !chosen.contains(&candidate.actor_id))
            .map(|candidate| candidate.actor_name.clone())
            .collect::<Vec<_>>();
        let mut lines = Vec::new();
        if !config.initiator_line_template.trim().is_empty() {
            lines.push(self.content.render_template(
                &config.initiator_line_template,
                &[
                    ("initiator_name", &request.initiator_actor_name),
                    ("selection_label", &request.selection_label),
                    ("selected_room_title", &request.selected_room_title),
                    ("remaining_room_title", &request.remaining_room_title),
                ],
            ));
        }
        if !selected_names.is_empty() && !config.selected_line_template.trim().is_empty() {
            let selected_names = join_with_and(&selected_names);
            lines.push(self.content.render_template(
                &config.selected_line_template,
                &[
                    ("selected_names", &selected_names),
                    ("selection_label", &request.selection_label),
                    ("selected_room_title", &request.selected_room_title),
                    ("remaining_room_title", &request.remaining_room_title),
                ],
            ));
        }
        if !remaining_names.is_empty() && !config.remaining_line_template.trim().is_empty() {
            let remaining_names = join_with_and(&remaining_names);
            lines.push(self.content.render_template(
                &config.remaining_line_template,
                &[
                    ("remaining_names", &remaining_names),
                    ("selection_label", &request.selection_label),
                    ("selected_room_title", &request.selected_room_title),
                    ("remaining_room_title", &request.remaining_room_title),
                ],
            ));
        }
        Ok(lines.join(" "))
    }

    fn advance_appointment_if_needed(&self) -> Result<Option<String>, Box<dyn Error>> {
        if !self.content.settings.multi_appointment {
            return Ok(None);
        }
        let feedback = self.build_perspective_review()?;
        let feedback_summary = feedback.as_ref().map(|review| AppointmentFeedbackSummary {
            rating: review.review.rating,
            review_text: review.review.review_text.clone(),
        });
        let mut state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for appointment rollover")?;
        if !state.game_over {
            return Ok(None);
        }
        Ok(advance_to_next_appointment(
            self.content.as_ref(),
            &mut state,
            feedback_summary.as_ref(),
        ))
    }

    pub fn current_time_label(&self) -> Result<String, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for current time")?;
        Ok(state.current_time_label())
    }

    pub fn current_day_number(&self) -> Result<u32, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for current day")?;
        Ok(state.current_day_number())
    }

    pub fn current_room_id(&self) -> Result<String, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for room id")?;
        Ok(state.current_room_id.clone())
    }

    pub fn followed_actor_id(&self) -> Result<Option<String>, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for followed actor")?;
        Ok(state.followed_actor_id.clone())
    }

    pub fn player_has_item(&self, item_id: &str) -> Result<bool, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for inventory")?;
        Ok(state.has_item(item_id))
    }

    pub fn inventory_items(&self) -> Result<HashMap<String, u32>, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for inventory")?;
        Ok(state.player_inventory.clone())
    }

    pub fn active_stage_ids(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for stages")?;
        Ok(state.active_objective_stage_ids.clone())
    }

    pub fn push_transcript_line(&self, line: &str) -> Result<(), Box<dyn Error>> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for transcript")?;
        state.transcript.push(line.to_string());
        Ok(())
    }

    pub fn transcript_lines(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for transcript")?;
        Ok(state.transcript.clone())
    }

    pub fn help_text(&self) -> String {
        let available_commands = player_command_help_text(self.content.as_ref());
        self.content.render_template(
            &self.content.opening.help_text,
            &[("available_commands", available_commands.as_str())],
        )
    }

    pub fn current_objective_summaries(&self) -> Result<Vec<(String, String)>, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for objectives")?;
        Ok(state
            .active_objective_stage_ids
            .iter()
            .filter_map(|current_id| {
                self.content
                    .beats
                    .stages
                    .iter()
                    .find(|stage| stage.id == *current_id)
                    .map(|stage| {
                        let summary = render_story_text(&stage.summary, &state);
                        let message = render_story_text(&stage.update_message, &state);
                        (summary, message)
                    })
            })
            .filter(|(summary, _)| !summary.is_empty())
            .collect())
    }

    pub fn current_objective_progress(&self) -> Result<(usize, usize), Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for progress")?;
        let completed = state.stages_completed;
        let total = self.content.beats.stages.len();
        Ok((completed, total))
    }

    pub fn current_secret_progress(&self) -> Result<(usize, usize), Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for secret progress")?;
        let secret_stages: Vec<_> = self
            .content
            .beats
            .stages
            .iter()
            .filter(|s| {
                s.advance_signals
                    .iter()
                    .any(|sig| sig.signal() == "stat_threshold")
            })
            .collect();
        let total = secret_stages.len();
        let found = secret_stages
            .iter()
            .filter(|s| !state.active_objective_stage_ids.contains(&s.id))
            .count();
        Ok((found, total))
    }

    pub fn content(&self) -> &ContentPack {
        self.content.as_ref()
    }

    pub fn with_content(&self, content: ContentPack) -> Self {
        Self {
            content: Arc::new(content),
            dialogue: Arc::clone(&self.dialogue),
            state: Arc::clone(&self.state),
            workflow: self.workflow.clone(),
            actor_tick_workflow: self.actor_tick_workflow.clone(),
            actor_move_workflow: self.actor_move_workflow.clone(),
            trace_events: self.trace_events,
            trace_dir: self.trace_dir.clone(),
            session_closure: Arc::new(Mutex::new(None)),
        }
    }

    pub fn workflow(&self) -> &WorkflowDefinition {
        &self.workflow
    }

    fn actor_tick_soft_error_text(&self, error: &ActorTickError) -> String {
        let actor_name = error
            .trace_records
            .iter()
            .find_map(|trace| {
                (trace.role_name == "npc_actor_turn" && trace.topic == "workflow.error")
                    .then(|| {
                        trace
                            .payload
                            .get("actor_name")
                            .and_then(serde_json::Value::as_str)
                    })
                    .flatten()
            })
            .unwrap_or(&self.content.ui_text.follow_unknown_actor_name);
        self.content.render_template(
            &self.content.ui_text.npc_tick_soft_error,
            &[("actor_name", actor_name)],
        )
    }

    pub fn consume_pending_projector_sequence(
        &self,
    ) -> Result<Option<OpeningMovieDefinition>, Box<dyn Error>> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for projector sequence")?;
        let Some(sequence_id) = state.pending_projector_sequence_id.take() else {
            return Ok(None);
        };
        let found = self
            .content
            .movies
            .iter()
            .find(|movie| movie.id == sequence_id)
            .cloned();
        Ok(found)
    }

    pub fn consume_pending_projector_narrative_lines(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for projector narrative")?;
        Ok(std::mem::take(&mut state.pending_projector_narrative_lines))
    }

    pub fn active_stage_summaries(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for active stages")?;
        Ok(state
            .active_objective_stage_ids
            .iter()
            .filter_map(|id| self.content.beats.stages.iter().find(|s| s.id == *id))
            .map(|s| render_story_text(&s.summary, &state))
            .filter(|s| !s.is_empty())
            .collect())
    }

    fn run_actor_turns(&self) -> Result<(String, bool), Box<dyn Error>> {
        let mut lines = Vec::new();
        let tracer = WorkflowTraceContext::new(self.trace_events, &self.trace_dir)?;
        tracer
            .emit(
                "npc_tick",
                "workflow.start",
                serde_json::json!({
                    "entry_role": "npc_tick",
                    "instruction": "tick",
                    "workflow": "cinder_npc_tick",
                }),
            )
            .map_err(std::io::Error::other)?;
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| "failed to lock runtime state to start npc tick")?;
            if state.game_over {
                return Ok((String::new(), true));
            }
            let tick_start = [TimestampedWorldEvent::now(WorldEvent::TurnStarted {
                turn_number: state.turn_number + 1,
                raw_input: "tick".to_string(),
                advances_time: true,
            })];
            let reduced = apply_events(&mut state, self.content.as_ref(), &tick_start);
            refresh_conversation_summaries(
                self.content.as_ref(),
                self.dialogue.as_ref(),
                &mut state,
            )
            .map_err(std::io::Error::other)?;
            lines.extend(reduced.lines);
        }
        let state_snapshot = {
            let state = self
                .state
                .lock()
                .map_err(|_| "failed to lock runtime state for npc turns")?;
            if state.game_over {
                return Ok((lines.join("\n\n"), true));
            }
            state.clone()
        };
        tracer
            .emit(
                "npc_tick",
                "stats.state",
                serde_json::json!({
                    "phase": "before_tick",
                    "snapshot": stats_trace_snapshot(&state_snapshot),
                }),
            )
            .map_err(std::io::Error::other)?;
        let tick = match run_actor_tick(
            Arc::clone(&self.content),
            Arc::clone(&self.dialogue),
            &self.actor_tick_workflow,
            &self.actor_move_workflow,
            &state_snapshot,
        ) {
            Ok(tick) => tick,
            Err(error) => {
                for trace in &error.trace_records {
                    tracer
                        .emit(&trace.role_name, &trace.topic, trace.payload.clone())
                        .map_err(std::io::Error::other)?;
                }
                tracer
                    .emit(
                        "npc_tick",
                        "workflow.error",
                        serde_json::json!({
                            "workflow": "cinder_npc_tick",
                            "message": error.to_string(),
                            "stats": stats_trace_snapshot(&state_snapshot),
                        }),
                    )
                    .map_err(std::io::Error::other)?;
                return Err(Box::new(error));
            }
        };
        for trace in tick.trace_records {
            tracer
                .emit(&trace.role_name, &trace.topic, trace.payload)
                .map_err(std::io::Error::other)?;
        }
        if !tick.events.is_empty() {
            let mut state = self
                .state
                .lock()
                .map_err(|_| "failed to lock runtime state to apply npc events")?;
            if state.game_over {
                return Ok((lines.join("\n\n"), true));
            }
            let logged_events = tick
                .events
                .into_iter()
                .map(TimestampedWorldEvent::now)
                .collect::<Vec<_>>();
            let reduced = apply_events(&mut state, self.content.as_ref(), &logged_events);
            refresh_conversation_summaries(
                self.content.as_ref(),
                self.dialogue.as_ref(),
                &mut state,
            )
            .map_err(std::io::Error::other)?;
            lines.extend(reduced.lines);
        }
        let final_state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state after npc turns")?
            .clone();
        let game_over = final_state.game_over;
        let final_stats = stats_trace_snapshot(&final_state);
        tracer
            .emit(
                "npc_tick",
                "stats.state",
                serde_json::json!({
                    "phase": "after_tick",
                    "snapshot": final_stats.clone(),
                }),
            )
            .map_err(std::io::Error::other)?;
        tracer
            .emit(
                "npc_tick",
                "workflow.complete",
                serde_json::json!({
                    "game_over": game_over,
                    "text": lines.join("\n\n"),
                    "stats": final_stats,
                }),
            )
            .map_err(std::io::Error::other)?;
        Ok((lines.join("\n\n"), game_over))
    }
}

fn join_with_and(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [only] => only.clone(),
        [first, second] => format!("{first} and {second}"),
        _ => {
            let mut result = items[..items.len() - 1].join(", ");
            result.push_str(", and ");
            result.push_str(&items[items.len() - 1]);
            result
        }
    }
}

mod menus;
mod perspective_review;
mod session_closure;
mod stats_trace;
pub use self::session_closure::FinalChapterSummary;
use self::stats_trace::stats_trace_snapshot;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::loader::load_pack_from_dir;
    use crate::engine::dialogue::ScriptedDialogueGenerator;
    use std::fs;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn stage_assignment_caps_selected_actors_and_marks_stage_complete() {
        let pack_dir = write_stage_assignment_test_pack();
        let content = load_pack_from_dir(&pack_dir).expect("load test pack");
        let state = WorldState::new(&content);
        let dialogue = Arc::new(ScriptedDialogueGenerator::new().with_stage_assignment(
            "dinner-prep",
            StageAssignment {
                assignments: vec![
                    StageAssignmentScore {
                        actor_id: "aera".to_string(),
                        selection_score: 90,
                        rationale: "already leaning toward Ren".to_string(),
                    },
                    StageAssignmentScore {
                        actor_id: "mio".to_string(),
                        selection_score: 82,
                        rationale: "likes the energy in the kitchen".to_string(),
                    },
                    StageAssignmentScore {
                        actor_id: "daichi".to_string(),
                        selection_score: 15,
                        rationale: "hangs back in the lounge".to_string(),
                    },
                ],
            },
        ));
        let runtime = CinderRuntime::new_with_dialogue_generator_and_workflows(
            content,
            state,
            false,
            dialogue,
            load_workflow(&workflow_path_for_id("cinder_turn")).expect("load turn workflow"),
            load_workflow(&cinder_npc_tick_workflow_path()).expect("load npc tick workflow"),
            load_workflow(&cinder_npc_turn_workflow_path()).expect("load npc move workflow"),
            std::env::temp_dir(),
        )
        .expect("build runtime");

        let first = runtime
            .apply_stage_assignments(TurnOutcome {
                text: String::new(),
                game_over: false,
            })
            .expect("apply assignment");
        let exported = runtime.export_state().expect("export state");

        assert!(
            first
                .text
                .contains("Ren heads to the Kitchen to start dinner prep.")
        );
        assert_eq!(
            exported.actor_room_overrides.get("ren").map(String::as_str),
            Some("kitchen")
        );
        assert_eq!(
            exported
                .actor_room_overrides
                .get("aera")
                .map(String::as_str),
            Some("kitchen")
        );
        assert_eq!(
            exported.actor_room_overrides.get("mio").map(String::as_str),
            Some("kitchen")
        );
        assert_eq!(
            exported
                .actor_room_overrides
                .get("daichi")
                .map(String::as_str),
            Some("lounge")
        );
        assert_eq!(
            exported
                .story_vars
                .get("stage_assignment_applied:dinner-prep")
                .map(String::as_str),
            Some("true")
        );

        let second = runtime
            .apply_stage_assignments(TurnOutcome {
                text: String::new(),
                game_over: false,
            })
            .expect("reapply assignment");
        assert!(second.text.is_empty());
    }

    fn write_stage_assignment_test_pack() -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let base = std::env::temp_dir().join(format!("cinder-stage-assignment-{unique}"));
        let locale_dir = base.join("locales").join("en");
        fs::create_dir_all(&locale_dir).expect("create locale dir");
        fs::write(base.join("settings.json"), "{}").expect("write settings");
        fs::write(locale_dir.join("ui.json"), "{}").expect("write ui");
        fs::write(locale_dir.join("system.json"), minimal_system_text_json())
            .expect("write system");
        fs::write(
            locale_dir.join("opening.json"),
            r#"{
  "id": "opening",
  "title": "Test Opening",
  "start_room_id": "lounge",
  "start_time_minutes": 1080,
  "intro_text": "Intro",
  "help_text": "Help"
}"#,
        )
        .expect("write opening");
        fs::write(
            locale_dir.join("rooms.json"),
            r#"[
  {
    "id": "lounge",
    "title": "Lounge",
    "summary": "A shared lounge.",
    "inspect_text": "A shared lounge.",
    "features": [],
    "exits": []
  },
  {
    "id": "kitchen",
    "title": "Kitchen",
    "summary": "A warm kitchen.",
    "inspect_text": "A warm kitchen.",
    "features": [],
    "exits": []
  }
]"#,
        )
        .expect("write rooms");
        fs::write(
            locale_dir.join("actors.json"),
            r#"[
  {
    "id": "ren",
    "name": "Ren",
    "room_id": "lounge",
    "initial_stats": { "confidence": 5, "stamina": 8, "hunger": 3 },
    "prompt_context": {}
  },
  {
    "id": "aera",
    "name": "Aera",
    "room_id": "lounge",
    "initial_stats": { "confidence": 3, "stamina": 6, "hunger": 5 },
    "initial_pair_stats": { "ren": { "connection": 4, "attraction": 2, "safety": 3 } },
    "prompt_context": {}
  },
  {
    "id": "mio",
    "name": "Mio",
    "room_id": "lounge",
    "initial_stats": { "confidence": 7, "stamina": 7, "hunger": 4 },
    "initial_pair_stats": { "ren": { "connection": 2, "attraction": 3, "safety": 1 } },
    "prompt_context": {}
  },
  {
    "id": "daichi",
    "name": "Daichi",
    "room_id": "lounge",
    "initial_stats": { "confidence": 1, "stamina": 9, "hunger": 2 },
    "initial_pair_stats": { "ren": { "connection": 1, "attraction": 0, "safety": 2 } },
    "prompt_context": {}
  }
]"#,
        )
        .expect("write actors");
        fs::write(
            locale_dir.join("beats.json"),
            r#"{
  "initial_stage_ids": ["dinner-prep"],
  "stages": [
    {
      "id": "dinner-prep",
      "summary": "Prep",
      "update_message": "Prep starts.",
      "beat_note": "Split the house.",
      "stage_assignment": {
        "selection_label": "dinner prep",
        "prompt_instructions": "Prefer the kitchen when someone would want to be near Ren through useful, practical closeness.",
        "initiator_actor_id": "ren",
        "selected_room_id": "kitchen",
        "remaining_room_id": "lounge",
        "max_selected_actors": 2,
        "min_selected_actors": 1,
        "score_threshold": 50,
        "initiator_line_template": "{initiator_name} heads to the {selected_room_title} to start {selection_label}.",
        "selected_line_template": "{selected_names} join in the {selected_room_title}.",
        "remaining_line_template": "{remaining_names} stay in the {remaining_room_title}."
      },
      "next_stage_ids": ["dinner"]
    },
    {
      "id": "dinner",
      "summary": "Dinner",
      "update_message": "Dinner starts."
    }
  ]
}"#,
        )
        .expect("write beats");
        fs::write(
            base.join("stats.json"),
            r#"{
  "actor": {
    "confidence": { "default": 0 },
    "stamina": { "default": 0 },
    "hunger": { "default": 0 }
  },
  "pair": {
    "connection": { "default": 0 },
    "attraction": { "default": 0 },
    "safety": { "default": 0 }
  }
}"#,
        )
        .expect("write stats");
        base
    }

    fn minimal_system_text_json() -> &'static str {
        r#"{
  "dialogue_system_prompt": "",
  "dialogue_section_character": "",
  "dialogue_section_setting": "",
  "dialogue_section_current_beat": "",
  "dialogue_section_subtext": "",
  "dialogue_section_recent_memory": "",
  "dialogue_latest_line_label": "",
  "dialogue_section_response": "",
  "dialogue_no_direct_question": "",
  "dialogue_no_character_facts": "",
  "dialogue_no_setting_facts": "",
  "dialogue_no_current_beat_facts": "",
  "dialogue_no_subtext_facts": "",
  "dialogue_no_recent_memory": "",
  "dialogue_response_fallback": "",
  "menu_intent_system_prompt": "",
  "menu_section_title": "",
  "menu_id_label": "",
  "menu_offered_by_label": "",
  "menu_intent_guidance_label": "",
  "menu_available_options_label": "",
  "menu_section_setting": "",
  "menu_section_current_beat": "",
  "menu_section_recent_memory": "",
  "menu_latest_line_label": "",
  "menu_decision_label": "",
  "menu_no_direct_request": "",
  "menu_no_authored_options": "",
  "menu_decision_instruction": "",
  "prompt_time_note": "",
  "prompt_current_room_note": "",
  "prompt_visible_features_note": "",
  "prompt_people_here_note": "",
  "prompt_exits_note": "",
  "prompt_current_speaker_note": "",
  "prompt_shared_room_note": "",
  "prompt_latest_words_note": "",
  "prompt_address_other_person_note": ""
}"#
    }
}
