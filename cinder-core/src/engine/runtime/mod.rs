use crate::content::types::{ContentPack, OpeningMenuOptionDefinition};
use crate::engine::dialogue::{DialogueGenerator, SynapseDialogueGenerator};
use crate::engine::narrative::NarrativeLines;
use crate::engine::neuron::{WorkflowDefinition, load_workflow};
use crate::engine::scripted::drain_scripted_sequences;
use crate::engine::state::{
    ActFeedbackSummary, GamePhase, TurnOutcome, WorldState, advance_to_next_act,
    initialize_act_state,
};
use crate::engine::turn_runner;
use crate::engine::workflows::{cinder_npc_tick_workflow_path, workflow_path_for_id};
use serde::Serialize;
use std::error::Error;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct CinderRuntime {
    content: Arc<ContentPack>,
    dialogue: Arc<dyn DialogueGenerator>,
    state: Arc<Mutex<WorldState>>,
    workflow: WorkflowDefinition,
    actor_tick_workflow: WorkflowDefinition,
    trace_events: bool,
    trace_dir: PathBuf,
    act_closure: Arc<Mutex<Option<ActClosure>>>,
    game_closure: Arc<Mutex<Option<ActClosure>>>,
}

impl Clone for CinderRuntime {
    fn clone(&self) -> Self {
        Self {
            content: Arc::clone(&self.content),
            dialogue: Arc::clone(&self.dialogue),
            state: Arc::clone(&self.state),
            workflow: self.workflow.clone(),
            actor_tick_workflow: self.actor_tick_workflow.clone(),
            trace_events: self.trace_events,
            trace_dir: self.trace_dir.clone(),
            act_closure: Arc::new(Mutex::new(
                self.act_closure
                    .lock()
                    .map(|opt| opt.clone())
                    .unwrap_or(None),
            )),
            game_closure: Arc::new(Mutex::new(
                self.game_closure
                    .lock()
                    .map(|opt| opt.clone())
                    .unwrap_or(None),
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PanelOption {
    pub id: String,
    pub title: String,
    pub command: String,
    pub menu_text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActiveMenuInfo {
    pub prompt: String,
    pub options: Vec<OpeningMenuOptionDefinition>,
    #[serde(default)]
    pub max_selections: usize,
    #[serde(default)]
    pub min_selections: usize,
    #[serde(default)]
    pub selected_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActClosure {
    pub title: String,
    pub subtitle: Option<String>,
    pub sections: Vec<ActClosureSection>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ActClosureSection {
    Text { title: String, body: String },
    Rating { title: String, value: u32, max: u32 },
}

impl CinderRuntime {
    pub fn drain_scripted_sequences(&self) -> Result<NarrativeLines, Box<dyn Error>> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for scripted sequences")?;
        Ok(drain_scripted_sequences(self.content.as_ref(), &mut state))
    }

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
        trace_dir: PathBuf,
    ) -> Result<Self, Box<dyn Error>> {
        let mut state = state;
        initialize_act_state(&content, &mut state);
        Ok(Self {
            state: Arc::new(Mutex::new(state)),
            content: Arc::new(content),
            dialogue,
            workflow,
            actor_tick_workflow,
            trace_events,
            trace_dir,
            act_closure: Arc::new(Mutex::new(None)),
            game_closure: Arc::new(Mutex::new(None)),
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
        let outcome = self.apply_stage_assignments(outcome)?;
        let outcome = match outcome.phase {
            GamePhase::ActEnded | GamePhase::GameEnded => {
                let ended_text = &self.content.presentation.presentation_text.act_ended;
                let text = if outcome.text.is_empty() {
                    ended_text.clone()
                } else {
                    format!("{}\n\n{}", outcome.text, ended_text)
                };
                TurnOutcome { text, ..outcome }
            }
            GamePhase::Active => outcome,
        };
        Ok(outcome)
    }

    pub fn continue_after_act(&self) -> Result<(), Box<dyn Error>> {
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| "failed to lock runtime state for act continuation")?;
            if state.phase != GamePhase::ActEnded {
                return Ok(());
            }
            state.phase = GamePhase::Active;
            state
                .story_vars
                .clear_scoped(crate::engine::state::VariableScope::Act);
        }
        self.clear_act_closure_cache()?;
        Ok(())
    }

    pub fn advance_act(&self) -> Result<Option<String>, Box<dyn Error>> {
        if self.content.act_cast.is_empty() {
            return Ok(None);
        }
        let feedback = self.build_perspective_review()?;
        let feedback_summary = feedback.as_ref().map(|review| ActFeedbackSummary {
            rating: review.review.rating,
            review_text: review.review.review_text.clone(),
        });
        let mut state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for act rollover")?;
        if state.phase != GamePhase::ActEnded {
            return Ok(None);
        }
        Ok(advance_to_next_act(
            self.content.as_ref(),
            &mut state,
            feedback_summary.as_ref(),
        ))
    }

    fn clear_act_closure_cache(&self) -> Result<(), Box<dyn Error>> {
        {
            let mut cached = self.act_closure.lock().map_err(|error| error.to_string())?;
            *cached = None;
        }
        {
            let mut cached = self
                .game_closure
                .lock()
                .map_err(|error| error.to_string())?;
            *cached = None;
        }
        Ok(())
    }
}

mod act_closure;
mod actor_ticks;
mod menus;
mod perspective_review;
mod queries;
mod stage_assignment;
#[cfg(test)]
mod stage_assignment_fixtures;
mod stats_trace;

pub use self::act_closure::{FinalChapterSummary, RelationshipPair};
