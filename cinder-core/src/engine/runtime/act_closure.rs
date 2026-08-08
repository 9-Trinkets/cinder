use super::{ActClosure, ActClosureSection, CinderRuntime};
use crate::content::types::ActClosureSource;
use crate::engine::dialogue::{
    ChapterRelationshipSummaryRequest, ChapterScriptSummaryRequest, SynapseChapterSummaryGenerator,
};
use crate::engine::dialogue_grounding::render_story_text;
use crate::engine::state::{GamePhase, WorldState};
use serde::Serialize;
use std::error::Error;

#[derive(Debug, Clone, Serialize)]
pub struct RelationshipPair {
    pub actor_a: String,
    pub actor_b: String,
    pub connection: i32,
    pub attraction: i32,
    pub safety: i32,
}

pub struct FinalChapterSummary {
    pub what_happened: String,
    pub relationship_status: String,
    pub next_chapter_preview: String,
}

impl CinderRuntime {
    pub fn relationship_pairs(&self) -> Result<Vec<RelationshipPair>, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for relationship pairs")?;
        let mut pairs = Vec::new();
        for (index, actor) in self.content.actors.iter().enumerate() {
            for other in self.content.actors.iter().skip(index + 1) {
                let stats = state.pair_stats_snapshot(&actor.id, &other.id);
                if stats.is_empty() {
                    continue;
                }
                let connection = stats.get("connection").copied().unwrap_or(0);
                if connection == 0 {
                    continue;
                }
                pairs.push(RelationshipPair {
                    actor_a: self.display_actor_name_for_state(&state, &actor.id),
                    actor_b: self.display_actor_name_for_state(&state, &other.id),
                    connection,
                    attraction: stats.get("attraction").copied().unwrap_or(0),
                    safety: stats.get("safety").copied().unwrap_or(0),
                });
            }
        }
        pairs.sort_by(|a, b| b.connection.cmp(&a.connection));
        Ok(pairs)
    }

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

    pub(super) fn final_chapter_summary(
        &self,
        transcript_lines: &[String],
    ) -> Result<FinalChapterSummary, Box<dyn Error>> {
        let transcript_lines = chapter_transcript_lines(transcript_lines);
        let relationship_lines = self.relationship_status_lines()?;
        let preview = self
            .current_next_chapter_preview()?
            .unwrap_or_else(|| self.content.ui_text.final_summary_empty_preview.clone());

        let summary_generator = SynapseChapterSummaryGenerator::new(self.workflow.clone())
            .map_err(|error| format!("failed to configure chapter summary roles: {error}"))?;

        let what_happened = if transcript_lines.is_empty() {
            self.content.ui_text.act_closure_empty_highlights.clone()
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
            self.content.ui_text.act_closure_empty_relationships.clone()
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

    pub fn act_closure(
        &self,
        transcript_lines: &[String],
    ) -> Result<Option<ActClosure>, Box<dyn Error>> {
        {
            let cached = self.act_closure.lock().map_err(|error| error.to_string())?;
            if let Some(closure) = cached.as_ref() {
                return Ok(Some(closure.clone()));
            }
        }
        {
            let state = self
                .state
                .lock()
                .map_err(|_| "failed to lock runtime state for act closure guard")?;
            if state.phase != GamePhase::ActEnded {
                return Ok(None);
            }
        }
        let definition = &self.content.ui_text.act_closure;
        if definition.sections.is_empty() || definition.title.trim().is_empty() {
            return Ok(None);
        }

        let summary = definition
            .sections
            .iter()
            .any(|section| {
                matches!(
                    section.source,
                    ActClosureSource::TranscriptHighlights
                        | ActClosureSource::RelationshipSummary
                        | ActClosureSource::ContinuationPreview
                )
            })
            .then(|| self.final_chapter_summary(transcript_lines))
            .transpose()?;

        let perspective = definition
            .sections
            .iter()
            .any(|section| {
                matches!(
                    section.source,
                    ActClosureSource::PerspectiveRating | ActClosureSource::PerspectiveReview
                )
            })
            .then(|| self.build_perspective_review())
            .transpose()?
            .flatten();

        let subject_name = perspective
            .as_ref()
            .map(|review| review.subject_name.clone())
            .or_else(|| self.current_cast_member_name().ok().flatten());

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
                ActClosureSource::PerspectiveRating => {
                    perspective
                        .as_ref()
                        .map(|review| ActClosureSection::Rating {
                            title: section.title.clone(),
                            value: review.review.rating,
                            max: 5,
                        })
                }
                ActClosureSource::PerspectiveReview => {
                    perspective.as_ref().map(|review| ActClosureSection::Text {
                        title: section.title.clone(),
                        body: review.review.review_text.clone(),
                    })
                }
                ActClosureSource::TranscriptHighlights => {
                    summary.as_ref().map(|summary| ActClosureSection::Text {
                        title: section.title.clone(),
                        body: summary.what_happened.clone(),
                    })
                }
                ActClosureSource::RelationshipSummary => {
                    summary.as_ref().map(|summary| ActClosureSection::Text {
                        title: section.title.clone(),
                        body: summary.relationship_status.clone(),
                    })
                }
                ActClosureSource::ContinuationPreview => {
                    summary.as_ref().map(|summary| ActClosureSection::Text {
                        title: section.title.clone(),
                        body: summary.next_chapter_preview.clone(),
                    })
                }
            })
            .collect::<Vec<_>>();

        if sections.is_empty() {
            return Ok(None);
        }

        let closure = ActClosure {
            title: definition.title.clone(),
            subtitle,
            sections,
        };
        {
            let mut cached = self.act_closure.lock().map_err(|error| error.to_string())?;
            *cached = Some(closure.clone());
        }
        Ok(Some(closure))
    }

    pub fn game_closure(
        &self,
        transcript_lines: &[String],
    ) -> Result<Option<ActClosure>, Box<dyn Error>> {
        {
            let cached = self
                .game_closure
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
                .map_err(|_| "failed to lock runtime state for game closure guard")?;
            if state.phase != GamePhase::GameEnded {
                return Ok(None);
            }
        }
        let definition = &self.content.ui_text.game_closure;
        if definition.sections.is_empty() || definition.title.trim().is_empty() {
            return Ok(None);
        }

        let summary = definition
            .sections
            .iter()
            .any(|section| {
                matches!(
                    section.source,
                    ActClosureSource::TranscriptHighlights
                        | ActClosureSource::RelationshipSummary
                        | ActClosureSource::ContinuationPreview
                )
            })
            .then(|| self.final_chapter_summary(transcript_lines))
            .transpose()?;

        let perspective = definition
            .sections
            .iter()
            .any(|section| {
                matches!(
                    section.source,
                    ActClosureSource::PerspectiveRating | ActClosureSource::PerspectiveReview
                )
            })
            .then(|| self.build_perspective_review())
            .transpose()?
            .flatten();

        let subject_name = perspective
            .as_ref()
            .map(|review| review.subject_name.clone())
            .or_else(|| self.current_cast_member_name().ok().flatten());

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
                ActClosureSource::PerspectiveRating => {
                    perspective
                        .as_ref()
                        .map(|review| ActClosureSection::Rating {
                            title: section.title.clone(),
                            value: review.review.rating,
                            max: 5,
                        })
                }
                ActClosureSource::PerspectiveReview => {
                    perspective.as_ref().map(|review| ActClosureSection::Text {
                        title: section.title.clone(),
                        body: review.review.review_text.clone(),
                    })
                }
                ActClosureSource::TranscriptHighlights => {
                    summary.as_ref().map(|summary| ActClosureSection::Text {
                        title: section.title.clone(),
                        body: summary.what_happened.clone(),
                    })
                }
                ActClosureSource::RelationshipSummary => {
                    summary.as_ref().map(|summary| ActClosureSection::Text {
                        title: section.title.clone(),
                        body: summary.relationship_status.clone(),
                    })
                }
                ActClosureSource::ContinuationPreview => {
                    summary.as_ref().map(|summary| ActClosureSection::Text {
                        title: section.title.clone(),
                        body: summary.next_chapter_preview.clone(),
                    })
                }
            })
            .collect::<Vec<_>>();

        if sections.is_empty() {
            return Ok(None);
        }

        let closure = ActClosure {
            title: definition.title.clone(),
            subtitle,
            sections,
        };
        {
            let mut cached = self
                .game_closure
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
