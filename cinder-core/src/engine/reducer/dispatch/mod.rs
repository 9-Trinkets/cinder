//! Domain-split event dispatch.
//!
//! The reducer used to funnel every `WorldEvent` through one ~40-arm `match`,
//! which made a single function carry the whole dispatch surface. Each domain
//! module below owns a small, cohesive slice of the event space and reports
//! whether it handled the event. [`dispatch`] walks a registry of those
//! domains in order and stops at the first match. Adding a new event variant
//! means extending exactly one domain's arm list rather than widening a
//! monolith; adding a new domain means registering one dispatcher here.

use super::context::ReduceCtx;
use crate::engine::events::WorldEvent;

mod combat;
mod content;
mod feedback;
pub mod handler;
mod items;
mod lifecycle;
mod menu;
mod movement;
mod observation;
mod party;
mod speech;

use handler::DomainDispatcher;

/// Ordered dispatch registry. Order matters: earlier domains claim their
/// events before later, broader ones. `content` is last because it owns the
/// residual actor-command and content-event space.
static REGISTRY: &[&dyn DomainDispatcher] = &[
    &lifecycle::LifecycleDispatcher,
    &feedback::FeedbackDispatcher,
    &observation::ObservationDispatcher,
    &movement::MovementDispatcher,
    &combat::CombatDispatcher,
    &items::ItemsDispatcher,
    &menu::MenuDispatcher,
    &speech::SpeechDispatcher,
    &party::PartyDispatcher,
    &content::ContentDispatcher,
];

/// Route a single event to the first domain dispatcher that claims it.
pub(crate) fn dispatch(ctx: &mut ReduceCtx<'_>, event: &WorldEvent) {
    for dispatcher in REGISTRY {
        if dispatcher.dispatch(ctx, event) {
            return;
        }
    }
}
