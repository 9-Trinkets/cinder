use super::super::context::ReduceCtx;
use super::super::handlers::handle_party_order_assigned;
use crate::engine::events::WorldEvent;

pub(super) fn dispatch(ctx: &mut ReduceCtx<'_>, event: &WorldEvent) -> bool {
    match event {
        WorldEvent::PartyOrderAssigned { actor_id, order } => {
            handle_party_order_assigned(ctx, actor_id, (*order).clone());
            true
        }
        _ => false,
    }
}

pub(super) struct PartyDispatcher;

impl super::handler::DomainDispatcher for PartyDispatcher {
    fn dispatch(&self, ctx: &mut ReduceCtx<'_>, event: &WorldEvent) -> bool {
        dispatch(ctx, event)
    }
}
