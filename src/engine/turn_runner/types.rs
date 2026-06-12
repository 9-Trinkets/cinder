use crate::engine::commands::PlayerCommand;
use crate::engine::dialogue::DialogueRequest;
use crate::engine::events::WorldEvent;
use crate::engine::state::WorldSnapshot;
use crate::engine::state::WorldState;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct RouteEnvelope {
    pub(super) next: String,
    pub(super) message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct TurnRequest {
    pub(super) raw_input: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SignalEnvelope {
    pub(super) window_id: String,
    pub(super) signal_type: String,
    pub(super) source: String,
    pub(super) payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct CommandSignal {
    pub(super) raw_input: String,
    pub(super) command: PlayerCommand,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct AggregatedTurn {
    pub(super) command: CommandSignal,
    pub(super) world: WorldSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct PlannedTurn {
    pub(super) events: Vec<WorldEvent>,
    pub(super) pending_dialogue: Option<PendingDialogue>,
    pub(super) grounded_dialogue: Option<DialogueRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct PendingDialogue {
    pub(super) actor_id: String,
    pub(super) current_room_id: String,
    pub(super) raw_input: String,
    pub(super) other_person_id: String,
    pub(super) other_person_name: String,
    pub(super) other_person_message: Option<String>,
    pub(super) turn_number: u32,
}

pub(super) fn parse_aggregated_turn(inbound: &str) -> Result<AggregatedTurn, String> {
    let payload: Value = serde_json::from_str(inbound).map_err(|error| error.to_string())?;
    let signals = payload
        .get("signals")
        .and_then(Value::as_object)
        .ok_or_else(|| "missing aggregated signals".to_string())?;
    let command: CommandSignal = serde_json::from_value(
        signals
            .get("command")
            .and_then(|value| value.get("payload"))
            .cloned()
            .ok_or_else(|| "missing command payload".to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let world: WorldSnapshot = serde_json::from_value(
        signals
            .get("world")
            .and_then(|value| value.get("payload"))
            .cloned()
            .ok_or_else(|| "missing world payload".to_string())?,
    )
    .map_err(|error| error.to_string())?;
    Ok(AggregatedTurn { command, world })
}

pub(super) fn next_turn_id(state: &Arc<Mutex<WorldState>>) -> Result<u32, String> {
    state
        .lock()
        .map(|state| state.turn_number + 1)
        .map_err(|_| "failed to lock state for next turn".to_string())
}

pub(super) fn extract_inbound_message(prompt: &str) -> Result<String, String> {
    let (marker, json_encoded) = if prompt.contains("INBOUND_MESSAGE_JSON:\n") {
        ("INBOUND_MESSAGE_JSON:\n", true)
    } else {
        ("INBOUND_MESSAGE:\n", false)
    };
    let start = prompt
        .find(marker)
        .ok_or_else(|| "missing INBOUND_MESSAGE block".to_string())?
        + marker.len();
    let rest = &prompt[start..];
    let end = rest
        .find("\n\nROUTING_PROTOCOL:")
        .ok_or_else(|| "missing ROUTING_PROTOCOL block".to_string())?;
    let inbound = &rest[..end];
    if json_encoded {
        serde_json::from_str(inbound).map_err(|error| error.to_string())
    } else {
        Ok(inbound.to_string())
    }
}
