use crate::content::types::{
    ContentPack, OpeningMenuOptionDefinition, OpeningMovieDefinition, RoomDefinition,
};
use crate::engine::actor_tick::{ActorTickError, run_actor_tick};
use crate::engine::commands::player_command_help_text;
use crate::engine::conversation_memory::refresh_conversation_summaries;
use crate::engine::dialogue::{
    ChapterRelationshipSummaryRequest, ChapterScriptSummaryRequest, DialogueGenerator,
    SynapseChapterSummaryGenerator, SynapseDialogueGenerator, YelpReview, YelpReviewRequest,
};
use crate::engine::dialogue_grounding::render_story_text;
use crate::engine::events::{ObservationMode, TimestampedWorldEvent, WorldEvent};
use crate::engine::menus::render_menu_prompt;
use crate::engine::neuron::{WorkflowDefinition, WorkflowTraceContext, load_workflow};
use crate::engine::reducer::apply_events;
use crate::engine::state::{TurnOutcome, WorldState};
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
    yelp_review: Mutex<Option<crate::engine::dialogue::YelpReview>>,
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
            yelp_review: Mutex::new(
                self.yelp_review
                    .lock()
                    .map(|opt| opt.clone())
                    .unwrap_or(None),
            ),
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
        Ok(Self {
            state: Arc::new(Mutex::new(state)),
            content: Arc::new(content),
            dialogue,
            workflow,
            actor_tick_workflow,
            actor_move_workflow,
            trace_events,
            trace_dir,
            yelp_review: Mutex::new(None),
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
        Ok(outcome)
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

    pub fn current_room_look_options(&self) -> Result<Vec<LookOptionItem>, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for look options")?;
        let current_room_id = &state.current_room_id;
        let Some(room) = self.content.room(current_room_id) else {
            return Ok(Vec::new());
        };
        let mut options = Vec::new();
        options.push(LookOptionItem {
            id: "__room__".to_string(),
            label: room.title.clone(),
            command: "look".to_string(),
        });
        for feature in &room.features {
            let alias = feature
                .aliases
                .first()
                .map(|a| a.as_str())
                .unwrap_or(&feature.label);
            options.push(LookOptionItem {
                id: format!("feature:{}", feature.id),
                label: feature.label.clone(),
                command: format!("x {}", alias),
            });
        }
        for actor in &self.content.actors {
            let actor_room = state.actor_room_id(&actor.id, &actor.room_id);
            if actor_room == current_room_id {
                options.push(LookOptionItem {
                    id: format!("actor:{}", actor.id),
                    label: actor.name.clone(),
                    command: format!("look at {}", actor.name),
                });
            }
        }
        for item in &self.content.items {
            if state.has_item(&item.id) {
                options.push(LookOptionItem {
                    id: format!("item:{}", item.id),
                    label: item.label.clone(),
                    command: format!("look at {}", item.label),
                });
            }
        }
        Ok(options)
    }

    pub fn current_room_talk_options(&self) -> Result<Vec<LookOptionItem>, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for talk options")?;
        let current_room_id = &state.current_room_id;
        let mut options = Vec::new();
        for actor in &self.content.actors {
            let actor_room = state.actor_room_id(&actor.id, &actor.room_id);
            if actor_room == current_room_id {
                options.push(LookOptionItem {
                    id: format!("actor:{}", actor.id),
                    label: actor.name.clone(),
                    command: format!("talk to {}", actor.name),
                });
            }
        }
        Ok(options)
    }

    pub fn current_active_menu_info(&self) -> Result<Option<ActiveMenuInfo>, Box<dyn Error>> {
        let menu_id = {
            let state = self
                .state
                .lock()
                .map_err(|_| "failed to lock runtime state for active menu")?;
            state.active_menu_id.clone()
        };
        let Some(ref menu_id) = menu_id else {
            return Ok(None);
        };
        let menu = self.content.menu(menu_id);
        let Some(menu) = menu else {
            return Ok(None);
        };
        let prompt = menu.selection_prompt.clone();
        if menu.dynamic {
            let needs_generation = {
                let state = self
                    .state
                    .lock()
                    .map_err(|_| "failed to lock runtime state for dynamic menu")?;
                !state.generated_menu_options.contains_key(menu_id.as_str())
            };
            if needs_generation {
                self.menu_choice_options()?;
            }
            let state = self
                .state
                .lock()
                .map_err(|_| "failed to lock runtime state after dynamic menu gen")?;
            let options = state
                .generated_menu_options
                .get(menu_id.as_str())
                .cloned()
                .unwrap_or_default();
            return Ok(Some(ActiveMenuInfo { prompt, options }));
        }
        Ok(Some(ActiveMenuInfo {
            prompt,
            options: menu.options.clone(),
        }))
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

    pub fn relationship_status_lines(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for relationship summary")?;
        let mut lines = self
            .content
            .actors
            .iter()
            .enumerate()
            .flat_map(|(index, actor)| {
                self.content
                    .actors
                    .iter()
                    .skip(index + 1)
                    .filter_map(|other| {
                        let stats = state.pair_stats_snapshot(&actor.id, &other.id);
                        if stats.is_empty() {
                            return None;
                        }
                        let mut score = 0i32;
                        let rendered_stats = stats
                            .into_iter()
                            .filter_map(|(stat_key, value)| {
                                let default = state
                                    .pair_stat_defs
                                    .get(&stat_key)
                                    .map(|definition| definition.default)
                                    .unwrap_or(0);
                                score += (value - default).abs();
                                (value != default).then(|| format!("{stat_key} {value}"))
                            })
                            .collect::<Vec<_>>();
                        if rendered_stats.is_empty() {
                            return None;
                        }
                        Some((
                            score,
                            format!(
                                "{} / {}: {}",
                                actor.name,
                                other.name,
                                rendered_stats.join(", ")
                            ),
                        ))
                    })
            })
            .collect::<Vec<_>>();
        lines.sort_by(|left, right| right.cmp(left));
        Ok(lines.into_iter().map(|(_, line)| line).collect())
    }

    pub fn current_next_chapter_preview(&self) -> Result<Option<String>, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for next chapter preview")?;
        Ok(state
            .active_objective_stage_ids
            .iter()
            .filter_map(|stage_id| {
                self.content
                    .beats
                    .stages
                    .iter()
                    .find(|stage| stage.id == *stage_id)
            })
            .find_map(|stage| {
                let preview = render_story_text(&stage.next_chapter_preview, &state);
                (!preview.is_empty()).then_some(preview)
            }))
    }

    pub fn final_chapter_summary(
        &self,
        transcript: &[String],
        chapter_start_index: usize,
    ) -> Result<FinalChapterSummary, Box<dyn Error>> {
        let transcript_lines = Self::chapter_transcript_lines(transcript, chapter_start_index);
        let relationship_lines = self.relationship_status_lines()?;
        let preview = self
            .current_next_chapter_preview()?
            .unwrap_or_else(|| self.content.ui_text.final_summary_empty_preview.clone());

        let summary_generator = SynapseChapterSummaryGenerator::new(self.workflow.clone())
            .map_err(|error| format!("failed to configure chapter summary roles: {error}"))?;

        let what_happened = if transcript_lines.is_empty() {
            self.content.ui_text.day_summary_empty_highlights.clone()
        } else {
            summary_generator
                .summarize_script(&ChapterScriptSummaryRequest {
                    locale: self.content.locale.clone(),
                    system_text: self.content.system_text.clone(),
                    transcript_lines,
                })
                .map_err(std::io::Error::other)?
        };
        let relationship_status = if relationship_lines.is_empty() {
            self.content.ui_text.day_summary_empty_relationships.clone()
        } else {
            summary_generator
                .summarize_relationships(&ChapterRelationshipSummaryRequest {
                    locale: self.content.locale.clone(),
                    system_text: self.content.system_text.clone(),
                    pair_stat_lines: relationship_lines,
                })
                .map_err(std::io::Error::other)?
        };

        Ok(FinalChapterSummary {
            what_happened,
            relationship_status,
            next_chapter_preview: preview,
        })
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
            yelp_review: Mutex::new(None),
        }
    }

    pub fn relocalize_story_vars(&self) -> Result<(), Box<dyn Error>> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for story-var relocalization")?;
        for menu in &self.content.menus {
            if menu.selection_var_key.is_empty() || menu.selection_id_var_key.is_empty() {
                continue;
            }
            let Some(selected_id) = state.story_vars.get(&menu.selection_id_var_key).cloned()
            else {
                continue;
            };
            let Some(option) = menu.options.iter().find(|option| option.id == selected_id) else {
                continue;
            };
            state
                .story_vars
                .insert(menu.selection_var_key.clone(), option.title.clone());
            state
                .story_vars
                .insert("selection_title".to_string(), option.title.clone());
        }
        Ok(())
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
        eprintln!("[debug] consume_pending_projector_sequence: pending_id={:?}, story_vars_keys={:?}, active_stages={:?}, actor_overrides={:?}",
            state.pending_projector_sequence_id,
            state.story_vars.keys().collect::<Vec<_>>(),
            state.active_objective_stage_ids,
            state.actor_room_overrides);
        let Some(sequence_id) = state.pending_projector_sequence_id.take() else {
            return Ok(None);
        };
        let found = self
            .content
            .movies
            .iter()
            .find(|movie| movie.id == sequence_id)
            .cloned();
        eprintln!("[debug] consume_pending_projector_sequence: looked up id={:?}, found={}",
            sequence_id, found.is_some());
        Ok(found)
    }

    pub fn consume_pending_projector_narrative_lines(
        &self,
    ) -> Result<Vec<String>, Box<dyn Error>> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for projector narrative")?;
        Ok(std::mem::take(&mut state.pending_projector_narrative_lines))
    }

    fn chapter_transcript_lines(transcript: &[String], chapter_start_index: usize) -> Vec<String> {
        transcript
            .iter()
            .skip(chapter_start_index)
            .map(|line| line.trim())
            .filter(|line| !line.is_empty() && !line.starts_with('>'))
            .map(ToString::to_string)
            .collect()
    }

    pub fn menu_choice_options(
        &self,
    ) -> Result<Option<Vec<MenuChoiceOption>>, Box<dyn Error>> {
        let menu_id = {
            let state = self
                .state
                .lock()
                .map_err(|_| "failed to lock runtime state for menu")?;
            state.active_menu_id.clone()
        };
        let Some(ref menu_id) = menu_id else {
            return Ok(None);
        };
        let menu = self.content.menu(menu_id);
        let Some(menu) = menu else {
            return Ok(None);
        };
        if menu.dynamic {
            // Check for already-generated options before acquiring write lock
            {
                let state = self
                    .state
                    .lock()
                    .map_err(|_| "failed to lock runtime state for dynamic menu")?;
                if let Some(options) = state.generated_menu_options.get(menu_id) {
                    return Ok(Some(
                        options
                            .iter()
                            .enumerate()
                            .map(|(index, option)| MenuChoiceOption {
                                prompt: render_menu_prompt(self.content.as_ref(), menu),
                                title: option.title.clone(),
                                menu_text: option.menu_text.clone(),
                                command: (index + 1).to_string(),
                                transcript_label: None,
                            })
                            .collect(),
                    ));
                }
            }
            // No generated options yet — acquire write lock and check again (double-checked)
            let needs_generation = {
                let state = self
                    .state
                    .lock()
                    .map_err(|_| "failed to lock runtime state for dynamic menu")?;
                !state.generated_menu_options.contains_key(menu_id)
            };
            if needs_generation {
                let actor_name = self
                    .content
                    .actors
                    .iter()
                    .find(|a| a.id == menu.actor_id)
                    .map(|a| a.name.clone())
                    .unwrap_or_default();
                let character_bio = self
                    .content
                    .actors
                    .iter()
                    .filter(|a| a.id == menu.actor_id)
                    .flat_map(|a| Some(a.prompt_context.character_notes.join("\n")))
                    .next()
                    .unwrap_or_default();
                let recent_memory = {
                    let state = self
                        .state
                        .lock()
                        .map_err(|_| "failed to lock runtime state for dynamic menu")?;
                    let conversation_key = format!("{}:{}", menu.actor_id, "isla");
                    state
                        .conversation_memory
                        .get(&conversation_key)
                        .map(|lines| lines.iter().rev().take(10).cloned().collect::<Vec<_>>())
                        .unwrap_or_default()
                };
                let role_name = if menu.generation_role.is_empty() {
                    "dynamic_menu"
                } else {
                    &menu.generation_role
                };
                let request = crate::engine::dialogue::DynamicMenuRequest {
                    locale: self.content.locale.clone(),
                    system_text: self.content.system_text.clone(),
                    role_name: role_name.to_string(),
                    actor_name,
                    character_bio,
                    current_beat_notes: menu.narrative_lines.clone(),
                    recent_memory,
                };
                let result = self.dialogue.generate_dynamic_menu_options(&request);
                let mut state = self
                    .state
                    .lock()
                    .map_err(|_| "failed to lock runtime state after dynamic menu generation")?;
                if let Ok(options) = result {
                    let options: Vec<OpeningMenuOptionDefinition> = options
                        .into_iter()
                        .map(|opt| OpeningMenuOptionDefinition {
                            id: opt.id,
                            title: opt.title,
                            menu_text: opt.menu_text,
                        })
                        .collect();
                    state
                        .generated_menu_options
                        .insert(menu_id.clone(), options.clone());
                    return Ok(Some(
                        options
                            .into_iter()
                            .enumerate()
                            .map(|(index, option)| MenuChoiceOption {
                                prompt: render_menu_prompt(self.content.as_ref(), menu),
                                title: option.title,
                                menu_text: option.menu_text,
                                command: (index + 1).to_string(),
                                transcript_label: None,
                            })
                            .collect(),
                    ));
                }
                // LLM failed, fall through to static options
            }
        }
        let _state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for menu")?;
        Ok(Some(
            menu.options
                .iter()
                .enumerate()
                .map(|(index, option)| MenuChoiceOption {
                    prompt: render_menu_prompt(self.content.as_ref(), menu),
                    title: option.title.clone(),
                    menu_text: option.menu_text.clone(),
                    command: (index + 1).to_string(),
                    transcript_label: None,
                })
                .collect(),
        ))
    }

    pub fn room_switch_options(&self) -> Result<Vec<MenuChoiceOption>, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for room switching")?;
        let Some(current_room) = self.content.room(&state.current_room_id) else {
            return Ok(Vec::new());
        };
        let prompt = self
            .content
            .ui_text
            .room_switch_prompt
            .replace("{}", &current_room.title);

        let exit_ids: Vec<String> = current_room.exits.iter().map(|e| e.room_id.clone()).collect();
        let rooms_iter: Box<dyn Iterator<Item = &RoomDefinition>> =
            if self.content.settings.channel_surfing_only {
                Box::new(self.content.rooms.iter())
            } else {
                Box::new(
                    self.content
                        .rooms
                        .iter()
                        .filter(move |r| exit_ids.contains(&r.id)),
                )
            };

        Ok(rooms_iter
            .map(|room| MenuChoiceOption {
                prompt: prompt.clone(),
                title: room.title.clone(),
                menu_text: room.title.clone(),
                command: room.id.clone(),
                transcript_label: Some(room.title.clone()),
            })
            .collect())
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

    pub fn yelp_review(&self) -> Result<Option<YelpReview>, Box<dyn Error>> {
        {
            let cached = self
                .yelp_review
                .lock()
                .map_err(|e| e.to_string())?;
            if let Some(review) = cached.as_ref() {
                return Ok(Some(review.clone()));
            }
        }
        {
            let state = self
                .state
                .lock()
                .map_err(|_| "failed to lock runtime state for yelp review guard")?;
            if !state.game_over {
                return Ok(None);
            }
        }
        let (stats_context, session_summary, relationship_lines) = {
            let state = self
                .state
                .lock()
                .map_err(|_| "failed to lock runtime state for yelp review")?;
            let noa_id = "noa";
            let current = state.actor_stats_snapshot(noa_id);
            let deltas = state.actor_stat_deltas(noa_id).unwrap_or_default();
            let stats_context = [
                "trust",
                "openness",
                "focus",
                "resistance",
                "energy",
                "secrets_found",
            ]
            .iter()
            .filter_map(|key| {
                let val = current.get(*key).copied()?;
                let delta = deltas.get(*key).copied().unwrap_or(0);
                Some(format!("  {key}: {val} ({delta:+})"))
            })
            .collect::<Vec<_>>()
            .join("\n");
            let session_summary = state.transcript.last().cloned().unwrap_or_default();
            let relationship_lines = self.relationship_status_lines().unwrap_or_default();
            (stats_context, session_summary, relationship_lines)
        };
        let request = YelpReviewRequest {
            locale: self.content.locale.clone(),
            system_text: self.content.system_text.clone(),
            actor_name: self
                .content
                .actors
                .iter()
                .find(|a| a.id == "noa")
                .map(|a| a.name.clone())
                .unwrap_or_else(|| "Patient".to_string()),
            other_person_name: "You".to_string(),
            stats_context,
            session_summary,
            relationship_lines,
        };
        let review = self
            .dialogue
            .generate_yelp_review(&request)
            .map_err(|e| format!("yelp review generation failed: {e}"))?;
        {
            let mut cached = self
                .yelp_review
                .lock()
                .map_err(|e| e.to_string())?;
            *cached = Some(review.clone());
        }
        Ok(Some(review))
    }

    pub fn follow_actor_options(&self) -> Result<Vec<MenuChoiceOption>, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for follow options")?;
        let follow_prompt = self.content.ui_text.follow_actor_prompt.clone();
        let nobody_label = self.content.ui_text.follow_nobody_option.clone();
        let mut options = vec![MenuChoiceOption {
            prompt: follow_prompt.clone(),
            title: nobody_label.clone(),
            menu_text: nobody_label,
            command: "none".to_string(),
            transcript_label: None,
        }];
        options.extend(self.content.actors.iter().map(|actor| {
            let room_id = state.actor_room_id(&actor.id, &actor.room_id);
            let room_title = self
                .content
                .room(room_id)
                .map(|room| room.title.clone())
                .unwrap_or_else(|| room_id.to_string());
            MenuChoiceOption {
                prompt: follow_prompt.clone(),
                title: actor.name.clone(),
                menu_text: format!("{} ({room_title})", actor.name),
                command: actor.id.clone(),
                transcript_label: Some(actor.name.clone()),
            }
        }));
        Ok(options)
    }

    pub fn switch_room_view(&self, room_id: &str) -> Result<TurnOutcome, Box<dyn Error>> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for room switching")?;
        let Some(room) = self.content.room(room_id) else {
            return Err(format!("missing room '{room_id}'").into());
        };
        let turn_number = state.turn_number + 1;
        let mut events = vec![TimestampedWorldEvent::now(WorldEvent::TurnStarted {
            turn_number,
            raw_input: format!("switch-room:{room_id}"),
            advances_time: false,
        })];
        if state.current_room_id != room.id {
            events.push(TimestampedWorldEvent::now(WorldEvent::PlayerMoved {
                from_room_id: state.current_room_id.clone(),
                to_room_id: room.id.clone(),
            }));
        }
        events.push(TimestampedWorldEvent::now(
            WorldEvent::CurrentRoomObserved {
                room_id: room.id.clone(),
                mode: ObservationMode::Summary,
            },
        ));
        let reduced = apply_events(&mut state, self.content.as_ref(), &events);
        refresh_conversation_summaries(self.content.as_ref(), self.dialogue.as_ref(), &mut state)
            .map_err(std::io::Error::other)?;
        Ok(TurnOutcome {
            text: reduced.lines.join("\n\n"),
            game_over: reduced.game_over,
        })
    }

    pub fn follow_actor(
        &self,
        actor_id: Option<&str>,
    ) -> Result<TurnOutcome, Box<dyn Error>> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for following actor")?;
        let turn_number = state.turn_number + 1;
        let feedback_line = format!(
            "> {}",
            match actor_id {
                Some(actor_id) => {
                    let actor = self
                        .content
                        .actor(actor_id)
                        .ok_or_else(|| format!("missing actor '{actor_id}'"))?;
                    self.content
                        .ui_text
                        .follow_actor_transcript
                        .replace("{title}", &actor.name)
                }
                None => self.content.ui_text.follow_actor_stop_transcript.clone(),
            }
        );
        let mut events = vec![TimestampedWorldEvent::now(WorldEvent::TurnStarted {
            turn_number,
            raw_input: format!("follow:{}", actor_id.unwrap_or("none")),
            advances_time: false,
        })];
        match actor_id {
            Some(actor_id) => {
                let actor = self
                    .content
                    .actor(actor_id)
                    .ok_or_else(|| format!("missing actor '{actor_id}'"))?;
                state.followed_actor_id = Some(actor_id.to_string());
                let room_id = state.actor_room_id(actor_id, &actor.room_id).to_string();
                if state.current_room_id != room_id {
                    events.push(TimestampedWorldEvent::now(WorldEvent::PlayerMoved {
                        from_room_id: state.current_room_id.clone(),
                        to_room_id: room_id.clone(),
                    }));
                    events.push(TimestampedWorldEvent::now(
                        WorldEvent::CurrentRoomObserved {
                            room_id,
                            mode: ObservationMode::Summary,
                        },
                    ));
                }
            }
            None => {
                state.followed_actor_id = None;
            }
        }
        let reduced = apply_events(&mut state, self.content.as_ref(), &events);
        refresh_conversation_summaries(self.content.as_ref(), self.dialogue.as_ref(), &mut state)
            .map_err(std::io::Error::other)?;
        let mut lines = vec![feedback_line];
        lines.extend(reduced.lines);
        Ok(TurnOutcome {
            text: lines.join("\n\n"),
            game_over: reduced.game_over,
        })
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
            eprintln!("[debug] run_actor_turns: applying {} tick events, active_stages={:?}, story_vars_keys={:?}",
                tick.events.len(),
                state.active_objective_stage_ids,
                state.story_vars.keys().collect::<Vec<_>>());
            let logged_events = tick
                .events
                .into_iter()
                .map(TimestampedWorldEvent::now)
                .collect::<Vec<_>>();
            let reduced = apply_events(&mut state, self.content.as_ref(), &logged_events);
            eprintln!("[debug] run_actor_turns: after apply, pending_projector_id={:?}",
                state.pending_projector_sequence_id);
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

mod stats_trace;
use self::stats_trace::stats_trace_snapshot;
