mod actor_commands;
pub(crate) mod beat_advance;
pub mod combat;
mod command_effects;
mod context;
mod dispatch;
mod equipment;
pub(crate) mod handlers;
mod movement;
mod observation;
mod protection;
pub mod skills;
mod summaries;
mod surround;
mod tick;
pub mod transformations;

pub(crate) use self::handlers::handler_attributed_line;
pub(crate) use self::observation::render_actor_speech_line;
pub use self::skills::award_actor_skill_sp;

use self::context::ReduceCtx;
use crate::content::types::ContentPack;
use crate::engine::events::{TimestampedWorldEvent, WorldEvent};
use crate::engine::narrative::NarrativeLines;
use crate::engine::state::{GamePhase, WorldState};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReducerOutput {
    pub lines: NarrativeLines,
    pub phase: GamePhase,
}

fn collect_suppressed_actor_stats(events: &[TimestampedWorldEvent]) -> Vec<(&str, &str)> {
    events
        .iter()
        .filter_map(|e| match &e.event {
            WorldEvent::ActorStatAdjusted {
                actor_id,
                stat,
                delta,
            } if *delta < 0 => Some((actor_id.as_str(), stat.as_str())),
            _ => None,
        })
        .collect()
}

pub fn apply_events(
    state: &mut WorldState,
    content: &ContentPack,
    events: &[TimestampedWorldEvent],
) -> ReducerOutput {
    let mut lines = NarrativeLines::default();
    let suppressed_actor_stats = collect_suppressed_actor_stats(events);
    // Queue-based so handlers can spawn follow-up events (e.g. relationship
    // updates triggered by command effects) that are applied in the same pass.
    let mut pending: VecDeque<TimestampedWorldEvent> = events.iter().cloned().collect();
    let mut spawned_events: Vec<WorldEvent> = Vec::new();
    let mut room_entry_line_start: Option<usize> = None;
    while let Some(entry) = pending.pop_front() {
        spawned_events.clear();
        {
            let mut ctx = ReduceCtx {
                state: &mut *state,
                content,
                lines: &mut lines,
                suppressed_actor_stats: &suppressed_actor_stats,
                outbox: &mut spawned_events,
                room_entry_line_start: &mut room_entry_line_start,
            };
            dispatch::dispatch(&mut ctx, &entry.event);
        }
        for event in spawned_events.drain(..) {
            pending.push_back(TimestampedWorldEvent::now(event));
        }
    }
    // Transformation triggers are evaluated once the whole batch has settled, so
    // a stage keyed on a story variable or stat applies in the same turn the
    // condition became true, whether or not the actor auto-equipped anything.
    transformations::apply_pending_transformations(state, content, &mut lines);
    lines.extend_lines(beat_advance::route_stopped_spawn_schedules(state, content));
    protection::apply_active_protection_rules(state, content, &mut lines);
    lines.extend_lines(tick::advance_story_var_objectives(state, content));
    if !events.is_empty()
        && !events
            .iter()
            .all(|e| matches!(e.event, WorldEvent::TurnStarted { .. }))
    {
        combat::evaluate_offscreen_combat_dispatches(state, content, &mut lines);
    }
    ReducerOutput {
        lines,
        phase: state.phase.clone(),
    }
}
