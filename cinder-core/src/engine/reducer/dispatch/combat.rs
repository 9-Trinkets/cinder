use super::super::combat::handle_periodic_actor_effect_applied;
use super::super::context::ReduceCtx;
use super::super::handlers::{
    handle_hostile_heal, handle_hostile_strike, handle_pair_stat_adjusted,
};
use crate::engine::events::WorldEvent;

pub(super) fn dispatch(ctx: &mut ReduceCtx<'_>, event: &WorldEvent) -> bool {
    match event {
        WorldEvent::ActorStatAdjusted {
            actor_id,
            stat,
            delta,
        } => {
            if let Err(e) = ctx
                .state
                .adjust_actor_stat(ctx.content, actor_id, stat, *delta)
            {
                eprintln!("[cinder] ActorStatAdjusted error: {e}");
            }
            true
        }
        WorldEvent::SkillSpAwarded {
            actor_id,
            skill_id,
            sp,
        } => {
            ctx.state.award_skill_sp(actor_id, skill_id, *sp);
            true
        }
        WorldEvent::PairStatAdjusted {
            participant_a_id,
            participant_b_id,
            stat,
            delta,
        } => {
            handle_pair_stat_adjusted(ctx, participant_a_id, participant_b_id, stat, *delta);
            true
        }
        WorldEvent::HostileStrike { actor_id } => {
            handle_hostile_strike(ctx, actor_id);
            true
        }
        WorldEvent::HostileHeal {
            actor_id,
            target_id,
            amount,
            message,
        } => {
            handle_hostile_heal(ctx, actor_id, target_id, *amount, message);
            true
        }
        WorldEvent::PeriodicActorEffectApplied {
            actor_id,
            effect_id,
        } => {
            handle_periodic_actor_effect_applied(ctx, actor_id, effect_id);
            true
        }
        _ => false,
    }
}
