use super::types::{PlannedTurn, RouteEnvelope};
use crate::content::types::{ContentPack, SpeechIntentEffect};
use crate::engine::dialogue::{
    DialogueGenerator, DirectSpeechIntentDecision, DirectSpeechIntentRequest,
};
use crate::engine::events::WorldEvent;

fn apply_speech_intent_effects(
    content: &ContentPack,
    decision: &DirectSpeechIntentDecision,
    actor_id: &str,
    other_person_id: &str,
) -> Vec<WorldEvent> {
    let label = &decision.0;
    let Some(intent) = content
        .speech_intents
        .intents
        .iter()
        .find(|i| i.label.eq_ignore_ascii_case(label))
    else {
        return Vec::new();
    };
    intent
        .effects
        .iter()
        .map(|effect| match effect {
            SpeechIntentEffect::ActorStat { stat, delta } => {
                WorldEvent::ActorStatAdjusted {
                    actor_id: actor_id.to_string(),
                    stat: stat.clone(),
                    delta: *delta,
                }
            }
            SpeechIntentEffect::PairStat { stat, delta } => {
                WorldEvent::PairStatAdjusted {
                    participant_a_id: actor_id.to_string(),
                    participant_b_id: other_person_id.to_string(),
                    stat: stat.clone(),
                    delta: *delta,
                }
            }
        })
        .collect()
}

pub(super) fn handle_actor_dialogue(
    dialogue: &dyn DialogueGenerator,
    content: &ContentPack,
    role_name: &str,
    next_role: String,
    inbound: &str,
    emit_trace: impl Fn(&str, &str, serde_json::Value) -> Result<(), String>,
) -> Result<RouteEnvelope, String> {
    let intents = &content.speech_intents.intents;
    let mut planned: PlannedTurn =
        serde_json::from_str(inbound).map_err(|error| error.to_string())?;
    let request = planned
        .grounded_dialogue
        .take()
        .ok_or_else(|| "actor_dialogue missing grounded dialogue".to_string())?;
    let prompt = dialogue.build_prompt(&request);
    let trace_backend = dialogue.trace_metadata(role_name);
    emit_trace(
        "actor_dialogue",
        "model.request",
        serde_json::json!({
            "role": role_name,
            "actor_id": request.actor_id.clone(),
            "actor_name": request.actor_name.clone(),
            "other_person_name": request.other_person_name.clone(),
            "dialogue_request": request.clone(),
            "prompt": prompt,
            "backend": trace_backend.clone(),
        }),
    )?;
    match dialogue.generate(&request) {
        Ok(text) => {
            emit_trace(
                "actor_dialogue",
                "model.response",
                serde_json::json!({
                    "role": role_name,
                    "actor_id": request.actor_id.clone(),
                    "actor_name": request.actor_name.clone(),
                    "response_text": text.clone(),
                    "backend": trace_backend.clone(),
                }),
            )?;
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
            planned.events.push(WorldEvent::ActorSpoke {
                actor_id: request.actor_id,
                actor_name: request.actor_name,
                other_person_id: request.other_person_id,
                other_person_name: request.other_person_name,
                other_person_message: request.other_person_message,
                room_id: request.current_room_id,
                text: attraction_request.spoken_line.clone(),
            });
            let attraction_prompt =
                dialogue.build_direct_speech_intent_prompt(&attraction_request, intents);
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
            )?;
            let decision =
                dialogue.extract_direct_speech_intent(&attraction_request, intents)?;
            emit_trace(
                "direct_speech_intent",
                "model.response",
                serde_json::json!({
                    "role": "direct_speech_intent",
                    "actor_id": attraction_request.actor_id.clone(),
                    "actor_name": attraction_request.actor_name.clone(),
                    "other_person_id": attraction_request.other_person_id.clone(),
                    "other_person_name": attraction_request.other_person_name.clone(),
                    "decision": decision.0,
                    "backend": attraction_backend,
                }),
            )?;
            planned.events.extend(apply_speech_intent_effects(
                content,
                &decision,
                &attraction_request.actor_id,
                &attraction_request.other_person_id,
            ));
        }
        Err(error) => planned.events.push(WorldEvent::ActionRejected {
            message: {
                emit_trace(
                    "actor_dialogue",
                    "model.response",
                    serde_json::json!({
                        "role": role_name,
                        "actor_id": request.actor_id.clone(),
                        "actor_name": request.actor_name.clone(),
                        "error": error.clone(),
                        "backend": trace_backend,
                    }),
                )?;
                content.render_template(
                    &content.presentation.error_text.dialogue_unavailable,
                    &[
                        ("actor_name", request.actor_name.as_str()),
                        ("error", error.as_str()),
                    ],
                )
            },
        }),
    }
    Ok(RouteEnvelope {
        next: next_role,
        message: serde_json::to_string(&planned).map_err(|error| error.to_string())?,
    })
}
