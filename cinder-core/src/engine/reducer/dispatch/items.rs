use super::super::context::ReduceCtx;
use super::super::equipment;
use super::super::handlers::{
    handle_item_acquired, handle_item_consumed, handle_item_transferred,
    handle_player_dropped_item, handle_player_took_item, handle_player_used_item,
};
use crate::content::types::ItemStorageTarget;
use crate::engine::events::WorldEvent;
use crate::engine::state::WorldState;

pub(super) fn dispatch(ctx: &mut ReduceCtx<'_>, event: &WorldEvent) -> bool {
    match event {
        WorldEvent::PlayerTookItem { item_id } => {
            handle_player_took_item(ctx, item_id);
            true
        }
        WorldEvent::PlayerDroppedItem { item_id } => {
            handle_player_dropped_item(ctx, item_id);
            true
        }
        WorldEvent::PlayerEquippedItem { item_id } => {
            equipment::apply_equip(&mut *ctx.state, ctx.content, item_id, &mut *ctx.lines);
            true
        }
        WorldEvent::PlayerUnequippedItem { item_id } => {
            equipment::apply_unequip(&mut *ctx.state, ctx.content, item_id, &mut *ctx.lines);
            true
        }
        WorldEvent::PlayerUsedItem { item_id } => {
            handle_player_used_item(&mut *ctx.state, ctx.content, item_id, &mut *ctx.lines);
            true
        }
        WorldEvent::ItemTransferred {
            item_id,
            from_actor_id,
            to_actor_id,
            initiator_actor_id,
        } => {
            handle_item_transferred(
                &mut *ctx.state,
                ctx.content,
                item_id,
                from_actor_id,
                to_actor_id,
                initiator_actor_id.as_deref(),
                &mut *ctx.lines,
            );
            true
        }
        WorldEvent::ItemAcquired { item_id, storage } => {
            handle_item_acquired(ctx, item_id, *storage);
            true
        }
        WorldEvent::ItemConsumed {
            item_id,
            storage,
            consumer_id,
            consumer_name,
        } => {
            handle_item_consumed(
                ctx,
                item_id,
                *storage,
                consumer_id.as_deref(),
                consumer_name.as_deref(),
            );
            true
        }
        WorldEvent::ChalkAnchorConsumed { room_id } => {
            handle_chalk_anchor_consumed(&mut *ctx.state, room_id);
            true
        }
        _ => false,
    }
}

fn handle_chalk_anchor_consumed(state: &mut WorldState, room_id: &str) {
    state.remove_chalk_anchor(room_id);
    state.remove_item_from_storage("teleport-sigil", ItemStorageTarget::CurrentRoom, room_id);
}
