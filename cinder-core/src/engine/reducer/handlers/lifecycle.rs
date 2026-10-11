use super::super::context::ReduceCtx;
use crate::engine::reducer::beat_advance::{advance_objective_for_signal, time_reached_signals};
use crate::engine::reducer::tick::{
    advance_actor_stats_on_tick, advance_house_progress_objectives,
    advance_stat_threshold_objectives, increment_shared_room_safety,
};
pub(crate) fn handle_turn_started(
    ctx: &mut ReduceCtx<'_>,
    turn_number: u32,
    _raw_input: &str,
    advances_time: bool,
) {
    let state = &mut *ctx.state;
    let content = ctx.content;
    let suppressed_actor_stats = ctx.suppressed_actor_stats;
    let lines = &mut *ctx.lines;
    state.clock.turn_number = turn_number;
    if advances_time {
        increment_shared_room_safety(state, content);
        state.clear_stale_pending_replies();
        let previous_time_minutes = state.clock.current_time_minutes;
        state.clock.current_time_minutes += content.settings.tick_minutes_per_turn;
        advance_actor_stats_on_tick(
            state,
            content,
            previous_time_minutes,
            state.clock.current_time_minutes,
            suppressed_actor_stats,
        );
        for signal in time_reached_signals(previous_time_minutes, state.clock.current_time_minutes)
        {
            lines.extend_lines(advance_objective_for_signal(state, content, &signal));
        }
        lines.extend_lines(advance_house_progress_objectives(state, content));
        lines.extend_lines(advance_stat_threshold_objectives(state, content));
        if let Some(until) = state.combat.invisible_until_minute {
            if state.clock.current_time_minutes >= until {
                state.combat.invisible_until_minute = None;
                state.story_vars.set_unchecked("is_invisible", "false");
                lines.narration("Your invisibility wears off. You fade back into view.");
            }
        }
    }
    // Strike policy is external (tick workflows, rules or LLM); the reducer
    // only resolves declared HostileStrike events.
}

/// Generic mechanics for a declared hostile strike: validates eligibility,
/// applies stat-based damage, emits the pack-authored narration, and reschedules
/// the actor's cooldown. Strike *policy* (who strikes, when) lives in tick
/// behaviors, never here.
/// First living, following actor with the `guard` role in the room, if any.
pub(crate) fn handle_act_ended(ctx: &mut ReduceCtx<'_>) {
    let state = &mut *ctx.state;
    state.phase = crate::engine::state::GamePhase::ActEnded;
}
