pub mod content;
pub mod engine;

pub use content::loader;
pub use content::types::{
    ContentPack, OpeningMovieDefinition, ShellMenuItem, UiTextDefinition,
};
pub use engine::runtime::{
    CinderRuntime, FinalChapterSummary, MenuChoiceOption, ReportCardData, ReportCardEntry,
};
pub use engine::state::TurnOutcome;
