use crate::content::types::{ContentPack, OpeningMenuOptionDefinition, OpeningMovieDefinition};
use crate::engine::actor_tick::{ActorTickError, run_actor_tick};
use crate::engine::commands::player_command_help_text;
use crate::engine::conversation_memory::refresh_conversation_summaries;
use crate::engine::dialogue::{DialogueGenerator, SynapseDialogueGenerator};
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
    session_feedback: Arc<Mutex<Option<crate::engine::dialogue::SessionFeedback>>>,
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
            session_feedback: Arc::new(Mutex::new(
                self.session_feedback
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

pub struct FinalChapterSummary {
    pub what_happened: String,
    pub relationship_status: String,
    pub next_chapter_preview: String,
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
            session_feedback: Arc::new(Mutex::new(None)),
        })
    }

    pub fn run_turn(&self, raw_input: &str) -> Result<TurnOutcome, Box<dyn Error>> {
        turn_runner::run_turn(
            Arc::clone(&self.content),
            Arc::clone(&self.dialogue),
            Arc::clone(&self.state),
            self.workflow.clone(),
            self.trace_events,
            &self.trace_dir,
            raw_input,
        )
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
        if !outcome.text.is_empty() {
            self.push_transcript_line(&outcome.text).ok();
        }
        self.continue_after_game_over(outcome)
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

    fn advance_appointment_if_needed(&self) -> Result<Option<String>, Box<dyn Error>> {
        if !self.content.settings.multi_appointment {
            return Ok(None);
        }
        let feedback = self.session_feedback()?;
        let feedback_summary = feedback.as_ref().map(|review| AppointmentFeedbackSummary {
            rating: review.rating,
            review_text: review.review_text.clone(),
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
            session_feedback: Arc::new(Mutex::new(None)),
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

mod menus;
mod session_feedback;
mod stats_trace;
use self::stats_trace::stats_trace_snapshot;
