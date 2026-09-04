use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShellMenuDefinition {
    #[serde(default)]
    pub items: Vec<ShellMenuItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellMenuItem {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub children: Vec<ShellMenuItem>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActClosureDefinition {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub subtitle_template: String,
    #[serde(default)]
    pub sections: Vec<ActClosureSectionDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActClosureSectionDefinition {
    #[serde(default)]
    pub title: String,
    pub source: ActClosureSource,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActClosureSource {
    PerspectiveRating,
    PerspectiveReview,
    RelationshipSummary,
    ContinuationPreview,
    #[default]
    TranscriptHighlights,
}
