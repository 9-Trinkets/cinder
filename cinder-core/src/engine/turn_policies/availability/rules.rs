use crate::content::types::{
    ActionAvailability, ActionItemConsumerTarget, ActionItemStorageTarget, CommandEffect,
    ContentPack, ItemStorageTarget,
};
use crate::engine::state::WorldState;
use crate::engine::turn_policies::objectives::{
    objective_progress_is_met, objective_progress_label,
};

use super::{AvailabilityContext, AvailabilityRule, CommandAvailabilityIssue, CommandItemState};

struct SkillGate;

impl AvailabilityRule for SkillGate {
    fn check(&self, ctx: &AvailabilityContext<'_>) -> Result<(), CommandAvailabilityIssue> {
        if !ctx.action.skill_id.is_empty()
            && !ctx
                .state
                .actor_has_skill(ctx.actor_id, &ctx.action.skill_id)
        {
            return Err(CommandAvailabilityIssue::MissingSkill);
        }
        Ok(())
    }
}

struct StageGate;

impl AvailabilityRule for StageGate {
    fn check(&self, ctx: &AvailabilityContext<'_>) -> Result<(), CommandAvailabilityIssue> {
        let available = &ctx.action.available;
        if !available.available_during.is_empty()
            && !available
                .available_during
                .iter()
                .any(|stage_id| ctx.state.active_objective_stage_ids.contains(stage_id))
        {
            return Err(CommandAvailabilityIssue::StageInactive);
        }
        Ok(())
    }
}

struct ObjectiveProgressGate;

impl AvailabilityRule for ObjectiveProgressGate {
    fn check(&self, ctx: &AvailabilityContext<'_>) -> Result<(), CommandAvailabilityIssue> {
        let available = &ctx.action.available;
        let missing = available
            .required_objective_progress
            .iter()
            .filter(|progress| !objective_progress_is_met(ctx.content, ctx.state, progress))
            .map(|progress| objective_progress_label(ctx.content, progress))
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(CommandAvailabilityIssue::MissingObjectiveProgress(missing));
        }
        let blocked = available
            .blocked_by_objective_progress
            .iter()
            .filter(|progress| objective_progress_is_met(ctx.content, ctx.state, progress))
            .map(|progress| objective_progress_label(ctx.content, progress))
            .collect::<Vec<_>>();
        if !blocked.is_empty() {
            return Err(CommandAvailabilityIssue::BlockedByObjectiveProgress(
                blocked,
            ));
        }
        Ok(())
    }
}

struct ConditionGate;

impl AvailabilityRule for ConditionGate {
    fn check(&self, ctx: &AvailabilityContext<'_>) -> Result<(), CommandAvailabilityIssue> {
        let conditions = &ctx.action.available.when;
        if !conditions.is_empty()
            && !crate::engine::conditions::all_hold(
                conditions,
                &crate::engine::conditions::ConditionContext::new(
                    ctx.content,
                    ctx.state,
                    ctx.room_id,
                ),
            )
        {
            return Err(CommandAvailabilityIssue::ConditionNotMet);
        }
        Ok(())
    }
}

struct RoomRestrictionGate;

impl AvailabilityRule for RoomRestrictionGate {
    fn check(&self, ctx: &AvailabilityContext<'_>) -> Result<(), CommandAvailabilityIssue> {
        let available = &ctx.action.available;
        if !available.allowed_rooms.is_empty()
            && !available
                .allowed_rooms
                .iter()
                .any(|room| room == ctx.room_id)
        {
            return Err(CommandAvailabilityIssue::RoomRestricted);
        }
        Ok(())
    }
}

struct ItemPossessionGate;

impl AvailabilityRule for ItemPossessionGate {
    fn check(&self, ctx: &AvailabilityContext<'_>) -> Result<(), CommandAvailabilityIssue> {
        let available = &ctx.action.available;
        if let Some(item_id) = &available.requires_item
            && !ctx.state.has_item_in_storage(
                item_id,
                to_item_storage(available.requires_item_storage.clone()),
                ctx.room_id,
            )
        {
            return Err(CommandAvailabilityIssue::MissingItem(item_id.clone()));
        }
        if let Some(item_id) = &available.consumes_item {
            if !ctx.state.has_item_in_storage(
                item_id,
                to_item_storage(available.consumes_item_storage.clone()),
                ctx.room_id,
            ) {
                return Err(CommandAvailabilityIssue::MissingItem(item_id.clone()));
            }
            if consumed_by_present_actor(ctx)
                && first_actor_in_room(ctx.content, ctx.state, ctx.room_id).is_none()
            {
                return Err(CommandAvailabilityIssue::NoActorToServe);
            }
        }
        check_any_candidates(ctx, available)
    }
}

fn check_any_candidates(
    ctx: &AvailabilityContext<'_>,
    available: &ActionAvailability,
) -> Result<(), CommandAvailabilityIssue> {
    if available.requires_any.is_empty() && available.consumes_any.is_empty() {
        return Ok(());
    }
    let has_any = available
        .requires_any
        .iter()
        .chain(available.consumes_any.iter())
        .any(|item_id| {
            let storage = if available.consumes_any.iter().any(|c| c == item_id) {
                to_item_storage(available.consumes_any_storage.clone())
            } else {
                to_item_storage(available.requires_any_storage.clone())
            };
            ctx.state.has_item_in_storage(item_id, storage, ctx.room_id)
        });
    if !has_any {
        return Err(if available.consumes_any.is_empty() {
            match available.requires_any.first() {
                Some(item_id) => CommandAvailabilityIssue::MissingItem(item_id.clone()),
                None => CommandAvailabilityIssue::NothingToConsume,
            }
        } else {
            CommandAvailabilityIssue::NothingToConsume
        });
    }
    if !available.consumes_any.is_empty()
        && consumed_by_present_actor(ctx)
        && first_actor_in_room(ctx.content, ctx.state, ctx.room_id).is_none()
    {
        return Err(CommandAvailabilityIssue::NoActorToServe);
    }
    Ok(())
}

/// Item-state gating derived from the effect, so content never has to restate
/// the obvious: equipping/using needs the item in the inventory, unequipping
/// needs it worn, dropping needs it held.
struct EffectStateGate;

impl AvailabilityRule for EffectStateGate {
    fn check(&self, ctx: &AvailabilityContext<'_>) -> Result<(), CommandAvailabilityIssue> {
        let action = ctx.action;
        if action.has_effect(CommandEffect::DropItem)
            && !action.item_id.is_empty()
            && !ctx.state.has_item(&action.item_id)
        {
            return Err(CommandAvailabilityIssue::ItemState(
                CommandItemState::NoneHeld(action.item_id.clone()),
            ));
        }
        if action.has_effect(CommandEffect::EquipItem) {
            let equipped = ctx
                .content
                .item(&action.item_id)
                .is_some_and(|item| ctx.state.item_is_equipped(item));
            if equipped {
                return Err(CommandAvailabilityIssue::ItemState(
                    CommandItemState::AlreadyEquipped(action.item_id.clone()),
                ));
            }
            if !ctx.state.has_item(&action.item_id) {
                return Err(CommandAvailabilityIssue::ItemState(
                    CommandItemState::NoneHeld(action.item_id.clone()),
                ));
            }
        }
        if action.has_effect(CommandEffect::UnequipItem) {
            let equipped = ctx
                .content
                .item(&action.item_id)
                .is_some_and(|item| ctx.state.item_is_equipped(item));
            if !equipped {
                return Err(CommandAvailabilityIssue::ItemState(
                    CommandItemState::NotEquipped(action.item_id.clone()),
                ));
            }
        }
        if action.has_effect(CommandEffect::UseItem) && !ctx.state.has_item(&action.item_id) {
            return Err(CommandAvailabilityIssue::ItemState(
                CommandItemState::NoneHeld(action.item_id.clone()),
            ));
        }
        Ok(())
    }
}

/// The ordered rule chain shared by every availability query. Later entries only
/// run when earlier ones pass, so the first failure is the most fundamental
/// blocker and its message is the most useful one.
pub(super) fn availability_rules() -> [&'static dyn AvailabilityRule; 7] {
    [
        &SkillGate,
        &StageGate,
        &ObjectiveProgressGate,
        &ConditionGate,
        &RoomRestrictionGate,
        &ItemPossessionGate,
        &EffectStateGate,
    ]
}

fn to_item_storage(storage: ActionItemStorageTarget) -> ItemStorageTarget {
    match storage {
        ActionItemStorageTarget::PlayerInventory => ItemStorageTarget::PlayerInventory,
        ActionItemStorageTarget::CurrentRoom => ItemStorageTarget::CurrentRoom,
    }
}

fn consumed_by_present_actor(ctx: &AvailabilityContext<'_>) -> bool {
    ctx.action.item_consumer == ActionItemConsumerTarget::FirstActorInRoom
}

fn first_actor_in_room(
    content: &ContentPack,
    state: &WorldState,
    room_id: &str,
) -> Option<(String, String)> {
    state
        .actors(content)
        .find(|actor| {
            !state.actor_is_defeated(&actor.id, &content.settings.combat.health_stat_id)
                && state.actor_room_id(&actor.id, &actor.room_id) == room_id
        })
        .map(|actor| (actor.id.clone(), actor.name.clone()))
}
