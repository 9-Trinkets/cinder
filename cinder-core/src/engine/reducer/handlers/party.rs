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

pub(crate) fn handle_player_followed_actor(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: Option<&str>,
    lines: &mut NarrativeLines,
) {
    state.followed_actor_id = actor_id.map(ToString::to_string);
    let feedback_text = match actor_id {
        Some(id) => {
            let actor_name = content
                .actor(id)
                .map(|a| crate::engine::state::display_actor_name(state, a))
                .unwrap_or_else(|| id.to_string());
            content
                .ui_text
                .follow_actor_transcript
                .replace("{title}", &actor_name)
        }
        None => content.ui_text.follow_actor_stop_transcript.clone(),
    };
    if !feedback_text.trim().is_empty() {
        lines.narration(feedback_text);
    }
}
