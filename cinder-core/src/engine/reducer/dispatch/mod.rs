//! Domain-split event dispatch.
//!
//! The reducer used to funnel every `WorldEvent` through one ~40-arm `match`,
//! which made a single function carry the whole dispatch surface. Each domain
//! module below owns a small, cohesive slice of the event space and reports
//! whether it handled the event; [`dispatch`] tries them in order and stops at
//! the first match. Adding a new event variant means extending exactly one
//! domain's arm list rather than widening a monolith.

use super::context::ReduceCtx;
use crate::engine::events::WorldEvent;

mod combat;
mod content;
mod feedback;
mod items;
mod lifecycle;
mod menu;
mod movement;
mod observation;
mod party;
mod speech;

/// Route a single event to the domain dispatcher that owns it.
pub(crate) fn dispatch(ctx: &mut ReduceCtx<'_>, event: &WorldEvent) {
    if lifecycle::dispatch(ctx, event) {
        return;
    }
    if feedback::dispatch(ctx, event) {
        return;
    }
    if observation::dispatch(ctx, event) {
        return;
    }
    if movement::dispatch(ctx, event) {
        return;
    }
    if combat::dispatch(ctx, event) {
        return;
    }
    if items::dispatch(ctx, event) {
        return;
    }
    if menu::dispatch(ctx, event) {
        return;
    }
    if speech::dispatch(ctx, event) {
        return;
    }
    if party::dispatch(ctx, event) {
        return;
    }
    content::dispatch(ctx, event);
}
