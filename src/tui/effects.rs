use std::collections::VecDeque;
use std::time::{Duration, Instant};
use tachyonfx::Effect;

pub struct TranscriptTypewriter {
    char_ms: u64,
    current: Option<TranscriptAnimation>,
    pending_entries: VecDeque<usize>,
}

struct TranscriptAnimation {
    entry_index: usize,
    started_at: Instant,
}

#[derive(Clone, Copy)]
pub struct TranscriptAnimationSnapshot {
    pub entry_index: usize,
    pub visible_chars: usize,
}

impl TranscriptTypewriter {
    pub fn new(char_ms: u64) -> Self {
        Self {
            char_ms: char_ms.max(1),
            current: None,
            pending_entries: VecDeque::new(),
        }
    }

    pub fn set_char_ms(&mut self, char_ms: u64) {
        self.char_ms = char_ms.max(1);
    }

    pub fn is_active(&self) -> bool {
        self.current.is_some() || !self.pending_entries.is_empty()
    }

    pub fn pending_entries(&self) -> Vec<usize> {
        self.pending_entries.iter().copied().collect()
    }

    pub fn enqueue(&mut self, entry_index: usize) {
        if self.current.is_none() {
            self.current = Some(TranscriptAnimation {
                entry_index,
                started_at: Instant::now(),
            });
        } else {
            self.pending_entries.push_back(entry_index);
        }
    }

    pub fn snapshot(&self, transcript: &[String]) -> Option<TranscriptAnimationSnapshot> {
        let animation = self.current.as_ref()?;
        let entry = transcript.get(animation.entry_index)?;
        let total_chars = Self::visible_char_count(entry);
        let elapsed_ms = animation.started_at.elapsed().as_millis() as u64;
        let visible_chars = ((elapsed_ms / self.char_ms) as usize).min(total_chars);
        Some(TranscriptAnimationSnapshot {
            entry_index: animation.entry_index,
            visible_chars,
        })
    }

    pub fn advance(&mut self, transcript: &[String]) {
        let Some(animation) = self.current.as_ref() else {
            return;
        };
        let Some(entry) = transcript.get(animation.entry_index) else {
            self.current = None;
            return;
        };
        let total_chars = Self::visible_char_count(entry) as u64;
        let duration_ms = total_chars.saturating_mul(self.char_ms);
        if animation.started_at.elapsed().as_millis() as u64 >= duration_ms {
            self.current =
                self.pending_entries
                    .pop_front()
                    .map(|entry_index| TranscriptAnimation {
                        entry_index,
                        started_at: Instant::now(),
                    });
        }
    }

    pub fn visible_char_count(entry: &str) -> usize {
        entry.chars().filter(|character| *character != '\n').count()
    }
}

pub struct TimedTextFrame {
    pub text: String,
    pub duration: Duration,
}

pub struct TimedTextPlayback {
    title: String,
    frames: Vec<TimedTextFrame>,
    frame_index: usize,
    next_frame_at: Instant,
    reveal_effect: Effect,
    finished: bool,
}

impl TimedTextPlayback {
    pub fn new(
        title: String,
        mut frames: Vec<TimedTextFrame>,
        reveal_effect: Effect,
        fallback_frame_duration: Duration,
    ) -> Self {
        if frames.is_empty() {
            frames.push(TimedTextFrame {
                text: String::new(),
                duration: fallback_frame_duration,
            });
        }
        let first_duration = frames[0].duration;
        Self {
            title,
            frames,
            frame_index: 0,
            next_frame_at: Instant::now() + first_duration,
            reveal_effect,
            finished: false,
        }
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn current_text(&self) -> &str {
        self.frames
            .get(self.frame_index)
            .map(|frame| frame.text.as_str())
            .unwrap_or_default()
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn reveal_effect_mut(&mut self) -> &mut Effect {
        &mut self.reveal_effect
    }

    pub fn advance(&mut self, now: Instant) {
        if self.finished || self.frames.is_empty() {
            self.finished = true;
            return;
        }
        if now < self.next_frame_at {
            return;
        }
        if self.frame_index + 1 < self.frames.len() {
            self.frame_index += 1;
            self.next_frame_at = now + self.frames[self.frame_index].duration;
        } else {
            self.finished = true;
        }
    }

    pub fn finish(&mut self) {
        if !self.frames.is_empty() {
            self.frame_index = self.frames.len() - 1;
        }
        self.finished = true;
    }
}
