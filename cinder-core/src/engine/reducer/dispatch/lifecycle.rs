use super::super::context::ReduceCtx;
use super::super::handlers::{handle_act_ended, handle_turn_started};
use crate::engine::events::WorldEvent;

pub(super) fn dispatch(ctx: &mut ReduceCtx<'_>, event: &WorldEvent) -> bool {
    match event {
        WorldEvent::TurnStarted {
            turn_number,
            raw_input,
            advances_time,
        } => {
            handle_turn_started(ctx, *turn_number, raw_input, *advances_time);
            true
        }
        WorldEvent::ActEnded => {
            handle_act_ended(ctx);
            true
        }
        _ => false,
    }
}

pub(super) struct LifecycleDispatcher;

impl super::handler::DomainDispatcher for LifecycleDispatcher {
    fn dispatch(&self, ctx: &mut ReduceCtx<'_>, event: &WorldEvent) -> bool {
        dispatch(ctx, event)
    }
}
