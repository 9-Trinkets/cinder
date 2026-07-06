use super::{CinderRuntime, FinalChapterSummary};
use crate::content::types::ContentPack;
use crate::engine::dialogue::{
    ChapterRelationshipSummaryRequest, ChapterScriptSummaryRequest, SessionFeedback,
    SessionFeedbackRequest, SynapseChapterSummaryGenerator,
};
use crate::engine::dialogue_grounding::{render_story_text, viewer_participant_id};
use crate::engine::state::WorldState;
use std::collections::BTreeMap;
use std::error::Error;
use std::sync::Arc;

impl CinderRuntime {
    pub fn relationship_status_lines(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for relationship summary")?;
        Ok(self.relationship_status_lines_for_state(&state))
    }

    fn relationship_status_lines_for_state(&self, state: &WorldState) -> Vec<String> {
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
        lines.into_iter().map(|(_, line)| line).collect()
    }

    fn select_session_feedback_actor_id(&self, state: &WorldState) -> Option<String> {
        pick_session_feedback_actor_id(
            self.content.as_ref(),
            state,
            &viewer_participant_id(self.content.as_ref()),
        )
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
        let transcript_lines = chapter_transcript_lines(transcript, chapter_start_index);
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

    pub fn session_feedback(&self) -> Result<Option<SessionFeedback>, Box<dyn Error>> {
        {
            let cached = self.session_feedback.lock().map_err(|error| error.to_string())?;
            if let Some(review) = cached.as_ref() {
                return Ok(Some(review.clone()));
            }
        }
        {
            let state = self
                .state
                .lock()
                .map_err(|_| "failed to lock runtime state for session feedback guard")?;
            if !state.game_over {
                return Ok(None);
            }
        }
        let (actor_id, current, deltas, stats_context, session_summary, relationship_lines) = {
            let state = self
                .state
                .lock()
                .map_err(|_| "failed to lock runtime state for session feedback")?;
            let Some(actor_id) = self.select_session_feedback_actor_id(&state) else {
                return Ok(None);
            };
            let current = state.actor_stats_snapshot(&actor_id);
            let deltas = state.actor_stat_deltas(&actor_id).unwrap_or_default();
            let stats_context = ["trust", "openness", "focus", "resistance", "energy", "secrets_found"]
                .iter()
                .filter_map(|key| {
                    let value = current.get(*key).copied()?;
                    let delta = deltas.get(*key).copied().unwrap_or(0);
                    Some(format!("  {key}: {value} ({delta:+})"))
                })
                .collect::<Vec<_>>()
                .join("\n");
            let session_summary = state.transcript.last().cloned().unwrap_or_default();
            let relationship_lines = self.relationship_status_lines_for_state(&state);
            (
                actor_id,
                current,
                deltas,
                stats_context,
                session_summary,
                relationship_lines,
            )
        };
        let request = SessionFeedbackRequest {
            locale: self.content.locale.clone(),
            system_text: self.content.system_text.clone(),
            actor_name: self
                .content
                .actors
                .iter()
                .find(|actor| actor.id == actor_id)
                .map(|actor| actor.name.clone())
                .unwrap_or_else(|| "Patient".to_string()),
            other_person_name: "You".to_string(),
            stats_context,
            session_summary,
            relationship_lines,
        };
        let review = match self.try_llm_session_feedback(request) {
            Some(review) => review,
            None => self.fallback_session_feedback(&current, &deltas),
        };
        {
            let mut cached = self.session_feedback.lock().map_err(|error| error.to_string())?;
            *cached = Some(review.clone());
        }
        Ok(Some(review))
    }

    fn try_llm_session_feedback(
        &self,
        request: SessionFeedbackRequest,
    ) -> Option<SessionFeedback> {
        let (tx, rx) = std::sync::mpsc::channel();
        let dialogue = Arc::clone(&self.dialogue);
        std::thread::spawn(move || {
            let _ = tx.send(dialogue.generate_session_feedback(&request));
        });
        match rx.recv_timeout(std::time::Duration::from_secs(30)) {
            Ok(Ok(review)) => Some(review),
            Ok(Err(error)) => {
                eprintln!("[cinder] session feedback LLM failed: {error}, using stat fallback");
                None
            }
            Err(_) => {
                eprintln!("[cinder] session feedback LLM timed out, using stat fallback");
                None
            }
        }
    }

    fn fallback_session_feedback(
        &self,
        current: &BTreeMap<String, i32>,
        deltas: &BTreeMap<String, i32>,
    ) -> SessionFeedback {
        let trust_delta = deltas.get("trust").copied().unwrap_or(0);
        let openness_delta = deltas.get("openness").copied().unwrap_or(0);
        let resistance_delta = deltas.get("resistance").copied().unwrap_or(0);
        let energy_delta = deltas.get("energy").copied().unwrap_or(0);
        let secrets_found = current.get("secrets_found").copied().unwrap_or(0);
        let net = trust_delta + openness_delta - resistance_delta + energy_delta + secrets_found * 2;
        let rating = if net >= 8 {
            5
        } else if net >= 4 {
            4
        } else if net >= 0 {
            3
        } else if net >= -4 {
            2
        } else {
            1
        };
        SessionFeedback {
            rating,
            review_text: String::new(),
        }
    }
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

fn pick_session_feedback_actor_id(
    content: &ContentPack,
    state: &WorldState,
    viewer_id: &str,
) -> Option<String> {
    if !content.settings.session_feedback_actor_id.is_empty()
        && content
            .actors
            .iter()
            .any(|actor| actor.id == content.settings.session_feedback_actor_id)
    {
        return Some(content.settings.session_feedback_actor_id.clone());
    }
    state
        .conversation_memory
        .iter()
        .filter_map(|(key, lines)| {
            let (participant_a_id, participant_b_id) = WorldState::conversation_participants(key)?;
            let actor_id = if participant_a_id == viewer_id {
                participant_b_id
            } else if participant_b_id == viewer_id {
                participant_a_id
            } else {
                return None;
            };
            content.actor(actor_id)?;
            let last_sequence = lines.last().map(|line| line.event_sequence).unwrap_or(0);
            Some((last_sequence, actor_id.to_string()))
        })
        .max_by_key(|(last_sequence, _)| *last_sequence)
        .map(|(_, actor_id)| actor_id)
        .or_else(|| content.actors.first().map(|actor| actor.id.clone()))
}
