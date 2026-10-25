use super::super::context::ReduceCtx;
use super::super::handlers::{
    ActorObservationContext, handle_actor_observed, handle_actor_observed_actor,
    handle_actor_observed_feature, handle_actor_observed_room, handle_current_room_observed,
    handle_feature_observed, handle_interaction_used, handle_item_observed,
};
use crate::engine::events::WorldEvent;
use crate::engine::state::GamePhase;

pub(super) fn dispatch(ctx: &mut ReduceCtx<'_>, event: &WorldEvent) -> bool {
    match event {
        WorldEvent::CurrentRoomObserved { room_id, mode } => {
            if ctx.state.phase != GamePhase::Active {
                return false;
            }
            let insert_at = ctx.room_entry_line_start.take();
            handle_current_room_observed(
                &mut *ctx.state,
                ctx.content,
                room_id,
                mode.clone(),
                &mut *ctx.lines,
                insert_at,
            );
            true
        }
        WorldEvent::FeatureObserved {
            room_id,
            feature_id,
        } => {
            handle_feature_observed(ctx, room_id, feature_id);
            true
        }
        WorldEvent::ActorObserved { actor_id } => {
            handle_actor_observed(ctx, actor_id);
            true
        }
        WorldEvent::ActorObservedRoom {
            actor_id,
            actor_name,
            room_id,
        } => {
            handle_actor_observed_room(ctx, actor_id, actor_name, room_id);
            true
        }
        WorldEvent::ActorObservedFeature {
            actor_id,
            actor_name,
            room_id,
            feature_id,
        } => {
            handle_actor_observed_feature(ctx, actor_id, actor_name, room_id, feature_id);
            true
        }
        WorldEvent::ActorObservedActor {
            actor_id,
            actor_name,
            room_id,
            target_actor_id,
            target_actor_name,
        } => {
            handle_actor_observed_actor(
                ctx,
                room_id,
                ActorObservationContext {
                    actor_id,
                    actor_name,
                    target_actor_id,
                    target_actor_name,
                },
            );
            true
        }
        WorldEvent::ItemObserved { item_id } => {
            handle_item_observed(ctx, item_id);
            true
        }
        WorldEvent::InteractionUsed {
            owner,
            interaction_id,
        } => {
            handle_interaction_used(ctx, owner, interaction_id);
            true
        }
        _ => false,
    }
}
