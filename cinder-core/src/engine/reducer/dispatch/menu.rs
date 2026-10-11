use super::super::context::ReduceCtx;
use super::super::handlers::{
    handle_menu_choice_made, handle_menu_opened, handle_menu_selection_toggled,
};
use crate::engine::events::WorldEvent;

pub(super) fn dispatch(ctx: &mut ReduceCtx<'_>, event: &WorldEvent) -> bool {
    match event {
        WorldEvent::MenuOpened { menu_id } => {
            handle_menu_opened(&mut *ctx.state, ctx.content, menu_id, &mut *ctx.lines);
            true
        }
        WorldEvent::MenuChoiceMade {
            menu_id,
            option_id,
            title,
        } => {
            handle_menu_choice_made(ctx, menu_id, option_id, title);
            true
        }
        WorldEvent::MenuSelectionToggled {
            menu_id,
            option_id,
            selected,
        } => {
            handle_menu_selection_toggled(ctx, menu_id, option_id, *selected);
            true
        }
        _ => false,
    }
}
