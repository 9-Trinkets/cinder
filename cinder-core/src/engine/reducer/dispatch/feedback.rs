use super::super::context::ReduceCtx;
use super::super::handlers::{
    handle_action_rejected, handle_help_shown, handle_narrative_line, handle_unknown_input,
};
use crate::engine::events::WorldEvent;

pub(super) fn dispatch(ctx: &mut ReduceCtx<'_>, event: &WorldEvent) -> bool {
    match event {
        WorldEvent::NarrativeLine { text } => {
            handle_narrative_line(ctx, text);
            true
        }
        WorldEvent::ActionRejected { message } => {
            handle_action_rejected(ctx, message);
            true
        }
        WorldEvent::HelpShown => {
            handle_help_shown(ctx);
            true
        }
        WorldEvent::UnknownInput { raw_input } => {
            handle_unknown_input(ctx, raw_input);
            true
        }
        _ => false,
    }
}
