mod actor_turn;
mod dialogue;
mod workflow;

pub use actor_turn::{ActorTurnRequestRenderer, ActorTurnResponseRenderer};
pub use dialogue::ActorDialogueRenderer;
pub use workflow::WorkflowCompleteRenderer;

use super::event::EventRenderer;

/// Ordered renderers. The first one that claims an event wins; specific
/// `(role, topic)` pairs come before the broad `workflow.complete` match.
pub static RENDERERS: &[&dyn EventRenderer] = &[
    &ActorTurnRequestRenderer,
    &ActorTurnResponseRenderer,
    &ActorDialogueRenderer,
    &WorkflowCompleteRenderer,
];
