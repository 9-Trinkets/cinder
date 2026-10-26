use crate::transcript::renderer::event::{EventFields, EventRenderer};
use crate::transcript::renderer::state::RenderState;

/// Renders a line of NPC or player dialogue.
pub struct ActorDialogueRenderer;

impl EventRenderer for ActorDialogueRenderer {
    fn matches(&self, fields: &EventFields<'_>) -> bool {
        fields.role == "actor_dialogue" && fields.topic == "model.response"
    }

    fn render(&self, state: &mut RenderState, fields: &EventFields<'_>) {
        let actor_name = fields
            .payload
            .get("actor_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let resp = fields
            .payload
            .get("response_text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        if !actor_name.is_empty() && !resp.is_empty() {
            state.push_unique(format!("{actor_name}: {resp}"));
        }
    }
}
