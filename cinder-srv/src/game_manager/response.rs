use cinder_core::engine::runtime::CinderRuntime;
use serde::Serialize;
use std::any::Any;

#[derive(Clone, Serialize)]
pub struct SessionFeedbackData {
    pub rating: u32,
    pub review_text: String,
    pub subject_name: String,
}

#[derive(Clone, Serialize)]
pub struct MovieFrameData {
    pub text: String,
    pub duration_ms: u64,
}

#[derive(Clone, Serialize)]
pub struct MovieData {
    pub title: String,
    pub frames: Vec<MovieFrameData>,
    pub narrative_lines: Vec<String>,
}

#[derive(Clone, Serialize)]
pub struct CommandResponse {
    pub text: String,
    pub game_over: bool,
    pub movie: Option<MovieData>,
    pub session_feedback: Option<SessionFeedbackData>,
}

pub(super) fn panic_payload_message(payload: &Box<dyn Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else if let Some(message) = payload.downcast_ref::<&'static str>() {
        (*message).to_string()
    } else {
        "unknown panic payload".to_string()
    }
}

pub(super) fn session_feedback_data(runtime: &CinderRuntime) -> Option<SessionFeedbackData> {
    runtime
        .session_feedback()
        .ok()
        .flatten()
        .map(|review| SessionFeedbackData {
            rating: review.rating,
            review_text: review.review_text,
            subject_name: runtime
                .current_patient_name()
                .ok()
                .flatten()
                .unwrap_or_else(|| "Patient".to_string()),
        })
}

pub fn consume_projector_sequence(runtime: &CinderRuntime) -> Option<MovieData> {
    let sequence = runtime.consume_pending_projector_sequence().ok()??;
    let narrative_lines = runtime
        .consume_pending_projector_narrative_lines()
        .ok()
        .unwrap_or_default();
    let frames = sequence
        .frames
        .into_iter()
        .map(|frame| MovieFrameData {
            text: frame.text,
            duration_ms: frame.duration_ms,
        })
        .collect();
    Some(MovieData {
        title: sequence.title,
        frames,
        narrative_lines,
    })
}
