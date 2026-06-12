use crate::content::types::CommandInputMode;
use crate::engine::dialogue::{
    ActorTurnActionDecision, ActorTurnActionRequest, ActorTurnCommandInvocation,
};
use std::error::Error;

pub fn has_clearly_preferred_target(request: &ActorTurnActionRequest) -> bool {
    let mut ranked_candidates = ranked_speak_candidates(request);
    if ranked_candidates.is_empty() {
        return false;
    }
    if ranked_candidates.len() == 1 {
        return true;
    }
    let top_score = ranked_candidates.remove(0).0;
    let second_score = ranked_candidates.remove(0).0;
    top_score > second_score
}

pub fn rest_decision(request: &ActorTurnActionRequest) -> Option<ActorTurnActionDecision> {
    request
        .affordances
        .iter()
        .find_map(|affordance| match &affordance.invocation {
            ActorTurnCommandInvocation::Command {
                command_id,
                target_room_id,
                target_actor_id,
                feature_id,
                consumable_id,
                context_label,
                input_mode,
            } if command_id == "rest" && *input_mode == CommandInputMode::None => {
                Some(ActorTurnActionDecision::Command {
                    command_id: command_id.clone(),
                    target_room_id: target_room_id.clone(),
                    target_actor_id: target_actor_id.clone(),
                    feature_id: feature_id.clone(),
                    consumable_id: consumable_id.clone(),
                    context_label: context_label.clone(),
                    freeform_text: None,
                })
            }
            _ => None,
        })
}

pub fn quiet_room_action_decision(
    request: &ActorTurnActionRequest,
    text: &str,
) -> Result<ActorTurnActionDecision, Box<dyn Error>> {
    request
        .affordances
        .iter()
        .find_map(|affordance| match &affordance.invocation {
            ActorTurnCommandInvocation::Command {
                command_id,
                target_room_id,
                target_actor_id,
                feature_id,
                consumable_id,
                context_label,
                input_mode: CommandInputMode::FreeformText,
            } => Some(ActorTurnActionDecision::Command {
                command_id: command_id.clone(),
                target_room_id: target_room_id.clone(),
                target_actor_id: target_actor_id.clone(),
                feature_id: feature_id.clone(),
                consumable_id: consumable_id.clone(),
                context_label: context_label.clone(),
                freeform_text: Some(text.to_string()),
            }),
            _ => None,
        })
        .ok_or_else(|| {
            Box::new(std::io::Error::other(
                "missing authored freeform npc command affordance for quiet in-room action",
            )) as Box<dyn Error>
        })
}

pub fn consume_decision_for_item(
    request: &ActorTurnActionRequest,
    item_id: &str,
) -> Option<ActorTurnActionDecision> {
    request
        .affordances
        .iter()
        .find_map(|affordance| match &affordance.invocation {
            ActorTurnCommandInvocation::Command {
                command_id,
                target_room_id,
                target_actor_id,
                feature_id,
                consumable_id: Some(consumable_id),
                context_label,
                input_mode: CommandInputMode::None,
            } if consumable_id == item_id => Some(ActorTurnActionDecision::Command {
                command_id: command_id.clone(),
                target_room_id: target_room_id.clone(),
                target_actor_id: target_actor_id.clone(),
                feature_id: feature_id.clone(),
                consumable_id: Some(consumable_id.clone()),
                context_label: context_label.clone(),
                freeform_text: None,
            }),
            _ => None,
        })
}

pub fn directly_addressed_target_actor_id(request: &ActorTurnActionRequest) -> Option<String> {
    request
        .speak_candidates
        .iter()
        .filter(|candidate| candidate.reply_now)
        .map(|candidate| candidate.actor_id.clone())
        .min()
}

pub fn preferred_target_actor_id(request: &ActorTurnActionRequest) -> Option<String> {
    ranked_speak_candidates(request)
        .first()
        .map(|(_, _, actor_id)| actor_id.clone())
}

pub fn ranked_speak_candidates(request: &ActorTurnActionRequest) -> Vec<(i32, i32, String)> {
    let mut candidates = request
        .speak_candidates
        .iter()
        .map(|candidate| {
            let connection = candidate
                .pair_stats
                .get("connection")
                .copied()
                .unwrap_or_default();
            let safety = candidate
                .pair_stats
                .get("safety")
                .copied()
                .unwrap_or_default();
            let attraction = candidate
                .pair_stats
                .get("attraction")
                .copied()
                .unwrap_or_default();
            (
                connection + safety + attraction,
                connection,
                candidate.actor_id.clone(),
            )
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| right.1.cmp(&left.1))
            .then_with(|| left.2.cmp(&right.2))
    });
    candidates
}
