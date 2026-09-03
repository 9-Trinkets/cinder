mod actor_turn;
mod drain;
mod hostility;
mod movement;
mod workflow;

use crate::content::types::{ActorTickScope, ContentPack};
use crate::engine::events::WorldEvent;
use crate::engine::state::WorldState;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

pub(crate) use drain::plan_drain_events;
pub(crate) use movement::{decide_movement, plan_wander_moves};
pub(crate) use workflow::run_actor_tick;

#[derive(Debug, Clone)]
pub struct ActorTickExecution {
    pub events: Vec<WorldEvent>,
    pub trace_records: Vec<ActorTraceRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(tag = "stage", content = "payload", rename_all = "snake_case")]
enum ActorTurnStageEnvelope {
    #[default]
    Idle,
    Built {
        actor_id: String,
    },
    Decided {
        actor_id: String,
        decision: crate::engine::dialogue::ActorTurnActionDecision,
    },
    Realized {
        actor_id: String,
        events: Vec<WorldEvent>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(tag = "stage", content = "payload", rename_all = "snake_case")]
enum HostilityStageEnvelope {
    #[default]
    Idle,
    Decided {
        events: Vec<WorldEvent>,
    },
    Applied,
}

#[derive(Debug, Clone)]
pub struct ActorTraceRecord {
    pub role_name: String,
    pub topic: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct ActorTickError {
    pub message: String,
    pub trace_records: Vec<ActorTraceRecord>,
}

impl fmt::Display for ActorTickError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for ActorTickError {}

fn tick_scope_room_ids(content: &ContentPack, state: &WorldState) -> Option<BTreeSet<String>> {
    match content.settings.actor_tick_scope {
        ActorTickScope::CurrentBoard => Some(content.reachable_room_ids(&state.current_room_id)),
        ActorTickScope::AllRooms => None,
    }
}

fn room_is_in_tick_scope(scope_room_ids: &Option<BTreeSet<String>>, room_id: &str) -> bool {
    scope_room_ids
        .as_ref()
        .is_none_or(|room_ids| room_ids.contains(room_id))
}
