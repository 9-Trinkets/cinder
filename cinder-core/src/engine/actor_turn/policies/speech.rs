use crate::engine::dialogue::{
    ActorTurnActionDecision, ActorTurnActionRequest, ActorTurnCommandInvocation,
};

use super::super::decisions::{directly_addressed_target_actor_id, preferred_target_actor_id};

#[derive(Debug, Clone, Copy)]
pub(crate) struct SpeechPolicyInput {
    pub should_speak: bool,
    pub should_direct_speech: bool,
}

pub(crate) fn decide_speech_action(
    request: &ActorTurnActionRequest,
    input: SpeechPolicyInput,
) -> Option<ActorTurnActionDecision> {
    if !input.should_speak {
        return None;
    }

    if input.should_direct_speech
        && let Some(target_actor_id) = directly_addressed_target_actor_id(request)
            .or_else(|| preferred_target_actor_id(request))
    {
        return Some(ActorTurnActionDecision::Command {
            command_id: "speak".to_string(),
            target_room_id: None,
            target_actor_id: Some(target_actor_id),
            feature_id: None,
            consumable_id: None,
            context_label: None,
            freeform_text: None,
        });
    }

    if let Some(command_id) =
        request
            .affordances
            .iter()
            .find_map(|affordance| match &affordance.invocation {
                ActorTurnCommandInvocation::Command {
                    command_id,
                    target_actor_id: None,
                    ..
                } if command_id == "speak" => Some(command_id.clone()),
                _ => None,
            })
    {
        return Some(ActorTurnActionDecision::Command {
            command_id,
            target_room_id: None,
            target_actor_id: None,
            feature_id: None,
            consumable_id: None,
            context_label: None,
            freeform_text: None,
        });
    }

    if let Some(target_actor_id) = preferred_target_actor_id(request) {
        return Some(ActorTurnActionDecision::Command {
            command_id: "speak".to_string(),
            target_room_id: None,
            target_actor_id: Some(target_actor_id),
            feature_id: None,
            consumable_id: None,
            context_label: None,
            freeform_text: None,
        });
    }

    None
}
