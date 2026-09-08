use crate::engine::reducer::observation::render_actor_speech_line;
use crate::engine::reducer::tick::advance_house_progress_objectives;
use crate::content::types::ContentPack;
use crate::engine::hook_ids;
use crate::engine::hooks::apply_world_hook_effects;
use crate::engine::messaging::{ChannelAudience, ChannelKind, ChannelMessage};
use crate::engine::narrative::NarrativeLines;
use crate::engine::state::{ConversationMemoryKind, ConversationMemoryLine, WorldState};
use crate::engine::turn_policies::{
    ObjectiveSpeechEvent, mark_actor_objective_progress_for_speech_event,
};
use serde_json::json;

pub(crate) struct SpokeContext<'a> {
    pub(crate) actor_id: &'a str,
    pub(crate) actor_name: &'a str,
    pub(crate) other_person_id: &'a str,
    pub(crate) other_person_name: &'a str,
    pub(crate) other_person_message: &'a Option<String>,
    pub(crate) room_id: &'a str,
    pub(crate) text: &'a str,
}

/// Renders a channel message for the player. Local speech is only narrated
/// when it happens in the player's current room; direct comms is always
/// narrated. Presentation is generic for now — styling by channel kind lands
/// with the transcript presentation pass.
pub(crate) fn handle_channel_message(
    state: &WorldState,
    content: &ContentPack,
    lines: &mut NarrativeLines,
    message: &ChannelMessage,
) {
    let hearable = match message.delivery.kind {
        ChannelKind::Direct => true,
        ChannelKind::Local => {
            message.delivery.room_id.as_deref() == Some(&state.current_room_id)
        }
    };
    if !hearable {
        return;
    }
    lines.narration(render_actor_speech_line(
        content,
        &message.speaker_name,
        match &message.audience {
            ChannelAudience::Targeted { recipient_name, .. } => Some(recipient_name),
            ChannelAudience::Broadcast => None,
        },
        &message.text,
    ));
}

pub(crate) fn handle_actor_spoke(
    state: &mut WorldState,
    content: &ContentPack,
    lines: &mut NarrativeLines,
    ctx: SpokeContext<'_>,
) {
    let actor_id = ctx.actor_id;
    let actor_name = ctx.actor_name;
    let other_person_id = ctx.other_person_id;
    let other_person_name = ctx.other_person_name;
    let other_person_message = ctx.other_person_message;
    let room_id = ctx.room_id;
    let text = ctx.text;
    mark_actor_objective_progress_for_speech_event(
        content,
        state,
        actor_id,
        ObjectiveSpeechEvent::ToActor,
    );
    let history = state.conversation_history(actor_id, other_person_id);
    let needs_other_person_line = other_person_message.as_ref().is_some_and(|message| {
        history
            .last()
            .is_none_or(|line| line.speaker_id != other_person_id || line.text != *message)
    });
    if needs_other_person_line && let Some(message) = other_person_message {
        state.push_conversation_line(
            actor_id,
            other_person_id,
            ConversationMemoryLine {
                turn_number: state.turn_number,
                event_sequence: 0,
                speaker_id: other_person_id.to_string(),
                speaker_name: other_person_name.to_string(),
                kind: ConversationMemoryKind::Speech,
                target_label: Some(actor_name.to_string()),
                text: message.clone(),
            },
        );
    }
    state.push_conversation_line(
        actor_id,
        other_person_id,
        ConversationMemoryLine {
            turn_number: state.turn_number,
            event_sequence: 0,
            speaker_id: actor_id.to_string(),
            speaker_name: actor_name.to_string(),
            kind: ConversationMemoryKind::Speech,
            target_label: Some(other_person_name.to_string()),
            text: text.to_string(),
        },
    );
    apply_world_hook_effects(
        state,
        content,
        hook_ids::SPEECH,
        json!({
            "event_kind": "speech",
            "actor_id": actor_id,
            "participant_a_id": actor_id,
            "participant_b_id": other_person_id,
        }),
    )
    .unwrap_or_else(|error| eprintln!("[cinder] hook warning (speech): {error}"));
    if state
        .pending_reply(actor_id, other_person_id)
        .is_some_and(|pending| {
            pending.speaker_id == other_person_id && pending.listener_id == actor_id
        })
    {
        state.clear_pending_reply(actor_id, other_person_id);
    }
    state.set_pending_reply(actor_id, other_person_id, room_id, state.turn_number);
    if state.current_room_id == room_id {
        lines.narration(render_actor_speech_line(
            content,
            actor_name,
            Some(other_person_name),
            text,
        ));
    }
    lines.extend_narration(advance_house_progress_objectives(state, content));
}

pub(crate) struct SpokeToRoomContext<'a> {
    pub(crate) actor_id: &'a str,
    pub(crate) actor_name: &'a str,
    pub(crate) audience_actor_ids: &'a [String],
    pub(crate) room_id: &'a str,
    pub(crate) text: &'a str,
}

pub(crate) fn handle_actor_spoke_to_room(
    state: &mut WorldState,
    content: &ContentPack,
    lines: &mut NarrativeLines,
    ctx: SpokeToRoomContext<'_>,
) {
    let actor_id = ctx.actor_id;
    let actor_name = ctx.actor_name;
    let audience_actor_ids = ctx.audience_actor_ids;
    let room_id = ctx.room_id;
    let text = ctx.text;
    mark_actor_objective_progress_for_speech_event(
        content,
        state,
        actor_id,
        ObjectiveSpeechEvent::ToRoom,
    );
    for audience_actor_id in audience_actor_ids.iter() {
        state.push_conversation_line(
            actor_id,
            audience_actor_id,
            ConversationMemoryLine {
                turn_number: state.turn_number,
                event_sequence: 0,
                speaker_id: actor_id.to_string(),
                speaker_name: actor_name.to_string(),
                kind: ConversationMemoryKind::Speech,
                target_label: Some("room".to_string()),
                text: text.to_string(),
            },
        );
        apply_world_hook_effects(
            state,
            content,
            hook_ids::SPEECH,
            json!({
                "event_kind": "speech",
                "actor_id": actor_id,
                "participant_a_id": actor_id,
                "participant_b_id": audience_actor_id,
            }),
        )
        .unwrap_or_else(|error| eprintln!("[cinder] hook warning (speech): {error}"));
    }
    if state.current_room_id == room_id {
        lines.narration(render_actor_speech_line(content, actor_name, None, text));
    }
    lines.extend_narration(advance_house_progress_objectives(state, content));
}
