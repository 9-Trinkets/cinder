//! Playheads for content-driven scripted conversation sequences.
//!
//! Every pack-declared [`ScriptedSequence`] gets a playhead in world state at
//! session creation. A sequence is `running` while each advancing turn should
//! surface its next line; the scanning position lives in `next_step` as a
//! scripted-line index into the pack's sequence definition. Playback is
//! strictly deterministic: the planner appends the next line's events for every
//! running sequence per turn, and the reducer commits each step via
//! `WorldEvent::ScriptedSequenceStepPlayed`.

use serde::{Deserialize, Serialize};

/// Where a scripted sequence's playback stands.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptedSequencePlayhead {
    /// The next step index into the pack's `ScriptedSequence.steps`.
    #[serde(default)]
    pub next_step: usize,
    /// Whether playback has run out of steps. A finished sequence no longer
    /// emits lines unless re-queued.
    #[serde(default)]
    pub finished: bool,
    /// Whether the sequence is currently advancing one line per turn.
    #[serde(default)]
    pub running: bool,
}

impl ScriptedSequencePlayhead {
    pub fn queued() -> Self {
        Self {
            next_step: 0,
            finished: false,
            running: true,
        }
    }

    pub fn advance_to(&mut self, next_step: usize, finished: bool) {
        self.next_step = next_step;
        self.finished = finished;
        if finished {
            self.running = false;
        }
    }
}

impl super::WorldState {
    /// Queues a scripted sequence to play from its start (or re-plays it from
    /// scratch). Unknown ids error so beats/hooks catch typos early.
    pub fn queue_scripted_sequence(&mut self, sequence_id: &str) -> Result<(), String> {
        let playhead = self
            .scripted_sequences
            .get_mut(sequence_id)
            .ok_or_else(|| format!("unknown scripted sequence '{sequence_id}'"))?;
        *playhead = ScriptedSequencePlayhead::queued();
        Ok(())
    }

    pub fn scripted_sequence_playhead(
        &self,
        sequence_id: &str,
    ) -> Option<&ScriptedSequencePlayhead> {
        self.scripted_sequences.get(sequence_id)
    }

    /// Commits a played step: moves the playhead to `next_step` and stops the
    /// sequence once it has run out of steps. Applied by the reducer from
    /// `WorldEvent::ScriptedSequenceStepPlayed` so playback stays
    /// deterministic and replayable.
    pub fn apply_scripted_sequence_progress(
        &mut self,
        sequence_id: &str,
        next_step: usize,
        finished: bool,
    ) {
        if let Some(playhead) = self.scripted_sequences.get_mut(sequence_id) {
            playhead.advance_to(next_step, finished);
        }
    }
}
