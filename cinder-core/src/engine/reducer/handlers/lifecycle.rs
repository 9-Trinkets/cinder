use crate::engine::reducer::beat_advance::{advance_objective_for_signal, time_reached_signals};
use crate::engine::reducer::tick::{
    advance_actor_stats_on_tick, advance_house_progress_objectives,
    advance_stat_threshold_objectives, increment_shared_room_safety,
};
use crate::content::types::ContentPack;
use crate::engine::narrative::NarrativeLines;
use crate::engine::state::WorldState;
pub(crate) fn handle_turn_started(
    state: &mut WorldState,
    content: &ContentPack,
    turn_number: u32,
    _raw_input: &str,
    advances_time: bool,
    lines: &mut NarrativeLines,
) {
    state.turn_number = turn_number;
    if advances_time {
        increment_shared_room_safety(state, content);
        state.clear_stale_pending_replies();
        let previous_time_minutes = state.current_time_minutes;
        state.current_time_minutes += content.settings.tick_minutes_per_turn;
        advance_actor_stats_on_tick(
            state,
            content,
            previous_time_minutes,
            state.current_time_minutes,
        );
        for signal in time_reached_signals(previous_time_minutes, state.current_time_minutes) {
            lines.extend_narration(advance_objective_for_signal(state, content, &signal));
        }
        lines.extend_narration(advance_house_progress_objectives(state, content));
        lines.extend_narration(advance_stat_threshold_objectives(state, content));
    }
    // Strike policy is external (tick workflows, rules or LLM); the reducer
    // only resolves declared HostileStrike events.
}

/// Generic mechanics for a declared hostile strike: validates eligibility,
/// applies stat-based damage, emits the pack-authored narration, and reschedules
/// the actor's cooldown. Strike *policy* (who strikes, when) lives in tick
/// behaviors, never here.
/// First living, following actor with the `guard` role in the room, if any.
pub(crate) fn handle_act_ended(
    state: &mut WorldState,
    _content: &ContentPack,
    _lines: &mut NarrativeLines,
) {
    state.phase = crate::engine::state::GamePhase::ActEnded;
}