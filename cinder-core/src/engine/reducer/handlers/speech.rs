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

/// Applies a channel message: updates the conversation memories that the
/// message actually reached, tracks the awaited reply for targeted speech,
/// triggers speech hooks, and narrates the line when the player can hear it.
/// Local channels only deliver to actors sharing the speaker's room; direct
/// channels always deliver.
pub(crate) fn handle_channel_message(
    state: &mut WorldState,
    content: &ContentPack,
    lines: &mut NarrativeLines,
    message: &ChannelMessage,
) {
    match &message.audience {
        ChannelAudience::Targeted {
            recipient_id,
            recipient_name,
        } => handle_targeted_message(state, content, lines, message, recipient_id, recipient_name),
        ChannelAudience::Broadcast => handle_broadcast_message(state, content, lines, message),
    }
    lines.extend_narration(advance_house_progress_objectives(state, content));
}

fn handle_targeted_message(
    state: &mut WorldState,
    content: &ContentPack,
    lines: &mut NarrativeLines,
    message: &ChannelMessage,
    recipient_id: &str,
    recipient_name: &str,
) {
    let actor_id = message.speaker_id.as_str();
    let actor_name = message.speaker_name.as_str();
    // Local channels require shared-room presence; direct channels always
    // deliver regardless of space.
    if message.delivery.kind == ChannelKind::Local
        && let Some(room_id) = message.delivery.room_id.as_deref()
    {
        let co_present = content
            .actor(recipient_id)
            .map(|_| state.actor_is_in_room(content, recipient_id, room_id))
            // An unresolvable recipient is not gated (never over-restrict).
            .unwrap_or(true);
        if !co_present {
            return;
        }
    }
    mark_actor_objective_progress_for_speech_event(
        content,
        state,
        actor_id,
        ObjectiveSpeechEvent::ToActor,
    );
    let history = state.conversation_history(actor_id, recipient_id);
    let needs_other_person_line = message.in_reply_to.as_deref().is_some_and(|reply_to| {
        history
            .last()
            .is_none_or(|line| line.speaker_id != recipient_id || line.text != reply_to)
    });
    if needs_other_person_line && let Some(reply_to) = &message.in_reply_to {
        state.push_conversation_line(
            actor_id,
            recipient_id,
            ConversationMemoryLine {
                turn_number: state.turn_number,
                event_sequence: 0,
                speaker_id: recipient_id.to_string(),
                speaker_name: recipient_name.to_string(),
                kind: ConversationMemoryKind::Speech,
                target_label: Some(actor_name.to_string()),
                text: reply_to.clone(),
            },
        );
    }
    state.push_conversation_line(
        actor_id,
        recipient_id,
        ConversationMemoryLine {
            turn_number: state.turn_number,
            event_sequence: 0,
            speaker_id: actor_id.to_string(),
            speaker_name: actor_name.to_string(),
            kind: ConversationMemoryKind::Speech,
            target_label: Some(recipient_name.to_string()),
            text: message.text.clone(),
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
            "participant_b_id": recipient_id,
        }),
    )
    .unwrap_or_else(|error| eprintln!("[cinder] hook warning (speech): {error}"));
    if state
        .pending_reply(actor_id, recipient_id)
        .is_some_and(|pending| {
            pending.speaker_id == recipient_id && pending.listener_id == actor_id
        })
    {
        state.clear_pending_reply(actor_id, recipient_id);
    }
    state.set_pending_reply(
        actor_id,
        recipient_id,
        message.delivery.room_id.as_deref().unwrap_or(""),
        state.turn_number,
    );
    if message_hearable(state, message) {
        lines.narration(render_actor_speech_line(
            content,
            actor_name,
            Some(recipient_name),
            &message.text,
        ));
    }
}

fn handle_broadcast_message(
    state: &mut WorldState,
    content: &ContentPack,
    lines: &mut NarrativeLines,
    message: &ChannelMessage,
) {
    let actor_id = message.speaker_id.as_str();
    let actor_name = message.speaker_name.as_str();
    mark_actor_objective_progress_for_speech_event(
        content,
        state,
        actor_id,
        ObjectiveSpeechEvent::ToRoom,
    );
    for recipient_id in &message.delivery.recipients {
        state.push_conversation_line(
            actor_id,
            recipient_id,
            ConversationMemoryLine {
                turn_number: state.turn_number,
                event_sequence: 0,
                speaker_id: actor_id.to_string(),
                speaker_name: actor_name.to_string(),
                kind: ConversationMemoryKind::Speech,
                target_label: Some("room".to_string()),
                text: message.text.clone(),
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
                "participant_b_id": recipient_id,
            }),
        )
        .unwrap_or_else(|error| eprintln!("[cinder] hook warning (speech): {error}"));
    }
    if message_hearable(state, message) {
        lines.narration(render_actor_speech_line(content, actor_name, None, &message.text));
    }
}

/// Whether the player can hear a message: direct comms always, local speech
/// only when it happens in the player's current room.
fn message_hearable(state: &WorldState, message: &ChannelMessage) -> bool {
    match message.delivery.kind {
        ChannelKind::Direct => true,
        ChannelKind::Local => message.delivery.room_id.as_deref() == Some(&state.current_room_id),
    }
}