use super::{CinderRuntime, ObjectiveSummary};
use crate::content::types::{
    ActionDefinition, ContentPack, ItemStorageTarget, OpeningMovieDefinition,
};
use crate::engine::commands::player_command_help_text;
use crate::engine::dialogue_grounding::render_story_text;
use crate::engine::neuron::WorkflowDefinition;
use crate::engine::state::{current_act_intro, current_cast_member_name, display_actor_name};
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};

impl CinderRuntime {
    pub fn current_intro_text(&self) -> Result<String, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for intro text")?;
        Ok(current_act_intro(&state).unwrap_or_else(|| self.content.opening.intro_text.clone()))
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

    pub fn current_cast_member_name(&self) -> Result<Option<String>, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for patient name")?;
        Ok(current_cast_member_name(&state))
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

    pub fn has_item_in_storage(
        &self,
        item_id: &str,
        storage: ItemStorageTarget,
    ) -> Result<bool, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for inventory")?;
        Ok(state.has_item_in_storage(item_id, storage, &state.current_room_id))
    }

    pub fn inventory_items(&self) -> Result<HashMap<String, u32>, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for inventory")?;
        Ok(state.player_inventory.clone())
    }

    pub fn current_room_item_count(&self, item_id: &str) -> Result<u32, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for room items")?;
        Ok(state.item_count_in_storage(
            item_id,
            ItemStorageTarget::CurrentRoom,
            &state.current_room_id,
        ))
    }

    pub fn feature_consumable_count(
        &self,
        room_id: &str,
        feature_id: &str,
        consumable_id: &str,
    ) -> Result<u32, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for consumable stock")?;
        Ok(state.remaining_consumable_stock(room_id, feature_id, consumable_id))
    }

    pub fn active_stage_ids(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for stages")?;
        Ok(state.active_objective_stage_ids.clone())
    }

    pub fn action_is_available(&self, action: &ActionDefinition) -> Result<bool, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for action availability")?;
        Ok(super::super::turn_policies::action_is_available(
            self.content.as_ref(),
            &state,
            action,
            &state.current_room_id,
        ))
    }

    pub fn set_transcript(&self, lines: Vec<String>) -> Result<(), Box<dyn Error>> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for transcript")?;
        state.transcript = lines;
        Ok(())
    }

    pub fn push_transcript_line(&self, line: &str) -> Result<(), Box<dyn Error>> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for transcript")?;
        state.last_transcript_line = Some(line.to_string());
        if state.transcript.last().map(String::as_str) != Some(line) {
            state.transcript.push(line.to_string());
        }
        Ok(())
    }

    pub fn last_transcript_line(&self) -> Result<Option<String>, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for transcript")?;
        Ok(state.last_transcript_line.clone())
    }

    pub fn help_text(&self) -> String {
        let available_commands = player_command_help_text(self.content.as_ref());
        self.content.render_template(
            &self.content.opening.help_text,
            &[("available_commands", available_commands.as_str())],
        )
    }

    pub fn current_objective_summaries(&self) -> Result<Vec<ObjectiveSummary>, Box<dyn Error>> {
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
                        let quest_title = stage
                            .quest_title
                            .as_ref()
                            .map(|title| render_story_text(title, &state));
                        ObjectiveSummary {
                            stage_id: stage.id.clone(),
                            summary,
                            message,
                            quest_id: stage.quest_id.clone(),
                            quest_title,
                            quest_kind: stage.quest_kind.clone(),
                        }
                    })
            })
            .filter(|o| !o.summary.is_empty())
            .collect())
    }

    pub fn completed_stage_ids(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for completed stages")?;
        Ok(state.completed_stage_ids.iter().cloned().collect())
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
        let current_actor_id = state.story_vars.get("act_cast_actor_id");
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
            .filter(|s| match current_actor_id {
                None => true,
                Some(actor_id) => s.advance_signals.iter().any(|sig| {
                    sig.conditions().iter().any(|c| {
                        c.path == "story_vars.act_cast_actor_id"
                            && c.operator == "equal"
                            && c.value.as_str() == Some(actor_id)
                    })
                }),
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
            trace_events: self.trace_events,
            trace_dir: self.trace_dir.clone(),
            act_closure: Arc::new(Mutex::new(None)),
            game_closure: Arc::new(Mutex::new(None)),
        }
    }

    pub fn workflow(&self) -> &WorkflowDefinition {
        &self.workflow
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::types::BeatDefinition;
    use crate::engine::test_fixtures::minimal_test_pack;

    #[test]
    fn current_objective_summaries_includes_quest_metadata() {
        let mut content = minimal_test_pack();
        content.beats.initial_stage_ids = vec!["mq_1".to_string(), "sq_1".to_string()];
        content.beats.stages = vec![
            BeatDefinition {
                id: "mq_1".to_string(),
                quest_id: Some("teleport_scroll".to_string()),
                quest_title: Some("The Teleportation Scroll".to_string()),
                quest_kind: Some("main".to_string()),
                summary: "Find Commander Malik's safe".to_string(),
                update_message: "Infiltrate the bastion and find the safe.".to_string(),
                ..BeatDefinition::default()
            },
            BeatDefinition {
                id: "sq_1".to_string(),
                quest_id: Some("save_zayd".to_string()),
                quest_title: Some("Save the Boy Zayd".to_string()),
                quest_kind: Some("side".to_string()),
                summary: "Locate the steam prison cage".to_string(),
                update_message: "Search for Zayd in the military complex.".to_string(),
                ..BeatDefinition::default()
            },
        ];

        let runtime = CinderRuntime::new(content, false).unwrap();
        let objectives = runtime.current_objective_summaries().unwrap();

        assert_eq!(objectives.len(), 2);
        assert_eq!(objectives[0].quest_id.as_deref(), Some("teleport_scroll"));
        assert_eq!(
            objectives[0].quest_title.as_deref(),
            Some("The Teleportation Scroll")
        );
        assert_eq!(objectives[0].quest_kind.as_deref(), Some("main"));
        assert_eq!(objectives[0].summary, "Find Commander Malik's safe");

        assert_eq!(objectives[1].quest_id.as_deref(), Some("save_zayd"));
        assert_eq!(
            objectives[1].quest_title.as_deref(),
            Some("Save the Boy Zayd")
        );
        assert_eq!(objectives[1].quest_kind.as_deref(), Some("side"));
        assert_eq!(objectives[1].summary, "Locate the steam prison cage");
    }
}

