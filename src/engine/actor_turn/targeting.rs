use super::builder::ActorTurnRealizationContext;
use crate::content::types::{CommandDefinition, CommandTargetMode, ContentPack};
use std::error::Error;

pub(crate) fn resolve_target_actor_name(
    realization_context: &ActorTurnRealizationContext,
    target_actor_id: Option<&str>,
) -> Result<Option<String>, Box<dyn Error>> {
    let Some(target_actor_id) = target_actor_id else {
        return Ok(None);
    };
    realization_context
        .talk_targets
        .iter()
        .chain(realization_context.inspect_actor_targets.iter())
        .find(|candidate| candidate.actor_id == target_actor_id)
        .map(|candidate| Some(candidate.actor_name.clone()))
        .ok_or_else(|| {
            Box::new(std::io::Error::other(format!(
                "missing command target '{target_actor_id}'"
            ))) as Box<dyn Error>
        })
}

pub(crate) fn validate_command_target_contract(
    command: &CommandDefinition,
    target_room_id: Option<&str>,
    target_actor_id: Option<&str>,
    context_label: Option<&str>,
    feature_id: Option<&str>,
    consumable_id: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    match command.target_mode {
        CommandTargetMode::None => {
            if target_room_id.is_some()
                || target_actor_id.is_some()
                || context_label.is_some()
                || feature_id.is_some()
                || consumable_id.is_some()
            {
                return Err(Box::new(std::io::Error::other(format!(
                    "command '{}' does not accept runtime targets",
                    command.id
                ))));
            }
        }
        CommandTargetMode::Room => {
            if target_room_id.is_none() {
                return Err(Box::new(std::io::Error::other(format!(
                    "command '{}' missing room target",
                    command.id
                ))));
            }
        }
        CommandTargetMode::Actor => {
            if target_actor_id.is_none() {
                return Err(Box::new(std::io::Error::other(format!(
                    "command '{}' missing actor target",
                    command.id
                ))));
            }
        }
        CommandTargetMode::ActorOptional => {}
        CommandTargetMode::Feature => {
            if feature_id.is_none() {
                return Err(Box::new(std::io::Error::other(format!(
                    "command '{}' missing feature target",
                    command.id
                ))));
            }
        }
        CommandTargetMode::Consumable => {
            if consumable_id.is_none() {
                return Err(Box::new(std::io::Error::other(format!(
                    "command '{}' missing consumable target",
                    command.id
                ))));
            }
        }
        CommandTargetMode::ContextLabel => {
            if context_label.is_none_or(|label| label.trim().is_empty()) {
                return Err(Box::new(std::io::Error::other(format!(
                    "command '{}' missing context label target",
                    command.id
                ))));
            }
        }
    }
    Ok(())
}

pub(crate) fn resolve_inspect_actor_name(
    realization_context: &ActorTurnRealizationContext,
    target_actor_id: Option<&str>,
) -> Result<Option<String>, Box<dyn Error>> {
    let Some(target_actor_id) = target_actor_id else {
        return Ok(None);
    };
    realization_context
        .inspect_actor_targets
        .iter()
        .find(|candidate| candidate.actor_id == target_actor_id)
        .map(|candidate| Some(candidate.actor_name.clone()))
        .ok_or_else(|| {
            Box::new(std::io::Error::other(format!(
                "missing inspect candidate '{target_actor_id}'"
            ))) as Box<dyn Error>
        })
}

pub(crate) fn resolve_command_consumable_fields(
    content: &ContentPack,
    current_room_id: &str,
    feature_id: Option<String>,
    consumable_id: Option<String>,
) -> Result<(Option<String>, Option<String>), Box<dyn Error>> {
    let Some(consumable_id) = consumable_id else {
        return Ok((feature_id, None));
    };
    if let Some(feature_id) = feature_id {
        if content
            .room_consumable(current_room_id, &feature_id, &consumable_id)
            .is_some()
        {
            return Ok((Some(feature_id), Some(consumable_id)));
        }
        return Err(Box::new(std::io::Error::other(format!(
            "missing consumable '{consumable_id}' at feature '{feature_id}' in room '{current_room_id}'"
        ))));
    }
    let Some(consumable) = content
        .room_consumables(current_room_id)
        .into_iter()
        .find(|candidate| candidate.consumable.id == consumable_id)
    else {
        return Err(Box::new(std::io::Error::other(format!(
            "missing consume candidate '{consumable_id}' in room '{current_room_id}'"
        ))));
    };
    Ok((
        Some(consumable.feature.id.clone()),
        Some(consumable.consumable.id.clone()),
    ))
}

pub(crate) fn validate_observe_feature_command(
    realization_context: &ActorTurnRealizationContext,
    feature_id: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    if realization_context.hide_inspect_feature {
        return Err(Box::new(std::io::Error::other(
            "actor turn decider chose INSPECT FEATURE while hidden exploration actions are unavailable",
        )));
    }
    let Some(feature_id) = feature_id else {
        return Err(Box::new(std::io::Error::other(
            "inspect-feature command missing feature target",
        )));
    };
    if realization_context
        .inspect_feature_ids
        .iter()
        .any(|candidate| candidate == feature_id)
    {
        return Ok(());
    }

    Err(Box::new(std::io::Error::other(format!(
        "missing feature '{feature_id}' in room '{}'",
        realization_context.current_room_id
    ))))
}
