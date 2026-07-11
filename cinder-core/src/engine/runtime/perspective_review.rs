use super::CinderRuntime;
use crate::content::types::ContentPack;
use crate::engine::dialogue::{PerspectiveReview, PerspectiveReviewRequest};
use crate::engine::dialogue_grounding::viewer_participant_id;
use crate::engine::state::{WorldState, current_patient_name, display_actor_name};
use std::collections::BTreeMap;
use std::error::Error;
use std::sync::Arc;

impl CinderRuntime {
    fn select_perspective_actor_id(&self, state: &WorldState) -> Option<String> {
        pick_perspective_actor_id(
            self.content.as_ref(),
            state,
            &viewer_participant_id(self.content.as_ref()),
        )
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
