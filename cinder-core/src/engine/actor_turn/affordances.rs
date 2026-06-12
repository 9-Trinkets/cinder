use crate::content::types::{AffordanceDefinition, CommandDefinition, ConsumableKind, ContentPack};
use crate::engine::dialogue::ActorTurnAffordanceOption;
use crate::engine::hooks::ActorTurnGuidanceAffordanceInput;
use std::collections::BTreeMap;
use std::error::Error;

#[derive(Debug, Clone)]
pub(crate) struct ActorAffordanceCandidate {
    pub(crate) order: usize,
    pub(crate) visible_by_default: bool,
    pub(crate) option: ActorTurnAffordanceOption,
}

impl ActorAffordanceCandidate {
    pub(crate) fn new(
        definition: &AffordanceDefinition,
        option: ActorTurnAffordanceOption,
    ) -> Self {
        Self {
            order: definition.sort_order,
            visible_by_default: definition.visible_by_default,
            option,
        }
    }
}

pub(crate) fn guidance_affordance_inputs(
    content: &ContentPack,
    affordances: &[ActorAffordanceCandidate],
) -> BTreeMap<String, ActorTurnGuidanceAffordanceInput> {
    let mut inputs = content
        .affordances
        .actions
        .iter()
        .map(|definition| {
            (
                definition.id.clone(),
                ActorTurnGuidanceAffordanceInput {
                    available: false,
                    option_count: 0,
                    group: definition.group.clone(),
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    for affordance in affordances {
        let entry = inputs
            .entry(affordance.option.affordance_id.clone())
            .or_insert_with(|| ActorTurnGuidanceAffordanceInput {
                available: true,
                option_count: 0,
                group: affordance.option.group.clone(),
            });
        entry.available = true;
        entry.option_count += 1;
    }
    inputs
}

pub(crate) fn require_affordance_command<'a>(
    content: &'a ContentPack,
    affordance: &AffordanceDefinition,
) -> Result<&'a CommandDefinition, Box<dyn Error>> {
    content.command(&affordance.command_id).ok_or_else(|| {
        Box::new(std::io::Error::other(format!(
            "missing command '{}' for affordance '{}'",
            affordance.command_id, affordance.id
        ))) as Box<dyn Error>
    })
}

pub(crate) fn require_actor_affordance_for_command_id<'a>(
    content: &'a ContentPack,
    command_id: &str,
) -> Result<(&'a AffordanceDefinition, &'a CommandDefinition), Box<dyn Error>> {
    content
        .affordances
        .actions
        .iter()
        .find_map(|affordance| {
            let command = content.command(&affordance.command_id)?;
            (command.id == command_id).then_some((affordance, command))
        })
        .ok_or_else(|| {
            Box::new(std::io::Error::other(format!(
                "missing actor affordance for command '{command_id}'"
            ))) as Box<dyn Error>
        })
}

pub(crate) fn require_actor_affordance_for_consumable_kind(
    content: &ContentPack,
    consumable_kind: ConsumableKind,
) -> Result<(&AffordanceDefinition, &CommandDefinition), Box<dyn Error>> {
    content
        .affordances
        .actions
        .iter()
        .find_map(|affordance| {
            let command = content.command(&affordance.command_id)?;
            (command.consumable_kind == Some(consumable_kind)).then_some((affordance, command))
        })
        .ok_or_else(|| {
            Box::new(std::io::Error::other(format!(
                "missing actor affordance for consumable kind '{consumable_kind:?}'"
            ))) as Box<dyn Error>
        })
}
