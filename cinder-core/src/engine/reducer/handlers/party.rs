use crate::content::types::{ContentPack, PartyOrderKind};
use crate::engine::narrative::NarrativeLines;
use crate::engine::state::WorldState;

use super::super::command_effects::actor_display_name;
use super::feedback::push_message;

pub(crate) fn handle_party_order_assigned(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    order: PartyOrderKind,
    lines: &mut NarrativeLines,
) {
    let key = format!("party.order_{order}_assigned");
    if let Err(error) = state.assign_party_order(content, actor_id, order) {
        eprintln!("[cinder] party order error: {error}");
        return;
    }
    let actor = actor_display_name(content, actor_id);
    push_message(lines, content, &key, &[("actor", actor.as_str())]);
}
