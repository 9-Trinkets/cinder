use crate::content::types::{ActionDefinition, ConsumableKind, ContentPack};
use crate::engine::dialogue::ActorTurnAffordanceOption;
use std::error::Error;

#[derive(Debug, Clone)]
pub(crate) struct ActorAffordanceCandidate {
    pub(crate) order: usize,
    pub(crate) visible_by_default: bool,
    pub(crate) option: ActorTurnAffordanceOption,
}

impl ActorAffordanceCandidate {
    pub(crate) fn new(
        action: &ActionDefinition,
        option: ActorTurnAffordanceOption,
    ) -> Self {
        Self {
            order: action.ui.sort_order,
            visible_by_default: action
                .npc
                .as_ref()
                .map_or(true, |n| n.visible_by_default),
            option,
        }
    }
}

pub(crate) fn require_actor_affordance_for_command_id<'a>(
    content: &'a ContentPack,
    command_id: &str,
) -> Result<&'a ActionDefinition, Box<dyn Error>> {
    content
        .actions
        .iter()
        .find(|action| action.id == command_id && action.npc.is_some())
        .ok_or_else(|| {
            Box::new(std::io::Error::other(format!(
                "missing actor affordance for command '{command_id}'"
            ))) as Box<dyn Error>
        })
}

pub(crate) fn require_actor_affordance_for_consumable_kind(
    content: &ContentPack,
    consumable_kind: ConsumableKind,
) -> Result<&ActionDefinition, Box<dyn Error>> {
    content
        .actions
        .iter()
        .find(|action| {
            action.consumable_kind == Some(consumable_kind) && action.npc.is_some()
        })
        .ok_or_else(|| {
            Box::new(std::io::Error::other(format!(
                "missing actor affordance for consumable kind '{consumable_kind:?}'"
            ))) as Box<dyn Error>
        })
}
