use super::super::context::ReduceCtx;
use super::super::handlers::{
    handle_actor_moved, handle_actor_relocated, handle_player_followed_actor, handle_player_moved,
};
use crate::engine::events::WorldEvent;

pub(super) fn dispatch(ctx: &mut ReduceCtx<'_>, event: &WorldEvent) -> bool {
    match event {
        WorldEvent::ActorRelocated {
            actor_id,
            to_room_id,
        } => {
            handle_actor_relocated(ctx, actor_id, to_room_id);
            true
        }
        WorldEvent::ActorMoved {
            actor_id,
            from_room_id,
            to_room_id,
            ..
        } => {
            handle_actor_moved(ctx, actor_id, from_room_id, to_room_id);
            true
        }
        WorldEvent::PlayerMoved { to_room_id, .. } => {
            *ctx.room_entry_line_start = Some(handle_player_moved(
                &mut *ctx.state,
                ctx.content,
                to_room_id,
                &mut *ctx.lines,
            ));
            true
        }
        WorldEvent::PlayerFollowedActor { actor_id } => {
            handle_player_followed_actor(ctx, actor_id.as_deref());
            true
        }
        _ => false,
    }
}
