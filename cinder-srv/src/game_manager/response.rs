use cinder_core::engine::runtime::{CinderRuntime, SessionClosure};
use serde::Serialize;

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
    pub session_closure: Option<SessionClosure>,
}

pub(super) fn session_closure_data(runtime: &CinderRuntime) -> Option<SessionClosure> {
    runtime.session_closure().ok().flatten()
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
