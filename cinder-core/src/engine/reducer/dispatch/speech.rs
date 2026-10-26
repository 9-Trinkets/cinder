use super::super::context::ReduceCtx;
use super::super::handlers::handle_channel_message;
use crate::engine::events::WorldEvent;

pub(super) fn dispatch(ctx: &mut ReduceCtx<'_>, event: &WorldEvent) -> bool {
    match event {
        WorldEvent::ChannelMessage { message } => {
            handle_channel_message(ctx, message);
            true
        }
        _ => false,
    }
}

pub(super) struct SpeechDispatcher;

impl super::handler::DomainDispatcher for SpeechDispatcher {
    fn dispatch(&self, ctx: &mut ReduceCtx<'_>, event: &WorldEvent) -> bool {
        dispatch(ctx, event)
    }
}
