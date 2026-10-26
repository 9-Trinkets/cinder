use cinder_core::engine::runtime::{ActClosure, CinderRuntime};
use serde::Serialize;

use super::ui::UiSnapshot;

#[derive(Clone, Serialize)]
pub struct MovieFrameData {
    pub text: String,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub heading: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub tint: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub text_color: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub ambient: String,
}

/// Look of a card movie, as the pack set it. Empty settings use the theme.
#[derive(Clone, Serialize)]
pub struct MovieCardData {
    pub background: String,
    pub text_color: String,
    pub font: String,
    pub align: String,
    pub ambient: String,
    pub reveal: String,
    pub fade_ms: u64,
}

#[derive(Clone, Serialize)]
pub struct MovieData {
    pub title: String,
    /// `frames` or `card`.
    pub style: String,
    pub card: MovieCardData,
    pub frames: Vec<MovieFrameData>,
    pub narrative_lines: Vec<String>,
}

#[derive(Clone, Serialize, Default)]
pub struct CommandResponse {
    /// The narrative lines produced by the command or tick, each tagged with its styling kind.
    #[serde(default)]
    pub lines: Vec<cinder_core::engine::narrative::NarrativeLine>,
    pub game_over: bool,
    pub movie: Option<MovieData>,
    pub act_closure: Option<ActClosure>,
    pub game_closure: Option<ActClosure>,
    pub ui_snapshot: Option<UiSnapshot>,
}

impl CommandResponse {
    pub fn new(
        lines: Vec<cinder_core::engine::narrative::NarrativeLine>,
        game_over: bool,
        ui_snapshot: Option<UiSnapshot>,
    ) -> Self {
        CommandResponse {
            lines,
            game_over,
            ui_snapshot,
            ..Default::default()
        }
    }
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
            heading: frame.heading,
            tint: frame.tint,
            text_color: frame.text_color,
            ambient: frame.ambient,
        })
        .collect();
    let style = match sequence.style {
        cinder_core::content::types::MovieStyle::Frames => "frames",
        cinder_core::content::types::MovieStyle::Card => "card",
    };
    Some(MovieData {
        title: sequence.title,
        style: style.to_string(),
        card: MovieCardData {
            background: sequence.card.background,
            text_color: sequence.card.text_color,
            font: sequence.card.font,
            align: sequence.card.align,
            ambient: sequence.card.ambient,
            reveal: sequence.card.reveal,
            fade_ms: sequence.card.fade_ms,
        },
        frames,
        narrative_lines,
    })
}
