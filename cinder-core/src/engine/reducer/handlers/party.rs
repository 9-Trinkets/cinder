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
    if let Err(error) = state.assign_party_order(content, actor_id, order) {
        eprintln!("[cinder] party order error: {error}");
        return;
    }
    let actor = actor_display_name(content, actor_id);
    let key = match order {
        PartyOrderKind::Guard => "party.order_guard_assigned",
        PartyOrderKind::Assist => "party.order_assist_assigned",
    };
    push_message(lines, content, key, &[("actor", actor.as_str())]);
}
