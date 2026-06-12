use crate::content::types::{
    CommandEffect, CommandOutcomeMode, CommandTargetMode, CommandsDefinition,
    PlayerCommandTargetMode,
};
use std::error::Error;

pub(crate) fn require_known_id(
    id: &str,
    known: &[&str],
    subject: &str,
    collection: &str,
) -> Result<(), Box<dyn Error>> {
    if known.contains(&id) {
        Ok(())
    } else {
        Err(format!("{subject} not found in {collection}").into())
    }
}

pub(crate) fn validate_player_commands(
    commands: &CommandsDefinition,
) -> Result<(), Box<dyn Error>> {
    for command in &commands.actions {
        let has_move_effect = command.has_effect(CommandEffect::MoveActor);
        let has_observe_room_effect = command.has_effect(CommandEffect::ObserveRoom);
        let has_target_observe_effect =
            command.has_any_effect(&[CommandEffect::ObserveFeature, CommandEffect::ObserveActor]);
        let has_target_memory_effect = command.has_effect(CommandEffect::RememberWithTargetActor);
        if !command.player_enabled {
            continue;
        }
        let metadata = command.player_command.as_ref().ok_or_else(|| {
            format!(
                "player-enabled command '{}' must define player_command metadata",
                command.id
            )
        })?;
        if command.player_phrases.is_empty() {
            return Err(format!(
                "player-enabled command '{}' must define player_phrases",
                command.id
            )
            .into());
        }
        if metadata.usage.trim().is_empty() {
            return Err(format!(
                "player-enabled command '{}' must define player_command.usage",
                command.id
            )
            .into());
        }
        if metadata.example.trim().is_empty() {
            return Err(format!(
                "player-enabled command '{}' must define player_command.example",
                command.id
            )
            .into());
        }
        if command.outcome_mode == CommandOutcomeMode::Dialogue {
            if metadata.target_mode == PlayerCommandTargetMode::None {
                return Err(format!(
                    "dialogue player command '{}' must define player_command.target_mode",
                    command.id
                )
                .into());
            }
            if metadata.target_mode == PlayerCommandTargetMode::ActorReference
                && !metadata.input.as_ref().is_some_and(|input| input.required)
            {
                return Err(format!(
                    "actor-reference dialogue command '{}' must require player input",
                    command.id
                )
                .into());
            }
            if metadata.target_mode == PlayerCommandTargetMode::FirstActorInRoom
                && metadata.input.is_some()
            {
                return Err(format!(
                    "first-actor-in-room dialogue command '{}' must not define player input",
                    command.id
                )
                .into());
            }
            if !matches!(
                metadata.target_mode,
                PlayerCommandTargetMode::ActorReference | PlayerCommandTargetMode::FirstActorInRoom
            ) {
                return Err(format!(
                    "dialogue player command '{}' must use actor_reference or first_actor_in_room target_mode",
                    command.id
                )
                .into());
            }
        } else {
            if has_move_effect && metadata.target_mode != PlayerCommandTargetMode::RoomReference {
                return Err(format!(
                    "move player command '{}' must define player_command.target_mode room_reference",
                    command.id
                )
                .into());
            }
            if has_target_observe_effect
                && metadata.target_mode != PlayerCommandTargetMode::ActorOrFeatureReference
            {
                return Err(format!(
                    "observe player command '{}' must define player_command.target_mode actor_or_feature_reference",
                    command.id
                )
                .into());
            }
            if !has_move_effect
                && !has_target_observe_effect
                && metadata.target_mode != PlayerCommandTargetMode::None
            {
                return Err(format!(
                    "non-targeted player command '{}' must not define player_command.target_mode",
                    command.id
                )
                .into());
            }
            if command.effects.is_empty() && command.content_event.is_none() {
                return Err(format!(
                    "generic player command '{}' must define effects or content_event",
                    command.id
                )
                .into());
            }
            if let Some(input) = metadata.input.as_ref()
                && input.required
                && input.payload_key.trim().is_empty()
                && command.content_event.is_some()
            {
                return Err(format!(
                    "content-event player command '{}' must define input.payload_key",
                    command.id
                )
                .into());
            }
        }
        if has_move_effect && command.target_mode != CommandTargetMode::Room {
            return Err(format!("move command '{}' must use target_mode room", command.id).into());
        }
        if command.has_effect(CommandEffect::ObserveFeature)
            && command.target_mode != CommandTargetMode::Feature
        {
            return Err(format!(
                "observe-feature command '{}' must use target_mode feature",
                command.id
            )
            .into());
        }
        if command.has_effect(CommandEffect::ObserveActor)
            && command.target_mode != CommandTargetMode::Actor
        {
            return Err(format!(
                "observe-actor command '{}' must use target_mode actor",
                command.id
            )
            .into());
        }
        if has_target_memory_effect && command.target_mode != CommandTargetMode::Actor {
            return Err(format!(
                "target-actor memory command '{}' must use target_mode actor",
                command.id
            )
            .into());
        }
        if has_observe_room_effect && command.target_mode != CommandTargetMode::None {
            return Err(format!(
                "observe-room command '{}' must use target_mode none",
                command.id
            )
            .into());
        }
        if command.consumable_kind.is_some() && command.target_mode != CommandTargetMode::Consumable
        {
            return Err(format!(
                "consumable command '{}' must use target_mode consumable",
                command.id
            )
            .into());
        }
        if command.has_effect(CommandEffect::ConsumeTargetedConsumable)
            && command.target_mode != CommandTargetMode::Consumable
        {
            return Err(format!(
                "consume-targeted-consumable command '{}' must use target_mode consumable",
                command.id
            )
            .into());
        }
    }
    Ok(())
}
