use crate::engine::reducer::beat_advance::advance_objective_for_signal;
use crate::engine::reducer::command_effects::{
    ActorMoveTransitionContext, actor_display_name, apply_actor_move_transition,
};
use crate::engine::reducer::tick::advance_house_progress_objectives;
use crate::content::types::ContentPack;
use crate::engine::narrative::NarrativeLines;
use crate::engine::state::WorldState;
pub(crate) fn handle_actor_relocated(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    to_room_id: &str,
    lines: &mut NarrativeLines,
) {
    state.mark_actor_room_visited(actor_id, to_room_id);
    state
        .actor_room_overrides
        .insert(actor_id.to_string(), to_room_id.to_string());
    lines.extend_narration(advance_house_progress_objectives(state, content));
}

pub(crate) fn handle_actor_moved(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    from_room_id: &str,
    to_room_id: &str,
    lines: &mut NarrativeLines,
) {
    apply_actor_move_transition(
        state,
        content,
        ActorMoveTransitionContext {
            actor_id,
            actor_name: None,
            from_room_id,
            to_room_id,
            command_text: None,
        },
        lines,
    );
}

pub(crate) fn handle_player_moved(
    state: &mut WorldState,
    content: &ContentPack,
    to_room_id: &str,
    lines: &mut NarrativeLines,
) {
    let from_room_id = state.current_room_id.clone();
    lines.extend_narration(advance_objective_for_signal(
        state,
        content,
        &format!("room_left:{from_room_id}"),
    ));
    state.current_room_id = to_room_id.to_string();
    lines.extend_narration(advance_objective_for_signal(
        state,
        content,
        &format!("room_entered:{to_room_id}"),
    ));
    sync_followers_to_room(state, content, to_room_id, lines);
}

/// Moves every living follower into the player's room (used when the player
/// moves or descends, so the party stays together).
pub(crate) fn sync_followers_to_room(
    state: &mut WorldState,
    content: &ContentPack,
    to_room_id: &str,
    lines: &mut NarrativeLines,
) {
    let follower_actor_ids: Vec<String> = state
        .relationships
        .iter()
        .filter(|(_, relationship)| relationship.follows_player)
        .map(|(actor_id, _)| actor_id.clone())
        .collect();
    for follower_id in follower_actor_ids {
        if follower_id == content.settings.combat.player_actor_id {
            continue;
        }
        if state.actor_stat(&follower_id, &content.settings.combat.health_stat_id) <= 0 {
            continue;
        }
        let default_room_id = content
            .actor(&follower_id)
            .map(|actor| actor.room_id.clone())
            .unwrap_or_default();
        let already_here = state.actor_room_id(&follower_id, &default_room_id) == to_room_id;
        if !already_here {
            state
                .actor_room_overrides
                .insert(follower_id.clone(), to_room_id.to_string());
            if let Some(line) = content.render_message(
                "follow.actor_follows",
                &[("actor", actor_display_name(content, &follower_id).as_str())],
            ) {
                lines.narration(line);
            }
        }
    }
}