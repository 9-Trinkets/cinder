use cinder_core::engine::runtime::{ActClosure, CinderRuntime};
use serde::Serialize;

use super::ui::UiSnapshot;

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
    pub act_closure: Option<ActClosure>,
    pub game_closure: Option<ActClosure>,
    pub ui_snapshot: Option<UiSnapshot>,
}

pub(super) fn act_closure_data(
    runtime: &CinderRuntime,
    transcript_lines: &[String],
) -> Option<ActClosure> {
    runtime.act_closure(transcript_lines).ok().flatten()
}

pub(super) fn game_closure_data(
    runtime: &CinderRuntime,
    transcript_lines: &[String],
) -> Option<ActClosure> {
    runtime.game_closure(transcript_lines).ok().flatten()
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
