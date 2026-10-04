use serde::{Deserialize, Serialize};

/// How a narrative line should be presented. Styling is decided here, in the
/// engine, rather than inferred by the client from text formatting, so the
/// transcript never has to parse room headings or error prefixes out of prose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NarrativeLineKind {
    /// Default prose.
    #[default]
    Narration,
    /// A scene/room heading (e.g. `== Lounge ==`).
    Heading,
    /// The player's own echoed command (`> place marker`).
    Player,
    /// A system/error feedback line.
    Error,
    /// A cold, system-voice teaching line (e.g. sigil instructions at the
    /// start of a level). Distinct from prose and from error feedback.
    System,
    /// A remote comms message (e.g. the handler's radio check-ins over the
    /// handler-comms channel). Distinct from spoken-in-room dialogue.
    Channel,
}

/// A single line of narrative output, tagged with how it should be styled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NarrativeLine {
    pub kind: NarrativeLineKind,
    pub text: String,
    /// Declared by a pack through the `narrate_message` hook effect's optional
    /// `generate_commentary` flag. Marks the line for a post-reduce upgrade:
    /// the fallback text is replaced with generated commentary drawn from a
    /// pack-authored prompt template. Never serialized: it is a transient
    /// in-process signal between the reducer and the turn runner.
    #[serde(default, skip_serializing)]
    pub pending_commentary_upgrade: Option<PendingCommentaryUpgrade>,
    /// Pending upgrade for autonomous offscreen combat dispatches. Replaced
    /// with an in-character LLM voice message during turn runner execution.
    #[serde(default, skip_serializing)]
    pub pending_comms_upgrade: Option<PendingCommsUpgrade>,
}

/// Context a flagged `narrate_message` line carries into the transition
/// commentary upgrade pass. Rooms come from the hook input (movement events),
/// so the engine never hardcodes which areas the pack considers milestones.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingCommentaryUpgrade {
    pub from_room_id: String,
    pub to_room_id: String,
    /// The rendered pack message that becomes the fallback when generation is
    /// unavailable or the pack's template asks for it.
    pub fallback_text: String,
}

/// High-level combat milestone reported over tactical comms by an offscreen party member.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommsMilestone {
    Contact,
    LowHealth,
    AllyDown,
    AreaCleared,
    PeriodicStatus,
}

/// Context an offscreen comms dispatch line carries into the character voice generation pass.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingCommsUpgrade {
    pub reporter_id: String,
    pub reporter_name: String,
    pub room_id: String,
    pub room_name: String,
    pub milestone: CommsMilestone,
    pub fallback_text: String,
    pub enemies_remaining: usize,
    pub ally_names: Vec<String>,
}

impl NarrativeLine {
    pub fn narration(text: impl Into<String>) -> Self {
        Self {
            kind: NarrativeLineKind::Narration,
            text: text.into(),
            pending_commentary_upgrade: None,
            pending_comms_upgrade: None,
        }
    }

    pub fn heading(text: impl Into<String>) -> Self {
        Self {
            kind: NarrativeLineKind::Heading,
            text: text.into(),
            pending_commentary_upgrade: None,
            pending_comms_upgrade: None,
        }
    }

    pub fn player(text: impl Into<String>) -> Self {
        Self {
            kind: NarrativeLineKind::Player,
            text: text.into(),
            pending_commentary_upgrade: None,
            pending_comms_upgrade: None,
        }
    }

    pub fn error(text: impl Into<String>) -> Self {
        Self {
            kind: NarrativeLineKind::Error,
            text: text.into(),
            pending_commentary_upgrade: None,
            pending_comms_upgrade: None,
        }
    }

    pub fn system(text: impl Into<String>) -> Self {
        Self {
            kind: NarrativeLineKind::System,
            text: text.into(),
            pending_commentary_upgrade: None,
            pending_comms_upgrade: None,
        }
    }

    pub fn channel(text: impl Into<String>) -> Self {
        Self {
            kind: NarrativeLineKind::Channel,
            text: text.into(),
            pending_commentary_upgrade: None,
            pending_comms_upgrade: None,
        }
    }

    pub fn comms_dispatch(text: impl Into<String>, upgrade: PendingCommsUpgrade) -> Self {
        Self {
            kind: NarrativeLineKind::Channel,
            text: text.into(),
            pending_commentary_upgrade: None,
            pending_comms_upgrade: Some(upgrade),
        }
    }
}

impl From<String> for NarrativeLine {
    fn from(text: String) -> Self {
        Self::narration(text)
    }
}

/// Convenience collection with typed push helpers, so handlers can say
/// `lines.narration(...)` / `lines.heading(...)` without wrapping each line.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NarrativeLines(pub Vec<NarrativeLine>);

impl NarrativeLines {
    pub fn narration(&mut self, text: impl Into<String>) {
        self.0.push(NarrativeLine::narration(text));
    }

    pub fn heading(&mut self, text: impl Into<String>) {
        self.0.push(NarrativeLine::heading(text));
    }

    pub fn player(&mut self, text: impl Into<String>) {
        self.0.push(NarrativeLine::player(text));
    }

    pub fn error(&mut self, text: impl Into<String>) {
        self.0.push(NarrativeLine::error(text));
    }

    pub fn system(&mut self, text: impl Into<String>) {
        self.0.push(NarrativeLine::system(text));
    }

    pub fn channel(&mut self, text: impl Into<String>) {
        self.0.push(NarrativeLine::channel(text));
    }

    pub fn comms_dispatch(&mut self, text: impl Into<String>, upgrade: PendingCommsUpgrade) {
        self.0.push(NarrativeLine::comms_dispatch(text, upgrade));
    }

    /// Extends from a stream of plain strings, each becoming narration.
    pub fn extend_narration<I: IntoIterator<Item = String>>(&mut self, iter: I) {
        self.0
            .extend(iter.into_iter().map(NarrativeLine::narration));
    }

    /// Extends from already-typed lines, preserving each line's kind. Used by
    /// producers that decide styling themselves (the beat reducer, which emits
    /// a stage's completion message at whatever voice the pack gave it).
    pub fn extend_lines<I: IntoIterator<Item = NarrativeLine>>(&mut self, iter: I) {
        self.0.extend(iter);
    }

    /// Joins the line texts the way the turn text has historically been built.
    pub fn to_text(&self) -> String {
        self.0
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

impl std::ops::Deref for NarrativeLines {
    type Target = Vec<NarrativeLine>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for NarrativeLines {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Vec<NarrativeLine>> for NarrativeLines {
    fn from(lines: Vec<NarrativeLine>) -> Self {
        Self(lines)
    }
}
