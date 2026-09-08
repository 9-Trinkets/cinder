//! Content-driven scripted conversation sequences: pre-authored, strictly
//! deterministic lines the engine plays back instead of (or before) live
//! narration or actor dialogue. The canonical consumer is the opening
//! exchange — Layla waking and the handler checking in over comms — but the
//! mechanism is generic so a beat or hook can queue any sequence.

use super::AdvanceEffect;
use serde::{Deserialize, Serialize};

/// An exact story-variable condition that must hold before a queued sequence
/// can advance. Unmet gates pause playback without consuming a step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptedSequenceGate {
    pub story_var: String,
    pub expected: String,
}

/// One scripted line in a sequence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "voice", rename_all = "snake_case")]
pub enum ScriptedLine {
    /// Narration in the narrator's voice (Layla's warm second-person
    /// perception). Rendered as a normal narration line.
    Narrate { line: String },
    /// A message spoken on a declared channel to its hearing audience. The
    /// speaker is explicit so scripts may author lines for the player
    /// character without waiting for freeform input.
    Channel {
        channel_id: String,
        speaker_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        recipient_id: Option<String>,
        line: String,
    },
}

/// A named, ordered scripted conversation sequence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptedSequence {
    pub id: String,
    /// Optional human-readable name for presentations.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub label: String,
    /// Exact story-variable conditions that pause the sequence until all hold.
    #[serde(default)]
    pub gates: Vec<ScriptedSequenceGate>,
    /// The lines, played in order, one per advancing turn while the sequence
    /// runs.
    pub steps: Vec<ScriptedLine>,
    /// Deterministic state changes emitted after the final line.
    #[serde(default)]
    pub completion_effects: Vec<AdvanceEffect>,
}

/// The complete set of a pack's scripted sequences, loaded from
/// `locales/<locale>/sequences.json`. Absent file → no scripted sequences.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SequencesDefinition {
    #[serde(default)]
    pub sequences: Vec<ScriptedSequence>,
}
