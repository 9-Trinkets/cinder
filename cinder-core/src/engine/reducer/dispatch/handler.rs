//! Domain dispatcher trait.
//!
//! Each domain module owns a slice of the `WorldEvent` space. Implementing
//! [`DomainDispatcher`] exposes that slice through one uniform interface so
//! [`super::dispatch`] can drive a registry instead of hard-coding the order
//! with a chain of `if` statements.

use super::super::context::ReduceCtx;
use crate::engine::events::WorldEvent;

pub trait DomainDispatcher: Sync {
    /// Handle `event` if this domain owns it. Returns `true` when handled, so
    /// the registry can stop at the first matching domain.
    fn dispatch(&self, ctx: &mut ReduceCtx<'_>, event: &WorldEvent) -> bool;
}
