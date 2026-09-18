use crate::content::types::{
    ActionDefinition, ActionItemStorageTarget, CommandEffect, CommandInputMode, CommandTargetMode,
    ContentPack, ItemStorageTarget,
};
use crate::engine::events::WorldEvent;
use crate::engine::hooks::apply_world_hook_effects;
use crate::engine::narrative::NarrativeLines;
use crate::engine::state::{
    ActorStance, ConversationMemoryKind, ConversationMemoryLine, WorldState,
};
use crate::engine::turn_policies::apply_command_objective_progress_effects;
use serde_json::json;

use super::beat_advance::advance_objective_for_signal;
use super::command_effects::apply_new_command_effects;
use super::surround::trigger_surrounded_hooks;
use super::movement::{ActorMoveTransitionContext, apply_actor_move_transition};
use super::tick::record_room_action_memory;

pub(super) struct ActorCommandContext<'a> {
    pub(super) actor_id: &'a str,
    pub(super) actor_name: &'a str,
    pub(super) room_id: &'a str,
    pub(super) target_room_id: Option<&'a str>,
    pub(super) target_actor_id: Option<&'a str>,
    pub(super) target_actor_name: Option<&'a str>,
    pub(super) context_label: Option<&'a str>,
    pub(super) feature_id: Option<&'a str>,
    pub(super) consumable_id: Option<&'a str>,
    pub(super) freeform_text: Option<&'a str>,
}

pub(super) fn handle_actor_command_used(
    state: &mut WorldState,
    content: &ContentPack,
    command_id: &str,
    command_context: &ActorCommandContext<'_>,
    outbox: &mut Vec<WorldEvent>,
) -> Option<NarrativeLines> {
    let mut lines = NarrativeLines::default();
    let previous_current_room_id = state.current_room_id.clone();
    let command = content.command(command_id)?;
    let created_item_id = resolved_created_item_id(state, content, command, command_context);
    if command
        .item_creation
        .as_ref()
        .is_some_and(|creation| !creation.craftable_items.is_empty())
        && created_item_id.is_none()
    {
        return None;
    }
    if created_item_id.as_deref().is_some_and(|item_id| {
        command
            .item_creation
            .as_ref()
            .is_some_and(|creation| creation.storage == ActionItemStorageTarget::CurrentRoom)
            && content.item(item_id).is_some_and(|item| item.trace_mark)
            && state.has_item_in_storage(
                item_id,
                ItemStorageTarget::CurrentRoom,
                command_context.room_id,
            )
    }) {
        return None;
    }
    let (item_label, feature_label) = resolve_actor_command_labels(content, command_context)?;
    if !apply_actor_command_realization_effects(state, content, command, command_context) {
        return None;
    }
    let command_text = render_actor_command_text(
        content,
        command,
        command_context,
        &item_label,
        &feature_label,
    )?;
    record_actor_command_memory(state, content, command, command_context, &command_text);
    apply_actor_command_effects(state, content, command, command_context);
    if !command.has_effect(CommandEffect::MoveActor)
        && state.current_room_id == command_context.room_id
    {
        lines.narration(command_text.clone());
    }
    apply_new_command_effects(state, content, command, command_context, &mut lines, outbox);
    apply_command_objective_progress_effects(state, command);
    if let Some(item_id) = created_item_id {
        let storage = command
            .item_creation
            .as_ref()
            .map(|creation| match creation.storage {
                ActionItemStorageTarget::PlayerInventory => ItemStorageTarget::PlayerInventory,
                ActionItemStorageTarget::CurrentRoom => ItemStorageTarget::CurrentRoom,
            })
            .unwrap_or_default();
        state.add_item_to_storage(&item_id, storage, command_context.room_id);
        if storage == ItemStorageTarget::CurrentRoom {
            trigger_surrounded_hooks(
                state,
                content,
                &item_id,
                command_context.room_id,
                &mut lines,
            );
        }
    }
    if command.has_effect(CommandEffect::MoveActor) {
        let fixed_destination =
            (!command.destination_room_id.is_empty()).then(|| command.destination_room_id.clone());
        if let Some(to_room_id) =
            fixed_destination.or_else(|| command_context.target_room_id.map(str::to_string))
        {
            apply_actor_move_transition(
                state,
                content,
                ActorMoveTransitionContext {
                    actor_id: command_context.actor_id,
                    actor_name: Some(command_context.actor_name),
                    from_room_id: command_context.room_id,
                    to_room_id: &to_room_id,
                    command_text: Some(&command_text),
                },
                &mut lines,
            );
        } else if previous_current_room_id == command_context.room_id
            || state.followed_actor_id.as_deref() == Some(command_context.actor_id)
        {
            lines.narration(command_text);
        }
    }
    lines.extend_narration(advance_objective_for_signal(state, content, "command_used"));
    Some(lines)
}

fn resolved_created_item_id(
    state: &WorldState,
    content: &ContentPack,
    command: &ActionDefinition,
    context: &ActorCommandContext<'_>,
) -> Option<String> {
    let creation = command.item_creation.as_ref()?;
    let default_item_id = &creation.creates_item;
    let unlocked = |item_id: &str| match creation.craftable_item_gates.get(item_id) {
        None => true,
        Some(gate) if gate.is_empty() => true,
        Some(gate) => crate::engine::turn_policies::story_var_is_truthy(state, gate),
    };
    let available = |item_id: &str| {
        unlocked(item_id)
            && !(creation.storage == ActionItemStorageTarget::CurrentRoom
                && content.item(item_id).is_some_and(|item| item.trace_mark)
                && state.has_item_in_storage(
                    item_id,
                    ItemStorageTarget::CurrentRoom,
                    context.room_id,
                ))
    };
    if !creation.craftable_items.is_empty() {
        if let Some(input) = context
            .freeform_text
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let normalized = input.to_ascii_lowercase().replace(' ', "");
            if let Some(item_id) = creation
                .craftable_items
                .iter()
                .find(|item_id| item_id.eq_ignore_ascii_case(input) && unlocked(item_id))
            {
                return Some(item_id.clone());
            }
            if let Some(item_id) = creation.craftable_items.iter().find(|item_id| {
                unlocked(item_id)
                    && content.item(item_id).is_some_and(|item| {
                        item.label.to_ascii_lowercase().replace(' ', "") == normalized
                    })
            }) {
                return Some(item_id.clone());
            }
        }
        return creation
            .craftable_items
            .iter()
            .find(|item_id| available(item_id))
            .cloned();
    }
    if !creation.creates_item_target_template.is_empty() {
        return Some(creation.resolve_target_item_id(context.target_actor_id));
    }
    Some(
        (!creation.creates_item_story_var.is_empty())
            .then_some(creation.creates_item_story_var.as_str())
            .and_then(|key| state.story_vars.get(key))
            .map(str::to_string)
            .unwrap_or_else(|| default_item_id.clone()),
    )
}

pub(super) fn apply_actor_command_realization_effects(
    state: &mut WorldState,
    content: &ContentPack,
    command: &ActionDefinition,
    context: &ActorCommandContext<'_>,
) -> bool {
    if let Some(item_id) = &command.available.requires_item {
        let storage = match command.available.requires_item_storage {
            ActionItemStorageTarget::PlayerInventory => ItemStorageTarget::PlayerInventory,
            ActionItemStorageTarget::CurrentRoom => ItemStorageTarget::CurrentRoom,
        };
        if !state.has_item_in_storage(item_id, storage, context.room_id) {
            return false;
        }
    }
    for effect in &command.effects {
        match effect {
            CommandEffect::ObserveFeature => {
                let Some(feature_id) = context.feature_id else {
                    return false;
                };
                let Some(feature) = content.feature(context.room_id, feature_id) else {
                    return false;
                };
                state.mark_actor_feature_seen(context.actor_id, context.room_id, feature_id);
                state.push_actor_observation_note(context.actor_id, feature.inspect_text.clone());
            }
            CommandEffect::ObserveActor => {
                let Some(target_actor_id) = context.target_actor_id else {
                    return false;
                };
                let Some(target_actor) = content.actor(target_actor_id) else {
                    return false;
                };
                state.mark_actor_studied_actor(context.actor_id, target_actor_id);
                state.push_actor_observation_note(
                    context.actor_id,
                    target_actor.inspect_text.clone(),
                );
            }
            CommandEffect::ConsumeTargetedConsumable => {
                let (Some(feature_id), Some(consumable_id)) =
                    (context.feature_id, context.consumable_id)
                else {
                    return false;
                };
                if !state.consume_feature_consumable(context.room_id, feature_id, consumable_id)
                    && !state.remove_item_from_storage(
                        consumable_id,
                        ItemStorageTarget::CurrentRoom,
                        context.room_id,
                    )
                {
                    return false;
                }
            }
            CommandEffect::DropItem => {
                if command.item_id.is_empty() || !state.has_item(&command.item_id) {
                    return false;
                }
            }
            CommandEffect::PickUpItem => {
                if command.item_id.is_empty()
                    || !state.has_item_in_storage(
                        &command.item_id,
                        ItemStorageTarget::CurrentRoom,
                        context.room_id,
                    )
                {
                    return false;
                }
            }
            CommandEffect::EquipItem => {
                let Some(item) = content.item(&command.item_id) else {
                    return false;
                };
                if !item.is_equippable()
                    || !state.has_item(&command.item_id)
                    || !item.occupied_slots().iter().all(|slot| {
                        content.settings.equipment_slots.contains(slot)
                    })
                    || state.item_is_equipped(item)
                {
                    return false;
                }
            }
            CommandEffect::UnequipItem => {
                let Some(item) = content.item(&command.item_id) else {
                    return false;
                };
                if !state.item_is_equipped(item) {
                    return false;
                }
            }
            CommandEffect::UseItem => {
                let Some(item) = content.item(&command.item_id) else {
                    return false;
                };
                if item.use_hook.is_empty() || !state.has_item(&command.item_id) {
                    return false;
                }
            }
            CommandEffect::AttackTarget => {
                let Some(target_actor_id) = context.target_actor_id else {
                    return false;
                };
                if !content
                    .actor(target_actor_id)
                    .is_some_and(|actor| actor.attackable)
                    || state.actor_stat(target_actor_id, &content.settings.combat.health_stat_id)
                        <= 0
                    || state.stance(target_actor_id) == ActorStance::Allied
                {
                    return false;
                }
            }
            CommandEffect::MoveActor
            | CommandEffect::ObserveRoom
            | CommandEffect::RememberInRoom
            | CommandEffect::RememberWithTargetActor
            | CommandEffect::FollowActor => {}
        }
    }
    true
}

pub(super) fn apply_actor_command_effects(
    state: &mut WorldState,
    content: &ContentPack,
    command: &ActionDefinition,
    context: &ActorCommandContext<'_>,
) {
    if command.hook_id.is_empty() {
        return;
    }
    let mut input = json!({ "actor_id": context.actor_id, "room_id": context.room_id });
    for (key, value) in [
        ("target_actor_id", context.target_actor_id),
        ("context_label", context.context_label),
        ("feature_id", context.feature_id),
        ("consumable_id", context.consumable_id),
    ] {
        if let Some(value) = value {
            input[key] = json!(value);
        }
    }
    if command.target_mode == CommandTargetMode::Consumable
        && let (Some(feature_id), Some(consumable_id)) = (context.feature_id, context.consumable_id)
        && let Some(consumable) =
            content.room_consumable(context.room_id, feature_id, consumable_id)
    {
        input["hunger_recovery"] = json!(consumable.consumable.hunger_recovery);
        input["stamina_recovery"] = json!(consumable.consumable.stamina_recovery);
    }
    apply_world_hook_effects(state, content, &command.hook_id, input)
        .unwrap_or_else(|error| eprintln!("[cinder] hook warning (command): {error}"));
}

pub(super) fn record_actor_command_memory(
    state: &mut WorldState,
    content: &ContentPack,
    command: &ActionDefinition,
    context: &ActorCommandContext<'_>,
    text: &str,
) {
    if command.has_effect(CommandEffect::RememberWithTargetActor) {
        let Some(target_actor_id) = context.target_actor_id else {
            return;
        };
        state.push_conversation_line(
            context.actor_id,
            target_actor_id,
            ConversationMemoryLine {
                turn_number: state.turn_number,
                event_sequence: 0,
                speaker_id: context.actor_id.to_string(),
                speaker_name: context.actor_name.to_string(),
                kind: ConversationMemoryKind::Action,
                target_label: context.target_actor_name.map(str::to_string),
                text: text.to_string(),
            },
        );
        if state
            .pending_reply(context.actor_id, target_actor_id)
            .is_some_and(|pending| {
                pending.speaker_id == target_actor_id && pending.listener_id == context.actor_id
            })
        {
            state.clear_pending_reply(context.actor_id, target_actor_id);
        }
    }
    if command.has_effect(CommandEffect::RememberInRoom) {
        record_room_action_memory(
            state,
            content,
            context.actor_id,
            context.actor_name,
            context.room_id,
            text,
        );
    }
}

pub(super) fn resolve_actor_command_labels(
    content: &ContentPack,
    context: &ActorCommandContext<'_>,
) -> Option<(String, String)> {
    if let (Some(feature_id), Some(consumable_id)) = (context.feature_id, context.consumable_id)
        && let Some(consumable) =
            content.room_consumable(context.room_id, feature_id, consumable_id)
    {
        return Some((
            consumable.consumable.label.clone(),
            consumable.feature.label.clone(),
        ));
    }
    Some((
        String::new(),
        context
            .feature_id
            .and_then(|feature_id| content.feature(context.room_id, feature_id))
            .map(|feature| feature.label.clone())
            .unwrap_or_default(),
    ))
}

pub(super) fn render_actor_command_text(
    content: &ContentPack,
    command: &ActionDefinition,
    context: &ActorCommandContext<'_>,
    item_label: &str,
    feature_label: &str,
) -> Option<String> {
    match command.input_mode {
        CommandInputMode::FreeformText => context.freeform_text.and_then(|text| {
            crate::engine::events::render_actor_action_text(context.actor_name, text).ok()
        }),
        _ if command.event_text.is_empty() => None,
        _ => Some(
            content.render_template(
                &command.event_text,
                &[
                    ("actor_name", context.actor_name),
                    ("target_actor_name", context.target_actor_name.unwrap_or("")),
                    ("context_label", context.context_label.unwrap_or("")),
                    ("feature_label", feature_label),
                    ("item_label", item_label),
                    (
                        "target_room_title",
                        context
                            .target_room_id
                            .and_then(|room_id| content.room(room_id))
                            .map(|room| room.title.as_str())
                            .unwrap_or(""),
                    ),
                ],
            ),
        ),
    }
}
