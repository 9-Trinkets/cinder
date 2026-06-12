use super::builder::ActorTurnTargetContext;
use super::context::{SpeakCandidateContext, actors_in_room_except};
use crate::content::types::{ActorDefinition, ContentPack};
use crate::engine::dialogue::{DialogueGenerator, DirectSpeechIntentRequest};
use crate::engine::dialogue_grounding::{
    build_grounded_dialogue_request_for_exchange, build_grounded_dialogue_request_for_room,
    latest_other_person_message,
};
use crate::engine::events::WorldEvent;
use crate::engine::state::{ConversationMemoryKind, ConversationMemoryLine, WorldState};
use std::error::Error;

pub(crate) fn actor_action_dialogue(
    content: &ContentPack,
    dialogue: &dyn DialogueGenerator,
    state: &WorldState,
    actor: &ActorDefinition,
    current_room_id: &str,
    emit_trace: &mut dyn FnMut(&str, &str, serde_json::Value) -> Result<(), String>,
) -> Result<String, Box<dyn Error>> {
    let mut request = build_grounded_dialogue_request_for_room(
        content,
        state,
        &actor.id,
        current_room_id,
        "the room",
    )?;
    request.response_notes = content.system_text.actor_action_response_notes.clone();
    let trace_backend = dialogue.trace_metadata("actor_dialogue");
    emit_trace(
        "actor_dialogue",
        "model.request",
        serde_json::json!({
            "actor_id": request.actor_id,
            "actor_name": request.actor_name,
            "other_person_id": request.other_person_id,
            "other_person_name": request.other_person_name,
            "dialogue_request": request.clone(),
            "prompt": dialogue.build_prompt(&request),
            "backend": trace_backend.clone(),
        }),
    )
    .map_err(|error| -> Box<dyn Error> { Box::new(std::io::Error::other(error)) })?;
    match dialogue.generate(&request) {
        Ok(text) => {
            emit_trace(
                "actor_dialogue",
                "model.response",
                serde_json::json!({
                    "actor_id": request.actor_id,
                    "actor_name": request.actor_name,
                    "other_person_id": request.other_person_id,
                    "other_person_name": request.other_person_name,
                    "response_text": text.clone(),
                    "backend": trace_backend,
                }),
            )
            .map_err(|error| -> Box<dyn Error> { Box::new(std::io::Error::other(error)) })?;
            Ok(text)
        }
        Err(error) => {
            emit_trace(
                "actor_dialogue",
                "model.response",
                serde_json::json!({
                    "actor_id": request.actor_id,
                    "actor_name": request.actor_name,
                    "other_person_id": request.other_person_id,
                    "other_person_name": request.other_person_name,
                    "error": error.clone(),
                    "backend": trace_backend,
                }),
            )
            .map_err(|trace_error| -> Box<dyn Error> {
                Box::new(std::io::Error::other(trace_error))
            })?;
            Err(Box::new(std::io::Error::other(error)))
        }
    }
}

pub(crate) fn recent_actor_turn_memory(
    state: &WorldState,
    actor: &ActorDefinition,
    speak_candidates: &[SpeakCandidateContext<'_>],
) -> Vec<ConversationMemoryLine> {
    let mut recent_memory = speak_candidates
        .iter()
        .flat_map(|candidate| {
            state
                .conversation_history(&actor.id, &candidate.actor.id)
                .iter()
                .cloned()
        })
        .collect::<Vec<_>>();
    recent_memory.sort_by(|left, right| {
        left.turn_number
            .cmp(&right.turn_number)
            .then_with(|| left.event_sequence.cmp(&right.event_sequence))
            .then_with(|| {
                conversation_memory_kind_order(left).cmp(&conversation_memory_kind_order(right))
            })
            .then_with(|| left.speaker_id.cmp(&right.speaker_id))
            .then_with(|| left.text.cmp(&right.text))
    });
    recent_memory.dedup_by(|left, right| {
        left.turn_number == right.turn_number
            && left.kind == right.kind
            && left.speaker_id == right.speaker_id
            && left.text == right.text
    });
    if recent_memory.len() > 6 {
        recent_memory.drain(..recent_memory.len() - 6);
    }
    recent_memory
}

fn conversation_memory_kind_order(line: &ConversationMemoryLine) -> u8 {
    match line.kind {
        ConversationMemoryKind::Speech => 0,
        ConversationMemoryKind::Action => 1,
    }
}

pub(crate) fn actor_to_actor_dialogue(
    content: &ContentPack,
    dialogue: &dyn DialogueGenerator,
    state: &WorldState,
    actor: &ActorDefinition,
    current_room_id: &str,
    emit_trace: &mut dyn FnMut(&str, &str, serde_json::Value) -> Result<(), String>,
    target: &ActorTurnTargetContext,
) -> Result<Vec<WorldEvent>, Box<dyn Error>> {
    let other_person_message = latest_other_person_message(state, &actor.id, &target.actor_id);
    let request = build_grounded_dialogue_request_for_exchange(
        content,
        state,
        &actor.id,
        current_room_id,
        &target.actor_id,
        &target.actor_name,
        other_person_message.clone(),
    )?;
    let trace_backend = dialogue.trace_metadata("actor_dialogue");
    emit_trace(
        "actor_dialogue",
        "model.request",
        serde_json::json!({
            "actor_id": request.actor_id,
            "actor_name": request.actor_name,
            "other_person_id": request.other_person_id,
            "other_person_name": request.other_person_name,
            "dialogue_request": request.clone(),
            "prompt": dialogue.build_prompt(&request),
            "backend": trace_backend.clone(),
        }),
    )
    .map_err(|error| -> Box<dyn Error> { Box::new(std::io::Error::other(error)) })?;
    let text = match dialogue.generate(&request) {
        Ok(text) => {
            emit_trace(
                "actor_dialogue",
                "model.response",
                serde_json::json!({
                    "actor_id": request.actor_id,
                    "actor_name": request.actor_name,
                    "other_person_id": request.other_person_id,
                    "other_person_name": request.other_person_name,
                    "response_text": text.clone(),
                    "backend": trace_backend,
                }),
            )
            .map_err(|error| -> Box<dyn Error> { Box::new(std::io::Error::other(error)) })?;
            text
        }
        Err(error) => {
            emit_trace(
                "actor_dialogue",
                "model.response",
                serde_json::json!({
                    "actor_id": request.actor_id,
                    "actor_name": request.actor_name,
                    "other_person_id": request.other_person_id,
                    "other_person_name": request.other_person_name,
                    "error": error.clone(),
                    "backend": trace_backend,
                }),
            )
            .map_err(|trace_error| -> Box<dyn Error> {
                Box::new(std::io::Error::other(trace_error))
            })?;
            return Err(Box::new(std::io::Error::other(error)));
        }
    };
    let attraction_request = DirectSpeechIntentRequest {
        locale: request.locale.clone(),
        system_text: request.system_text.clone(),
        actor_id: request.actor_id.clone(),
        actor_name: request.actor_name.clone(),
        other_person_id: request.other_person_id.clone(),
        other_person_name: request.other_person_name.clone(),
        current_beat_notes: request.current_beat_notes.clone(),
        subtext_notes: request.subtext_notes.clone(),
        recent_memory: request.recent_memory.clone(),
        other_person_message: request.other_person_message.clone(),
        target_person_message: request.other_person_message.clone(),
        spoken_line: text,
    };
    let attraction_prompt = dialogue.build_direct_speech_intent_prompt(&attraction_request);
    let attraction_backend = dialogue.trace_metadata("direct_speech_intent");
    emit_trace(
        "direct_speech_intent",
        "model.request",
        serde_json::json!({
            "role": "direct_speech_intent",
            "actor_id": attraction_request.actor_id.clone(),
            "actor_name": attraction_request.actor_name.clone(),
            "other_person_id": attraction_request.other_person_id.clone(),
            "other_person_name": attraction_request.other_person_name.clone(),
            "intent_request": attraction_request.clone(),
            "prompt": attraction_prompt,
            "backend": attraction_backend.clone(),
        }),
    )
    .map_err(|error| -> Box<dyn Error> { Box::new(std::io::Error::other(error)) })?;
    let decision = dialogue
        .extract_direct_speech_intent(&attraction_request)
        .map_err(|error| -> Box<dyn Error> { Box::new(std::io::Error::other(error)) })?;
    emit_trace(
        "direct_speech_intent",
        "model.response",
        serde_json::json!({
            "role": "direct_speech_intent",
            "actor_id": attraction_request.actor_id.clone(),
            "actor_name": attraction_request.actor_name.clone(),
            "other_person_id": attraction_request.other_person_id.clone(),
            "other_person_name": attraction_request.other_person_name.clone(),
            "decision": decision.label(),
            "delta": decision.attraction_delta(),
            "backend": attraction_backend,
        }),
    )
    .map_err(|error| -> Box<dyn Error> { Box::new(std::io::Error::other(error)) })?;
    let mut events = vec![WorldEvent::ActorSpoke {
        actor_id: actor.id.clone(),
        actor_name: actor.name.clone(),
        other_person_id: target.actor_id.clone(),
        other_person_name: target.actor_name.clone(),
        other_person_message,
        room_id: current_room_id.to_string(),
        text: attraction_request.spoken_line.clone(),
    }];
    if decision.attraction_delta() > 0 {
        events.push(WorldEvent::PairStatAdjusted {
            participant_a_id: attraction_request.actor_id,
            participant_b_id: attraction_request.other_person_id,
            stat: "attraction".to_string(),
            delta: decision.attraction_delta(),
        });
    }
    Ok(events)
}

pub(crate) struct RoomSpeakDialogueTarget<'a> {
    pub audience: &'a [ActorTurnTargetContext],
}

pub(crate) fn actor_room_speak_dialogue(
    content: &ContentPack,
    dialogue: &dyn DialogueGenerator,
    state: &WorldState,
    actor: &ActorDefinition,
    current_room_id: &str,
    emit_trace: &mut dyn FnMut(&str, &str, serde_json::Value) -> Result<(), String>,
    target: RoomSpeakDialogueTarget<'_>,
) -> Result<Vec<WorldEvent>, Box<dyn Error>> {
    let request = crate::engine::dialogue_grounding::build_grounded_dialogue_request_for_room(
        content,
        state,
        &actor.id,
        current_room_id,
        "everyone here",
    )?;
    let trace_backend = dialogue.trace_metadata("actor_dialogue");
    emit_trace(
        "actor_dialogue",
        "model.request",
        serde_json::json!({
            "actor_id": request.actor_id,
            "actor_name": request.actor_name,
            "other_person_id": request.other_person_id,
            "other_person_name": request.other_person_name,
            "dialogue_request": request.clone(),
            "prompt": dialogue.build_prompt(&request),
            "backend": trace_backend.clone(),
        }),
    )
    .map_err(|error| -> Box<dyn Error> { Box::new(std::io::Error::other(error)) })?;
    let text = match dialogue.generate(&request) {
        Ok(text) => {
            emit_trace(
                "actor_dialogue",
                "model.response",
                serde_json::json!({
                    "actor_id": request.actor_id,
                    "actor_name": request.actor_name,
                    "other_person_id": request.other_person_id,
                    "other_person_name": request.other_person_name,
                    "response_text": text.clone(),
                    "backend": trace_backend,
                }),
            )
            .map_err(|error| -> Box<dyn Error> { Box::new(std::io::Error::other(error)) })?;
            text
        }
        Err(error) => {
            emit_trace(
                "actor_dialogue",
                "model.response",
                serde_json::json!({
                    "actor_id": request.actor_id,
                    "actor_name": request.actor_name,
                    "other_person_id": request.other_person_id,
                    "other_person_name": request.other_person_name,
                    "error": error.clone(),
                    "backend": trace_backend,
                }),
            )
            .map_err(|trace_error| -> Box<dyn Error> {
                Box::new(std::io::Error::other(trace_error))
            })?;
            return Err(Box::new(std::io::Error::other(error)));
        }
    };
    Ok(vec![WorldEvent::ActorSpokeToRoom {
        actor_id: actor.id.clone(),
        actor_name: actor.name.clone(),
        audience_actor_ids: target
            .audience
            .iter()
            .map(|actor| actor.actor_id.clone())
            .collect(),
        room_id: current_room_id.to_string(),
        text,
    }])
}

pub(crate) fn actor_turn_setting_notes(
    content: &ContentPack,
    state: &WorldState,
    actor: &ActorDefinition,
    current_room_id: &str,
) -> Vec<String> {
    let Some(room) = content.room(current_room_id) else {
        return Vec::new();
    };
    let mut notes = vec![
        content.render_template(
            &content.system_text.prompt_current_room_note,
            &[("room_title", room.title.as_str())],
        ),
        state.current_time_note(),
        room.summary.clone(),
    ];
    if !room.features.is_empty() {
        let features = room
            .features
            .iter()
            .map(|feature| feature.label.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        notes.push(content.render_template(
            &content.system_text.prompt_visible_features_note,
            &[("features", features.as_str())],
        ));
    }
    notes.extend(
        state
            .actor_recent_observation_notes(&actor.id)
            .iter()
            .cloned(),
    );
    let other_people = actors_in_room_except(content, state, current_room_id, &actor.id)
        .into_iter()
        .map(|other| other.name.as_str())
        .collect::<Vec<_>>();
    if !other_people.is_empty() {
        notes.push(content.render_template(
            &content.system_text.prompt_people_here_note,
            &[("people", &other_people.join(", "))],
        ));
    }
    if !room.exits.is_empty() {
        let exits = room
            .exits
            .iter()
            .map(|exit| exit.label.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        notes.push(content.render_template(
            &content.system_text.prompt_exits_note,
            &[("exits", exits.as_str())],
        ));
    }
    notes
}
