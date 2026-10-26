use crate::transcript::renderer::event::{EventFields, EventRenderer};
use crate::transcript::renderer::state::RenderState;
use serde_json::Value;

/// Captures the dialogue request emitted before an actor chooses its turn.
pub struct ActorTurnRequestRenderer;

impl EventRenderer for ActorTurnRequestRenderer {
    fn matches(&self, fields: &EventFields<'_>) -> bool {
        fields.role == "actor_turn_decider" && fields.topic == "model.request"
    }

    fn render(&self, state: &mut RenderState, fields: &EventFields<'_>) {
        let request = fields
            .payload
            .get("dialogue_request")
            .cloned()
            .unwrap_or(Value::Null);
        let actor_id = fields
            .payload
            .get("actor_id")
            .or_else(|| request.get("actor_id"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if !actor_id.is_empty() {
            state.turn_requests.insert(actor_id.to_string(), request);
        }
    }
}

/// Renders the actor's chosen MOVE/ACT decision, resolving the request
/// captured by [`ActorTurnRequestRenderer`] when a destination title is
/// needed.
pub struct ActorTurnResponseRenderer;

impl EventRenderer for ActorTurnResponseRenderer {
    fn matches(&self, fields: &EventFields<'_>) -> bool {
        fields.role == "actor_turn_decider" && fields.topic == "model.response"
    }

    fn render(&self, state: &mut RenderState, fields: &EventFields<'_>) {
        let actor_id = fields
            .payload
            .get("actor_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let actor_name = fields
            .payload
            .get("actor_name")
            .and_then(|v| v.as_str())
            .unwrap_or(actor_id);
        let decision = fields
            .payload
            .get("decision")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        let request = state.turn_requests.get(actor_id);

        if decision == "MOVE" || decision.starts_with("MOVE ") {
            if let Some(room_title) = request
                .and_then(|r| r.get("move_target_room_title"))
                .and_then(|v| v.as_str())
            {
                state.push_unique(format!("{actor_name} heads to the {room_title}."));
            }
        } else if let Some(stripped) = decision.strip_prefix("ACT") {
            let action = stripped.trim_start_matches([' ', ':', '-', '—']).trim();
            if !action.is_empty() {
                let mut norm = action.trim();
                if let Some(rest) = norm.strip_prefix(actor_name) {
                    norm = rest.trim_start();
                }
                state.push_unique(format!(
                    "{actor_name} {}.",
                    norm.trim_end_matches(['.', '!', '?'])
                ));
            }
        }
    }
}
