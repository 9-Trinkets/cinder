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
}

/// A single line of narrative output, tagged with how it should be styled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NarrativeLine {
    pub kind: NarrativeLineKind,
    pub text: String,
}

impl NarrativeLine {
    pub fn narration(text: impl Into<String>) -> Self {
        Self {
            kind: NarrativeLineKind::Narration,
            text: text.into(),
        }
    }

    pub fn heading(text: impl Into<String>) -> Self {
        Self {
            kind: NarrativeLineKind::Heading,
            text: text.into(),
        }
    }

    pub fn player(text: impl Into<String>) -> Self {
        Self {
            kind: NarrativeLineKind::Player,
            text: text.into(),
        }
    }

    pub fn error(text: impl Into<String>) -> Self {
        Self {
            kind: NarrativeLineKind::Error,
            text: text.into(),
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

    /// Extends from a stream of plain strings, each becoming narration.
    pub fn extend_narration<I: IntoIterator<Item = String>>(&mut self, iter: I) {
        self.0
            .extend(iter.into_iter().map(NarrativeLine::narration));
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
