use crate::content::types::{ActionDefinition, CommandEffect, CommandTargetMode};
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

pub(crate) fn validate_actions(
    actions: &[ActionDefinition],
    known_room_ids: &[&str],
    known_stage_ids: &[&str],
) -> Result<(), Box<dyn Error>> {
    for action in actions {
        if action.player_enabled && !action.phrases.is_empty() {
            let metadata = action.player_command.as_ref().ok_or_else(|| {
                format!(
                    "player-enabled action '{}' with phrases must define player_command metadata",
                    action.id
                )
            })?;
            if metadata.usage.trim().is_empty() {
                return Err(format!(
                    "player-enabled action '{}' must define player_command.usage",
                    action.id
                )
                .into());
            }
            if metadata.example.trim().is_empty() {
                return Err(format!(
                    "player-enabled action '{}' must define player_command.example",
                    action.id
                )
                .into());
            }
        }
        if action.npc.is_some() && action.command.trim().is_empty() {
            return Err(format!(
                "NPC action '{}' must define a command field",
                action.id
            )
            .into());
        }
        for room_id in &action.available.allowed_rooms {
            require_known_id(
                room_id,
                known_room_ids,
                &format!("action '{}' allowed_room '{}'", action.id, room_id),
                "rooms",
            )?;
        }
        for stage_id in &action.available.available_during {
            require_known_id(
                stage_id,
                known_stage_ids,
                &format!("action '{}' available_during '{}'", action.id, stage_id),
                "beats.stages",
            )?;
        }
        let has_move_effect = action.has_effect(CommandEffect::MoveActor);
        let has_observe_room_effect = action.has_effect(CommandEffect::ObserveRoom);
        if has_move_effect && action.target_mode != CommandTargetMode::Room {
            return Err(format!("move action '{}' must use target_mode room", action.id).into());
        }
        if has_observe_room_effect && action.target_mode != CommandTargetMode::None {
            return Err(format!(
                "observe-room action '{}' must use target_mode none",
                action.id
            )
            .into());
        }
    }
    Ok(())
}
