use super::{CinderRuntime, FinalChapterSummary, SessionClosure, SessionClosureSection};
use crate::content::types::{ContentPack, SessionClosureSource};
use crate::engine::dialogue::{
    ChapterRelationshipSummaryRequest, ChapterScriptSummaryRequest, PerspectiveReview,
    PerspectiveReviewRequest, SynapseChapterSummaryGenerator,
};
use crate::engine::dialogue_grounding::{render_story_text, viewer_participant_id};
use crate::engine::state::{WorldState, current_patient_name, display_actor_name};
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
                                display_actor_name(state, actor),
                                display_actor_name(state, other),
                                rendered_stats.join(", ")
                            ),
                        ))
                    })
            })
            .collect::<Vec<_>>();
        lines.sort_by(|left, right| right.cmp(left));
        lines.into_iter().map(|(_, line)| line).collect()
    }

    fn select_perspective_actor_id(&self, state: &WorldState) -> Option<String> {
        pick_perspective_actor_id(
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

    pub fn final_chapter_summary(&self) -> Result<FinalChapterSummary, Box<dyn Error>> {
        let transcript_lines = self.transcript_lines()?;
        let transcript_lines = chapter_transcript_lines(&transcript_lines);
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

    pub fn session_closure(&self) -> Result<Option<SessionClosure>, Box<dyn Error>> {
        {
            let cached = self
                .session_closure
                .lock()
                .map_err(|error| error.to_string())?;
            if let Some(closure) = cached.as_ref() {
                return Ok(Some(closure.clone()));
            }
        }
        {
            let state = self
                .state
                .lock()
                .map_err(|_| "failed to lock runtime state for session closure guard")?;
            if !state.game_over {
                return Ok(None);
            }
        }
        let definition = &self.content.ui_text.session_closure;
        if definition.sections.is_empty() || definition.title.trim().is_empty() {
            return Ok(None);
        }

        let summary = definition
            .sections
            .iter()
            .any(|section| {
                matches!(
                    section.source,
                    SessionClosureSource::TranscriptHighlights
                        | SessionClosureSource::RelationshipSummary
                        | SessionClosureSource::ContinuationPreview
                )
            })
            .then(|| self.final_chapter_summary())
            .transpose()?;

        let perspective = definition
            .sections
            .iter()
            .any(|section| {
                matches!(
                    section.source,
                    SessionClosureSource::PerspectiveRating
                        | SessionClosureSource::PerspectiveReview
                )
            })
            .then(|| self.build_perspective_review())
            .transpose()?
            .flatten();

        let subject_name = perspective
            .as_ref()
            .map(|review| review.subject_name.clone())
            .or_else(|| self.current_patient_name().ok().flatten());

        let subtitle = if definition.subtitle_template.trim().is_empty() {
            None
        } else {
            Some(self.content.render_template(
                &definition.subtitle_template,
                &[("subject_name", subject_name.as_deref().unwrap_or(""))],
            ))
        }
        .filter(|value| !value.trim().is_empty());

        let sections = definition
            .sections
            .iter()
            .filter_map(|section| match section.source {
                SessionClosureSource::PerspectiveRating => {
                    perspective
                        .as_ref()
                        .map(|review| SessionClosureSection::Rating {
                            title: section.title.clone(),
                            value: review.review.rating,
                            max: 5,
                        })
                }
                SessionClosureSource::PerspectiveReview => {
                    perspective
                        .as_ref()
                        .map(|review| SessionClosureSection::Text {
                            title: section.title.clone(),
                            body: review.review.review_text.clone(),
                        })
                }
                SessionClosureSource::TranscriptHighlights => {
                    summary.as_ref().map(|summary| SessionClosureSection::Text {
                        title: section.title.clone(),
                        body: summary.what_happened.clone(),
                    })
                }
                SessionClosureSource::RelationshipSummary => {
                    summary.as_ref().map(|summary| SessionClosureSection::Text {
                        title: section.title.clone(),
                        body: summary.relationship_status.clone(),
                    })
                }
                SessionClosureSource::ContinuationPreview => {
                    summary.as_ref().map(|summary| SessionClosureSection::Text {
                        title: section.title.clone(),
                        body: summary.next_chapter_preview.clone(),
                    })
                }
            })
            .collect::<Vec<_>>();

        if sections.is_empty() {
            return Ok(None);
        }

        let closure = SessionClosure {
            title: definition.title.clone(),
            subtitle,
            sections,
        };
        {
            let mut cached = self
                .session_closure
                .lock()
                .map_err(|error| error.to_string())?;
            *cached = Some(closure.clone());
        }
        Ok(Some(closure))
    }

    pub(super) fn build_perspective_review(
        &self,
    ) -> Result<Option<CachedPerspectiveReview>, Box<dyn Error>> {
        let (
            actor_name,
            subject_name,
            current,
            deltas,
            stats_context,
            session_summary,
            relationship_lines,
        ) = {
            let state = self
                .state
                .lock()
                .map_err(|_| "failed to lock runtime state for perspective review")?;
            let Some(actor_id) = self.select_perspective_actor_id(&state) else {
                return Ok(None);
            };
            let actor_name = self
                .content
                .actor(&actor_id)
                .map(|actor| display_actor_name(&state, actor))
                .unwrap_or_else(|| "Patient".to_string());
            let subject_name = current_patient_name(&state).unwrap_or_else(|| actor_name.clone());
            let current = state.actor_stats_snapshot(&actor_id);
            let deltas = state.actor_stat_deltas(&actor_id).unwrap_or_default();
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
                let value = current.get(*key).copied()?;
                let delta = deltas.get(*key).copied().unwrap_or(0);
                Some(format!("  {key}: {value} ({delta:+})"))
            })
            .collect::<Vec<_>>()
            .join("\n");
            let session_summary = state.transcript.last().cloned().unwrap_or_default();
            let relationship_lines = self.relationship_status_lines_for_state(&state);
            (
                actor_name,
                subject_name,
                current,
                deltas,
                stats_context,
                session_summary,
                relationship_lines,
            )
        };
        let request = PerspectiveReviewRequest {
            locale: self.content.locale.clone(),
            system_text: self.content.system_text.clone(),
            actor_name,
            other_person_name: "You".to_string(),
            stats_context,
            session_summary,
            relationship_lines,
        };
        let review = match self.try_llm_perspective_review(request) {
            Some(review) => review,
            None => self.fallback_perspective_review(&current, &deltas),
        };
        Ok(Some(CachedPerspectiveReview {
            subject_name,
            review,
        }))
    }

    fn try_llm_perspective_review(
        &self,
        request: PerspectiveReviewRequest,
    ) -> Option<PerspectiveReview> {
        let (tx, rx) = std::sync::mpsc::channel();
        let dialogue = Arc::clone(&self.dialogue);
        std::thread::spawn(move || {
            let _ = tx.send(dialogue.generate_perspective_review(&request));
        });
        match rx.recv_timeout(std::time::Duration::from_secs(30)) {
            Ok(Ok(review)) => Some(review),
            Ok(Err(error)) => {
                eprintln!("[cinder] perspective review LLM failed: {error}, using stat fallback");
                None
            }
            Err(_) => {
                eprintln!("[cinder] perspective review LLM timed out, using stat fallback");
                None
            }
        }
    }

    fn fallback_perspective_review(
        &self,
        current: &BTreeMap<String, i32>,
        deltas: &BTreeMap<String, i32>,
    ) -> PerspectiveReview {
        let trust_delta = deltas.get("trust").copied().unwrap_or(0);
        let openness_delta = deltas.get("openness").copied().unwrap_or(0);
        let resistance_delta = deltas.get("resistance").copied().unwrap_or(0);
        let energy_delta = deltas.get("energy").copied().unwrap_or(0);
        let secrets_found = current.get("secrets_found").copied().unwrap_or(0);
        let net =
            trust_delta + openness_delta - resistance_delta + energy_delta + secrets_found * 2;
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
        PerspectiveReview {
            rating,
            review_text: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct CachedPerspectiveReview {
    pub(super) subject_name: String,
    pub(super) review: PerspectiveReview,
}

fn chapter_transcript_lines(transcript: &[String]) -> Vec<String> {
    transcript
        .iter()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('>'))
        .map(ToString::to_string)
        .collect()
}

fn pick_perspective_actor_id(
    content: &ContentPack,
    _state: &WorldState,
    _viewer_id: &str,
) -> Option<String> {
    if !content.settings.closure_perspective_actor_id.is_empty()
        && content
            .actors
            .iter()
            .any(|actor| actor.id == content.settings.closure_perspective_actor_id)
    {
        return Some(content.settings.closure_perspective_actor_id.clone());
    }
    None
}
