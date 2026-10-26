use crate::transcript::renderer::event::{EventFields, EventRenderer};
use crate::transcript::renderer::state::RenderState;

/// Renders the final `workflow.complete` block, splitting it into lines and
/// retaining the raw text for `--raw` output.
pub struct WorkflowCompleteRenderer;

impl EventRenderer for WorkflowCompleteRenderer {
    fn matches(&self, fields: &EventFields<'_>) -> bool {
        fields.topic == "workflow.complete"
    }

    fn render(&self, state: &mut RenderState, fields: &EventFields<'_>) {
        let Some(text) = fields.payload.get("text").and_then(|v| v.as_str()) else {
            return;
        };
        state.raw_blocks.push(text.to_string());
        for line in text.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                state.push_unique(trimmed.to_string());
            }
        }
    }
}
