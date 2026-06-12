use crate::engine::state::WorldState;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize)]
pub(super) struct StatTraceSnapshot {
    turn_number: u32,
    current_time_minutes: u32,
    actor_stats: Vec<ActorStatsTrace>,
    pair_stats: Vec<PairStatsTrace>,
}

#[derive(Debug, Clone, Serialize)]
struct ActorStatsTrace {
    actor_id: String,
    stats: BTreeMap<String, i32>,
}

#[derive(Debug, Clone, Serialize)]
struct PairStatsTrace {
    participant_a_id: String,
    participant_b_id: String,
    stats: BTreeMap<String, i32>,
}

pub(super) fn stats_trace_snapshot(state: &WorldState) -> StatTraceSnapshot {
    let actor_stats = state
        .actor_stats
        .iter()
        .map(|(actor_id, stats)| ActorStatsTrace {
            actor_id: actor_id.clone(),
            stats: stats.clone(),
        })
        .collect::<Vec<_>>();
    let pair_stats = state
        .pair_stats
        .iter()
        .filter_map(|(key, stats)| {
            let (participant_a_id, participant_b_id) = WorldState::conversation_participants(key)?;
            Some(PairStatsTrace {
                participant_a_id: participant_a_id.to_string(),
                participant_b_id: participant_b_id.to_string(),
                stats: stats.clone(),
            })
        })
        .collect::<Vec<_>>();
    StatTraceSnapshot {
        turn_number: state.turn_number,
        current_time_minutes: state.current_time_minutes,
        actor_stats,
        pair_stats,
    }
}
