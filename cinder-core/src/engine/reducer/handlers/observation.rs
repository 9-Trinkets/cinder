use crate::engine::reducer::observation::{
    render_feature_consumables_line, render_room_observation, render_story_text,
};
use crate::content::types::ContentPack;
use crate::engine::events::ObservationMode;
use crate::engine::narrative::NarrativeLines;
use crate::engine::state::WorldState;

pub(crate) fn handle_current_room_observed(
    state: &mut WorldState,
    content: &ContentPack,
    room_id: &str,
    mode: ObservationMode,
    lines: &mut NarrativeLines,
) {
    if let Some(observation) = render_room_observation(content, state, room_id, mode) {
        lines.extend(observation.0);
    } else {
        lines.error(content.presentation.error_text.room_missing.clone());
    }
}

pub(crate) fn handle_feature_observed(
    state: &mut WorldState,
    content: &ContentPack,
    room_id: &str,
    feature_id: &str,
    lines: &mut NarrativeLines,
) {
    if let Some(feature) = content.room(room_id).and_then(|room| {
        room.features
            .iter()
            .find(|feature| feature.id == feature_id)
    }) {
        lines.narration(feature.inspect_text.clone());
        if let Some(consumables_line) =
            render_feature_consumables_line(content, state, room_id, feature_id)
        {
            lines.narration(consumables_line);
        }
    } else {
        lines.narration(content.presentation.error_text.room_missing.clone());
    }
}

pub(crate) fn handle_actor_observed(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    lines: &mut NarrativeLines,
) {
    if let Some(actor) = content.actor(actor_id) {
        lines.narration(render_story_text(&actor.inspect_text, state));
    } else {
        lines.narration(content.presentation.error_text.actor_unknown.clone());
    }
}

pub(crate) fn handle_actor_observed_room(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    actor_name: &str,
    room_id: &str,
    lines: &mut NarrativeLines,
) {
    if let Some(room) = content.room(room_id) {
        state.mark_actor_observed_room(actor_id, room_id);
        state.push_actor_observation_note(actor_id, room.inspect_text.clone());
        if state.current_room_id == room_id
            && let Some(line) = content.render_message(
                "observation.actor_inspects_room",
                &[
                    ("actor_name", actor_name),
                    ("room_title", room.title.as_str()),
                ],
            )
        {
            lines.narration(line);
        }
    } else {
        lines.narration(content.presentation.error_text.room_missing.clone());
    }
}

pub(crate) fn handle_actor_observed_feature(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    actor_name: &str,
    room_id: &str,
    feature_id: &str,
    lines: &mut NarrativeLines,
) {
    if let Some((room, feature)) = content.room(room_id).and_then(|room| {
        room.features
            .iter()
            .find(|feature| feature.id == feature_id)
            .map(|feature| (room, feature))
    }) {
        state.mark_actor_feature_seen(actor_id, room_id, feature_id);
        state.push_actor_observation_note(actor_id, feature.inspect_text.clone());
        if state.current_room_id == room_id
            && let Some(line) = content.render_message(
                "observation.actor_inspects_feature",
                &[
                    ("actor_name", actor_name),
                    ("feature_label", feature.label.as_str()),
                ],
            )
        {
            lines.narration(line);
        }
        let _ = room;
    } else {
        lines.narration(content.presentation.error_text.room_missing.clone());
    }
}

pub(crate) struct ActorObservationContext<'a> {
    pub(crate) actor_id: &'a str,
    pub(crate) actor_name: &'a str,
    pub(crate) target_actor_id: &'a str,
    pub(crate) target_actor_name: &'a str,
}

pub(crate) fn handle_actor_observed_actor(
    state: &mut WorldState,
    content: &ContentPack,
    room_id: &str,
    lines: &mut NarrativeLines,
    ctx: ActorObservationContext<'_>,
) {
    let actor_id = ctx.actor_id;
    let actor_name = ctx.actor_name;
    let target_actor_id = ctx.target_actor_id;
    let target_actor_name = ctx.target_actor_name;
    if let Some(target_actor) = content.actor(target_actor_id) {
        state.mark_actor_studied_actor(actor_id, target_actor_id);
        state.push_actor_observation_note(actor_id, target_actor.inspect_text.clone());
        if state.current_room_id == room_id
            && let Some(line) = content.render_message(
                "observation.actor_studies_actor",
                &[
                    ("actor_name", actor_name),
                    ("target_actor_name", target_actor_name),
                ],
            )
        {
            lines.narration(line);
        }
    } else {
        lines.narration(content.presentation.error_text.actor_unknown.clone());
    }
}
