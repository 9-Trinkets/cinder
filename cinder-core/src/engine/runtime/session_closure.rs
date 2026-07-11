use super::{CinderRuntime, SessionClosure, SessionClosureSection};
use crate::content::types::SessionClosureSource;
use crate::engine::dialogue::{
    ChapterRelationshipSummaryRequest, ChapterScriptSummaryRequest, SynapseChapterSummaryGenerator,
};
use crate::engine::dialogue_grounding::render_story_text;
use crate::engine::state::WorldState;
use std::error::Error;

pub struct FinalChapterSummary {
    pub what_happened: String,
    pub relationship_status: String,
    pub next_chapter_preview: String,
}

impl CinderRuntime {
    pub fn relationship_status_lines(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for relationship summary")?;
        Ok(self.relationship_status_lines_for_state(&state))
    }

    pub(super) fn relationship_status_lines_for_state(&self, state: &WorldState) -> Vec<String> {
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
                                self.display_actor_name_for_state(state, &actor.id),
                                self.display_actor_name_for_state(state, &other.id),
                                rendered_stats.join(", ")
                            ),
                        ))
                    })
            })
            .collect::<Vec<_>>();
        lines.sort_by(|left, right| right.cmp(left));
        lines.into_iter().map(|(_, line)| line).collect()
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

    fn display_actor_name_for_state(&self, state: &WorldState, actor_id: &str) -> String {
        self.content
            .actor(actor_id)
            .map(|actor| crate::engine::state::display_actor_name(state, actor))
            .unwrap_or_else(|| actor_id.to_string())
    }
}

fn chapter_transcript_lines(transcript: &[String]) -> Vec<String> {
    transcript
        .iter()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('>'))
        .map(ToString::to_string)
        .collect()
}
