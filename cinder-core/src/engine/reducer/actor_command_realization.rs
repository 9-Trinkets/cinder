//! Realization-time checks and side effects for actor commands.
//!
//! A command's [`CommandEffect`] list mixes two concerns: some effects are
//! gated on preconditions (does the actor carry the item? is the target
//! attackable?) and some apply a small side effect while doing so (marking a
//! feature seen, consuming a room item). Each effect is modelled as an
//! [`EffectRealizer`] so the dispatch stays flat and every rule can be read
//! and tested in isolation.

use super::actor_commands::ActorCommandContext;
use crate::content::types::{
    ActionDefinition, ActionItemStorageTarget, CommandEffect, ContentPack, ItemStorageTarget,
};
use crate::engine::state::{ActorStance, WorldState};

/// Whether the command's `requires_item` prerequisite is satisfied.
fn required_item_present(
    state: &WorldState,
    command: &ActionDefinition,
    context: &ActorCommandContext<'_>,
) -> bool {
    let Some(item_id) = &command.available.requires_item else {
        return true;
    };
    let storage = match command.available.requires_item_storage {
        ActionItemStorageTarget::PlayerInventory => ItemStorageTarget::PlayerInventory,
        ActionItemStorageTarget::CurrentRoom => ItemStorageTarget::CurrentRoom,
    };
    state.has_item_in_storage(item_id, storage, context.room_id)
}

/// Validates and applies one command effect at realization time.
trait EffectRealizer: Sync {
    fn realize(
        &self,
        state: &mut WorldState,
        content: &ContentPack,
        command: &ActionDefinition,
        context: &ActorCommandContext<'_>,
    ) -> bool;
}

/// Map an effect onto its realizer.
fn realizer(effect: &CommandEffect) -> &'static dyn EffectRealizer {
    match effect {
        CommandEffect::ObserveFeature => &ObserveFeatureRealizer,
        CommandEffect::ObserveActor => &ObserveActorRealizer,
        CommandEffect::UseInteraction => &UseInteractionRealizer,
        CommandEffect::ConsumeTargetedConsumable => &ConsumeTargetedConsumableRealizer,
        CommandEffect::DropItem => &DropItemRealizer,
        CommandEffect::PickUpItem => &PickUpItemRealizer,
        CommandEffect::EquipItem => &EquipItemRealizer,
        CommandEffect::UnequipItem => &UnequipItemRealizer,
        CommandEffect::UseItem => &UseItemRealizer,
        CommandEffect::AttackTarget => &AttackTargetRealizer,
        CommandEffect::MoveActor
        | CommandEffect::ObserveRoom
        | CommandEffect::RememberInRoom
        | CommandEffect::RememberWithTargetActor
        | CommandEffect::FollowActor => &NoOpRealizer,
    }
}

/// Apply all realization checks for `command`, returning `false` as soon as one
/// effect cannot be realized.
pub(super) fn apply_actor_command_realization_effects(
    state: &mut WorldState,
    content: &ContentPack,
    command: &ActionDefinition,
    context: &ActorCommandContext<'_>,
) -> bool {
    if !required_item_present(state, command, context) {
        return false;
    }
    command
        .effects
        .iter()
        .all(|effect| realizer(effect).realize(state, content, command, context))
}

struct ObserveFeatureRealizer;

impl EffectRealizer for ObserveFeatureRealizer {
    fn realize(
        &self,
        state: &mut WorldState,
        content: &ContentPack,
        _command: &ActionDefinition,
        context: &ActorCommandContext<'_>,
    ) -> bool {
        let Some(feature_id) = context.feature_id else {
            return false;
        };
        let Some(feature) = content.feature(context.room_id, feature_id) else {
            return false;
        };
        state.mark_actor_feature_seen(context.actor_id, context.room_id, feature_id);
        state.push_actor_observation_note(context.actor_id, feature.inspect_text.clone());
        true
    }
}

struct ObserveActorRealizer;

impl EffectRealizer for ObserveActorRealizer {
    fn realize(
        &self,
        state: &mut WorldState,
        content: &ContentPack,
        _command: &ActionDefinition,
        context: &ActorCommandContext<'_>,
    ) -> bool {
        let Some(target_actor_id) = context.target_actor_id else {
            return false;
        };
        let Some(target_actor) = content.actor(target_actor_id) else {
            return false;
        };
        state.mark_actor_studied_actor(context.actor_id, target_actor_id);
        state.push_actor_observation_note(context.actor_id, target_actor.inspect_text.clone());
        true
    }
}

struct UseInteractionRealizer;

impl EffectRealizer for UseInteractionRealizer {
    fn realize(
        &self,
        _state: &mut WorldState,
        _content: &ContentPack,
        _command: &ActionDefinition,
        _context: &ActorCommandContext<'_>,
    ) -> bool {
        // Interactions are chosen by the player from the Talk and Do panels.
        false
    }
}

struct ConsumeTargetedConsumableRealizer;

impl EffectRealizer for ConsumeTargetedConsumableRealizer {
    fn realize(
        &self,
        state: &mut WorldState,
        _content: &ContentPack,
        _command: &ActionDefinition,
        context: &ActorCommandContext<'_>,
    ) -> bool {
        let (Some(feature_id), Some(consumable_id)) = (context.feature_id, context.consumable_id)
        else {
            return false;
        };
        state.remove_feature_item(context.room_id, feature_id, consumable_id)
            || state.remove_item_from_storage(
                consumable_id,
                ItemStorageTarget::CurrentRoom,
                context.room_id,
            )
    }
}

struct DropItemRealizer;

impl EffectRealizer for DropItemRealizer {
    fn realize(
        &self,
        state: &mut WorldState,
        _content: &ContentPack,
        command: &ActionDefinition,
        _context: &ActorCommandContext<'_>,
    ) -> bool {
        !command.item_id.is_empty() && state.has_item(&command.item_id)
    }
}

struct PickUpItemRealizer;

impl EffectRealizer for PickUpItemRealizer {
    fn realize(
        &self,
        state: &mut WorldState,
        _content: &ContentPack,
        command: &ActionDefinition,
        context: &ActorCommandContext<'_>,
    ) -> bool {
        !command.item_id.is_empty()
            && state.has_item_in_storage(
                &command.item_id,
                ItemStorageTarget::CurrentRoom,
                context.room_id,
            )
    }
}

struct EquipItemRealizer;

impl EffectRealizer for EquipItemRealizer {
    fn realize(
        &self,
        state: &mut WorldState,
        content: &ContentPack,
        command: &ActionDefinition,
        _context: &ActorCommandContext<'_>,
    ) -> bool {
        let Some(item) = content.item(&command.item_id) else {
            return false;
        };
        item.is_equippable()
            && state.has_item(&command.item_id)
            && item
                .occupied_slots()
                .iter()
                .all(|slot| content.settings.equipment_slots.contains(slot))
            && !state.item_is_equipped(item)
    }
}

struct UnequipItemRealizer;

impl EffectRealizer for UnequipItemRealizer {
    fn realize(
        &self,
        state: &mut WorldState,
        content: &ContentPack,
        command: &ActionDefinition,
        _context: &ActorCommandContext<'_>,
    ) -> bool {
        let Some(item) = content.item(&command.item_id) else {
            return false;
        };
        state.item_is_equipped(item)
    }
}

struct UseItemRealizer;

impl EffectRealizer for UseItemRealizer {
    fn realize(
        &self,
        state: &mut WorldState,
        content: &ContentPack,
        command: &ActionDefinition,
        _context: &ActorCommandContext<'_>,
    ) -> bool {
        let Some(item) = content.item(&command.item_id) else {
            return false;
        };
        !item.use_hook.is_empty() && state.has_item(&command.item_id)
    }
}

struct AttackTargetRealizer;

impl EffectRealizer for AttackTargetRealizer {
    fn realize(
        &self,
        state: &mut WorldState,
        content: &ContentPack,
        _command: &ActionDefinition,
        context: &ActorCommandContext<'_>,
    ) -> bool {
        let Some(target_actor_id) = context.target_actor_id else {
            return false;
        };
        content
            .actor(target_actor_id)
            .is_some_and(|actor| actor.attackable)
            && state.actor_stat(target_actor_id, &content.settings.combat.health_stat_id) > 0
            && state.stance(target_actor_id) != ActorStance::Allied
    }
}

struct NoOpRealizer;

impl EffectRealizer for NoOpRealizer {
    fn realize(
        &self,
        _state: &mut WorldState,
        _content: &ContentPack,
        _command: &ActionDefinition,
        _context: &ActorCommandContext<'_>,
    ) -> bool {
        true
    }
}
