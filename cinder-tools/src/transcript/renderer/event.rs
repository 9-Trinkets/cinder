use super::state::RenderState;
use serde_json::Value;

/// The identifying fields of a single trace event.
pub struct EventFields<'a> {
    pub role: &'a str,
    pub topic: &'a str,
    pub payload: &'a Value,
}

/// Renders one kind of trace event into transcript lines.
///
/// Implementations are stateless: all accumulation happens in [`RenderState`],
/// so a renderer can be shared from a static registry.
pub trait EventRenderer: Sync {
    /// Whether this renderer owns the event described by `fields`.
    fn matches(&self, fields: &EventFields<'_>) -> bool;

    /// Render `fields` into `state`.
    fn render(&self, state: &mut RenderState, fields: &EventFields<'_>);
}
