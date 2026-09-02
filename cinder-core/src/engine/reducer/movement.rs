use crate::content::types::ContentPack;
use crate::engine::events::ObservationMode;
use crate::engine::hook_ids;
use crate::engine::hooks::apply_world_hook_effects;
use crate::engine::narrative::NarrativeLines;
use crate::engine::state::WorldState;
use serde_json::json;

use super::beat_advance::advance_objective_for_signal;
use super::observation::render_room_observation;
use super::tick::{advance_house_progress_objectives, pending_reply_broken_by_move};

pub(super) struct ActorMoveTransitionContext<'a> {
    pub(super) actor_id: &'a str,
    pub(super) actor_name: Option<&'a str>,
    pub(super) from_room_id: &'a str,
    pub(super) to_room_id: &'a str,
    pub(super) command_text: Option<&'a str>,
}

pub(super) fn apply_actor_move_transition(
    state: &mut WorldState,
    content: &ContentPack,
    movement: ActorMoveTransitionContext<'_>,
    lines: &mut NarrativeLines,
) {
    apply_world_hook_effects(
        state,
        content,
        hook_ids::ACTOR_MOVED,
        json!({
            "actor_id": movement.actor_id,
            "from_room_id": movement.from_room_id,
            "to_room_id": movement.to_room_id,
        }),
    )
    .unwrap_or_else(|error| eprintln!("[cinder] hook warning (actor.moved): {error}"));
    if let Some(pending) =
        pending_reply_broken_by_move(state, movement.actor_id, movement.from_room_id)
    {
        apply_world_hook_effects(
            state,
            content,
            hook_ids::BROKEN_REPLY,
            json!({
                "event_kind": "broken_reply",
                "participant_a_id": pending.speaker_id,
                "participant_b_id": pending.listener_id,
            }),
        )
        .unwrap_or_else(|error| eprintln!("[cinder] hook warning (broken_reply): {error}"));
        state.clear_pending_reply(&pending.speaker_id, &pending.listener_id);
    }
    let is_followed_actor = state.followed_actor_id.as_deref() == Some(movement.actor_id);
    let is_player = movement.actor_id == content.settings.combat.player_actor_id;
    let actor_name = movement
        .actor_name
        .or_else(|| {
            content
                .actor(movement.actor_id)
                .map(|actor| actor.name.as_str())
        })
        .unwrap_or(movement.actor_id);
    if let Some(command_text) = movement.command_text {
        if state.current_room_id == movement.from_room_id || is_followed_actor {
            lines.narration(command_text.to_string());
        } else if !is_followed_actor
            && state.current_room_id == movement.to_room_id
            && let Some(origin) = content.room(movement.from_room_id)
        {
            lines.narration(content.render_template(
                &content.presentation.presentation_text.actor_arrived,
                &[
                    ("actor_name", actor_name),
                    ("room_title", origin.title.as_str()),
                ],
            ));
        }
    } else if !is_followed_actor && state.current_room_id == movement.from_room_id {
        if let Some(destination) = content.room(movement.to_room_id) {
            lines.narration(content.render_template(
                &content.presentation.presentation_text.actor_departed,
                &[
                    ("actor_name", actor_name),
                    ("room_title", destination.title.as_str()),
                ],
            ));
        }
    } else if !is_followed_actor
        && state.current_room_id == movement.to_room_id
        && let Some(origin) = content.room(movement.from_room_id)
    {
        lines.narration(content.render_template(
            &content.presentation.presentation_text.actor_arrived,
            &[
                ("actor_name", actor_name),
                ("room_title", origin.title.as_str()),
            ],
        ));
    }
    state.mark_actor_room_visited(movement.actor_id, movement.to_room_id);
    state.actor_room_overrides.insert(
        movement.actor_id.to_string(),
        movement.to_room_id.to_string(),
    );
    if is_followed_actor || is_player {
        state.current_room_id = movement.to_room_id.to_string();
        if is_player {
            super::handlers::sync_followers_to_room(state, content, movement.to_room_id, lines);
        }
        if let Some(observation) = render_room_observation(
            content,
            state,
            movement.to_room_id,
            ObservationMode::Summary,
        ) {
            lines.extend(observation.0);
        }
    }
    lines.extend_narration(advance_objective_for_signal(
        state,
        content,
        &format!(
            "actor_entered:{}:{}",
            movement.actor_id, movement.to_room_id
        ),
    ));
    lines.extend_narration(advance_house_progress_objectives(state, content));
}
