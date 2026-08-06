use crate::content::types::CommandInputMode;
use crate::engine::dialogue::{
    ActorTurnActionDecision, ActorTurnActionRequest, ActorTurnCommandInvocation,
};
use std::error::Error;

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
