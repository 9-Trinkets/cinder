use crate::content::types::{ActionDefinition, CommandEffect, ContentPack, ItemStorageTarget};
use crate::engine::events::WorldEvent;
use crate::engine::narrative::NarrativeLines;
use crate::engine::state::WorldState;

use super::combat::apply_attack_target;
use super::equipment::{apply_equip, apply_unequip, apply_use_item};

#[allow(unused_imports)]
pub(super) use super::actor_commands::{
    ActorCommandContext, apply_actor_command_effects, apply_actor_command_realization_effects,
    handle_actor_command_used, record_actor_command_memory, render_actor_command_text,
    resolve_actor_command_labels,
};
#[allow(unused_imports)]
pub(super) use super::combat::{
    actor_display_name, award_defeat_xp, defeat_actor, defeat_player_if_dead, spawn_defeat_drops,
};
pub(super) use super::movement::{ActorMoveTransitionContext, apply_actor_move_transition};
pub(super) use super::surround::trigger_surrounded_hooks;

pub(super) fn apply_new_command_effects(
    state: &mut WorldState,
    content: &ContentPack,
    command: &ActionDefinition,
    context: &ActorCommandContext<'_>,
    lines: &mut NarrativeLines,
    _outbox: &mut Vec<WorldEvent>,
) {
    if command.has_effect(CommandEffect::DropItem) {
        let room_id = context.room_id.to_string();
        if state.remove_item(&command.item_id) {
            state.add_item_to_storage(&command.item_id, ItemStorageTarget::CurrentRoom, &room_id);
            trigger_surrounded_hooks(state, content, &command.item_id, &room_id, lines);
        }
    }
    if command.has_effect(CommandEffect::PickUpItem) {
        let room_id = context.room_id.to_string();
        if state.remove_item_from_storage(
            &command.item_id,
            ItemStorageTarget::CurrentRoom,
            &room_id,
        ) {
            state.add_item(&command.item_id);
        }
    }
    if command.has_effect(CommandEffect::EquipItem) {
        apply_equip(state, content, &command.item_id, lines);
    }
    if command.has_effect(CommandEffect::UnequipItem) {
        apply_unequip(state, content, &command.item_id, lines);
    }
    if command.has_effect(CommandEffect::UseItem) {
        apply_use_item(state, content, command, lines);
    }
    if command.has_effect(CommandEffect::AttackTarget) {
        let Some(target_actor_id) = context.target_actor_id else {
            return;
        };
        apply_attack_target(
            state,
            content,
            context.actor_id,
            target_actor_id,
            context.room_id,
            lines,
        );
    }
}
