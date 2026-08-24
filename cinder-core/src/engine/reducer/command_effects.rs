use crate::content::types::{
    ActionDefinition, ActionItemStorageTarget, CommandEffect, CommandInputMode, CommandTargetMode,
    ContentPack, ConversionTrigger, ItemStorageTarget,
};
use crate::engine::events::{ObservationMode, WorldEvent};
use crate::engine::hook_ids;
use crate::engine::hooks::{apply_narrating_world_hook_effects, apply_world_hook_effects};
use crate::engine::state::{
    ActorRelationship, ActorStance, ConversationMemoryKind, ConversationMemoryLine, WorldState,
};
use crate::engine::turn_policies::apply_command_bundle_progress_effects;
use serde_json::json;

use super::beat_advance::advance_objective_for_signal;
use super::observation::render_room_observation;
use super::tick::{
    advance_house_progress_objectives, pending_reply_broken_by_move, record_room_action_memory,
};

fn to_item_storage(storage: ActionItemStorageTarget) -> ItemStorageTarget {
    match storage {
        ActionItemStorageTarget::PlayerInventory => ItemStorageTarget::PlayerInventory,
        ActionItemStorageTarget::CurrentRoom => ItemStorageTarget::CurrentRoom,
    }
}

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

pub(super) struct ActorMoveTransitionContext<'a> {
    pub(super) actor_id: &'a str,
    pub(super) actor_name: Option<&'a str>,
    pub(super) from_room_id: &'a str,
    pub(super) to_room_id: &'a str,
    pub(super) command_text: Option<&'a str>,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_actor_command_used(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    actor_name: &str,
    room_id: &str,
    command_id: &str,
    target_room_id: Option<&str>,
    target_actor_id: Option<&str>,
    target_actor_name: Option<&str>,
    context_label: Option<&str>,
    feature_id: Option<&str>,
    consumable_id: Option<&str>,
    freeform_text: Option<&str>,
    outbox: &mut Vec<WorldEvent>,
) -> Option<Vec<String>> {
    let mut lines = Vec::new();
    let previous_current_room_id = state.current_room_id.clone();
    let command_context = ActorCommandContext {
        actor_id,
        actor_name,
        room_id,
        target_room_id,
        target_actor_id,
        target_actor_name,
        context_label,
        feature_id,
        consumable_id,
        freeform_text,
    };
    let command = content.command(command_id)?;
    let (item_label, feature_label) = resolve_actor_command_labels(content, &command_context)?;
    if !apply_actor_command_realization_effects(state, content, command, &command_context) {
        return None;
    }
    let command_text = render_actor_command_text(
        content,
        command,
        &command_context,
        &item_label,
        &feature_label,
    )?;
    record_actor_command_memory(state, content, command, &command_context, &command_text);
    apply_actor_command_effects(state, content, command, &command_context);
    // Narrate the actor's own action first; its world consequences (e.g. an
    // encirclement conversion triggered by placing a flag) follow as responses.
    if !command.has_effect(CommandEffect::MoveActor) && state.current_room_id == room_id {
        lines.push(command_text.clone());
    }
    apply_new_command_effects(
        state,
        content,
        command,
        &command_context,
        &mut lines,
        outbox,
    );
    apply_command_bundle_progress_effects(state, command);
    if let Some(item_id) = resolved_created_item_id(state, command, &command_context) {
        let storage = command
            .item_creation
            .as_ref()
            .map(|ic| to_item_storage(ic.storage.clone()))
            .unwrap_or_default();
        state.add_item_to_storage(item_id.as_str(), storage, room_id);
    }
    if command.has_effect(CommandEffect::MoveActor) {
        if let Some(target_room_id) = target_room_id {
            apply_actor_move_transition(
                state,
                content,
                ActorMoveTransitionContext {
                    actor_id,
                    actor_name: Some(actor_name),
                    from_room_id: room_id,
                    to_room_id: target_room_id,
                    command_text: Some(command_text.as_str()),
                },
                &mut lines,
            );
        } else if previous_current_room_id == room_id
            || state.followed_actor_id.as_deref() == Some(actor_id)
        {
            lines.push(command_text);
        }
    }
    lines.extend(advance_objective_for_signal(state, content, "command_used"));
    Some(lines)
}

fn resolved_created_item_id(
    state: &WorldState,
    command: &ActionDefinition,
    command_context: &ActorCommandContext<'_>,
) -> Option<String> {
    let item_id = &command.item_creation.as_ref()?.creates_item;
    if command
        .item_creation
        .as_ref()
        .is_some_and(|ic| ic.creates_item_resolve_from_target)
    {
        return Some(
            command_context
                .target_actor_id
                .map(|target_actor_id| format!("clip-{target_actor_id}"))
                .unwrap_or_else(|| item_id.clone()),
        );
    }
    Some(
        command
            .item_creation
            .as_ref()
            .map(|ic| &ic.creates_item_story_var)
            .and_then(|var_key| state.story_vars.get(var_key))
            .unwrap_or(item_id)
            .to_string(),
    )
}

pub(super) fn apply_actor_command_realization_effects(
    state: &mut WorldState,
    content: &ContentPack,
    command: &ActionDefinition,
    command_context: &ActorCommandContext<'_>,
) -> bool {
    for effect in &command.effects {
        match effect {
            CommandEffect::ObserveFeature => {
                let Some(feature_id) = command_context.feature_id else {
                    return false;
                };
                let Some(feature) = content.room(command_context.room_id).and_then(|room| {
                    room.features
                        .iter()
                        .find(|feature| feature.id == feature_id)
                }) else {
                    return false;
                };
                state.mark_actor_feature_seen(
                    command_context.actor_id,
                    command_context.room_id,
                    feature_id,
                );
                state.push_actor_observation_note(
                    command_context.actor_id,
                    feature.inspect_text.clone(),
                );
            }
            CommandEffect::ObserveActor => {
                let Some(target_actor_id) = command_context.target_actor_id else {
                    return false;
                };
                let Some(target_actor) = content.actor(target_actor_id) else {
                    return false;
                };
                state.mark_actor_studied_actor(command_context.actor_id, target_actor_id);
                state.push_actor_observation_note(
                    command_context.actor_id,
                    target_actor.inspect_text.clone(),
                );
            }
            CommandEffect::MoveActor => {}
            CommandEffect::ConsumeTargetedConsumable => {
                let Some(feature_id) = command_context.feature_id else {
                    return false;
                };
                let Some(consumable_id) = command_context.consumable_id else {
                    return false;
                };
                if !state.consume_feature_consumable(
                    command_context.room_id,
                    feature_id,
                    consumable_id,
                ) && !state.remove_item_from_storage(
                    consumable_id,
                    crate::content::types::ItemStorageTarget::CurrentRoom,
                    command_context.room_id,
                ) {
                    return false;
                }
            }
            CommandEffect::ObserveRoom
            | CommandEffect::RememberInRoom
            | CommandEffect::RememberWithTargetActor
            | CommandEffect::FollowActor => {}
            CommandEffect::DropItem => {
                if command.drop_item.is_empty() {
                    return false;
                }
                if !state.has_item(&command.drop_item) {
                    return false;
                }
            }
            CommandEffect::PickUpItem => {
                if command.drop_item.is_empty() {
                    return false;
                }
                if !state.has_item_in_storage(
                    &command.drop_item,
                    ItemStorageTarget::CurrentRoom,
                    command_context.room_id,
                ) {
                    return false;
                }
            }
            CommandEffect::AttackTarget => {
                let Some(target_actor_id) = command_context.target_actor_id else {
                    return false;
                };
                if !content
                    .actor(target_actor_id)
                    .is_some_and(|actor| actor.attackable)
                {
                    return false;
                }
                if state.actor_stat(target_actor_id, &content.settings.combat.health_stat_id) <= 0 {
                    return false;
                }
                if state.stance(target_actor_id) == ActorStance::Allied {
                    return false;
                }
            }
        }
    }
    true
}

pub(super) fn apply_actor_command_effects(
    state: &mut WorldState,
    content: &ContentPack,
    command: &ActionDefinition,
    command_context: &ActorCommandContext<'_>,
) {
    if command.hook_id.is_empty() {
        return;
    }
    let mut input = json!({
        "actor_id": command_context.actor_id,
        "room_id": command_context.room_id,
    });
    if let Some(target_actor_id) = command_context.target_actor_id {
        input["target_actor_id"] = json!(target_actor_id);
    }
    if let Some(context_label) = command_context.context_label {
        input["context_label"] = json!(context_label);
    }
    if let Some(feature_id) = command_context.feature_id {
        input["feature_id"] = json!(feature_id);
    }
    if let Some(consumable_id) = command_context.consumable_id {
        input["consumable_id"] = json!(consumable_id);
    }
    if command.target_mode == CommandTargetMode::Consumable
        && let (Some(feature_id), Some(consumable_id)) =
            (command_context.feature_id, command_context.consumable_id)
        && let Some(consumable) =
            content.room_consumable(command_context.room_id, feature_id, consumable_id)
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
    command_context: &ActorCommandContext<'_>,
    text: &str,
) {
    if command.has_effect(CommandEffect::RememberWithTargetActor) {
        let Some(target_actor_id) = command_context.target_actor_id else {
            return;
        };
        state.push_conversation_line(
            command_context.actor_id,
            target_actor_id,
            ConversationMemoryLine {
                turn_number: state.turn_number,
                event_sequence: 0,
                speaker_id: command_context.actor_id.to_string(),
                speaker_name: command_context.actor_name.to_string(),
                kind: ConversationMemoryKind::Action,
                target_label: command_context.target_actor_name.map(str::to_string),
                text: text.to_string(),
            },
        );
        if state
            .pending_reply(command_context.actor_id, target_actor_id)
            .is_some_and(|pending| {
                pending.speaker_id == target_actor_id
                    && pending.listener_id == command_context.actor_id
            })
        {
            state.clear_pending_reply(command_context.actor_id, target_actor_id);
        }
    }
    if command.has_effect(CommandEffect::RememberInRoom) {
        record_room_action_memory(
            state,
            content,
            command_context.actor_id,
            command_context.actor_name,
            command_context.room_id,
            text,
        );
    }
}

pub(super) fn resolve_actor_command_labels(
    content: &ContentPack,
    command_context: &ActorCommandContext<'_>,
) -> Option<(String, String)> {
    if let (Some(feature_id), Some(consumable_id)) =
        (command_context.feature_id, command_context.consumable_id)
        && let Some(consumable) =
            content.room_consumable(command_context.room_id, feature_id, consumable_id)
    {
        return Some((
            consumable.consumable.label.clone(),
            consumable.feature.label.clone(),
        ));
    }

    Some((
        String::new(),
        command_context
            .feature_id
            .and_then(|feature_id| {
                content
                    .room(command_context.room_id)?
                    .features
                    .iter()
                    .find(|feature| feature.id == feature_id)
            })
            .map(|feature| feature.label.clone())
            .unwrap_or_default(),
    ))
}

pub(super) fn render_actor_command_text(
    content: &ContentPack,
    command: &ActionDefinition,
    command_context: &ActorCommandContext<'_>,
    item_label: &str,
    feature_label: &str,
) -> Option<String> {
    match command.input_mode {
        CommandInputMode::FreeformText => command_context.freeform_text.and_then(|text| {
            crate::engine::events::render_actor_action_text(command_context.actor_name, text).ok()
        }),
        _ if command.event_text.is_empty() => None,
        _ => Some(
            content.render_template(
                &command.event_text,
                &[
                    ("actor_name", command_context.actor_name),
                    (
                        "target_actor_name",
                        command_context.target_actor_name.unwrap_or(""),
                    ),
                    ("context_label", command_context.context_label.unwrap_or("")),
                    ("feature_label", feature_label),
                    ("item_label", item_label),
                    (
                        "target_room_title",
                        command_context
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

pub(super) fn apply_actor_move_transition(
    state: &mut WorldState,
    content: &ContentPack,
    movement: ActorMoveTransitionContext<'_>,
    lines: &mut Vec<String>,
) {
    apply_world_hook_effects(
        state,
        content,
        hook_ids::ACTOR_MOVED,
        json!({
            "actor_id": movement.actor_id,
            "from_room_id": movement.from_room_id,
            "to_room_id": movement.to_room_id,
        }),
    )
    .unwrap_or_else(|error| eprintln!("[cinder] hook warning (actor.moved): {error}"));
    if let Some(pending) =
        pending_reply_broken_by_move(state, movement.actor_id, movement.from_room_id)
    {
        apply_world_hook_effects(
            state,
            content,
            hook_ids::BROKEN_REPLY,
            json!({
                "event_kind": "broken_reply",
                "participant_a_id": pending.speaker_id,
                "participant_b_id": pending.listener_id,
            }),
        )
        .unwrap_or_else(|error| eprintln!("[cinder] hook warning (broken_reply): {error}"));
        state.clear_pending_reply(&pending.speaker_id, &pending.listener_id);
    }
    let is_followed_actor = state.followed_actor_id.as_deref() == Some(movement.actor_id);
    let actor_name = movement
        .actor_name
        .or_else(|| {
            content
                .actor(movement.actor_id)
                .map(|actor| actor.name.as_str())
        })
        .unwrap_or(movement.actor_id);
    if let Some(command_text) = movement.command_text {
        if state.current_room_id == movement.from_room_id || is_followed_actor {
            lines.push(command_text.to_string());
        } else if !is_followed_actor
            && state.current_room_id == movement.to_room_id
            && let Some(origin) = content.room(movement.from_room_id)
        {
            lines.push(content.render_template(
                &content.presentation.presentation_text.actor_arrived,
                &[
                    ("actor_name", actor_name),
                    ("room_title", origin.title.as_str()),
                ],
            ));
        }
    } else if !is_followed_actor && state.current_room_id == movement.from_room_id {
        if let Some(destination) = content.room(movement.to_room_id) {
            lines.push(content.render_template(
                &content.presentation.presentation_text.actor_departed,
                &[
                    ("actor_name", actor_name),
                    ("room_title", destination.title.as_str()),
                ],
            ));
        }
    } else if !is_followed_actor
        && state.current_room_id == movement.to_room_id
        && let Some(origin) = content.room(movement.from_room_id)
    {
        lines.push(content.render_template(
            &content.presentation.presentation_text.actor_arrived,
            &[
                ("actor_name", actor_name),
                ("room_title", origin.title.as_str()),
            ],
        ));
    }
    state.mark_actor_room_visited(movement.actor_id, movement.to_room_id);
    state.actor_room_overrides.insert(
        movement.actor_id.to_string(),
        movement.to_room_id.to_string(),
    );
    if is_followed_actor {
        state.current_room_id = movement.to_room_id.to_string();
        if let Some(observation) = render_room_observation(
            content,
            state,
            movement.to_room_id,
            ObservationMode::Summary,
        ) {
            lines.push(observation);
        }
    }
    lines.extend(advance_objective_for_signal(
        state,
        content,
        &format!(
            "actor_entered:{}:{}",
            movement.actor_id, movement.to_room_id
        ),
    ));
    lines.extend(advance_house_progress_objectives(state, content));
}

pub(super) fn apply_new_command_effects(
    state: &mut WorldState,
    content: &ContentPack,
    command: &ActionDefinition,
    command_context: &ActorCommandContext<'_>,
    lines: &mut Vec<String>,
    _outbox: &mut Vec<WorldEvent>,
) {
    if command.has_effect(CommandEffect::DropItem) {
        let room_id = command_context.room_id.to_string();
        if state.remove_item(&command.drop_item) {
            state.add_item_to_storage(&command.drop_item, ItemStorageTarget::CurrentRoom, &room_id);
            run_encirclement_rules(state, content, &command.drop_item, lines);
        }
    }
    if command.has_effect(CommandEffect::PickUpItem) {
        let room_id = command_context.room_id.to_string();
        if state.remove_item_from_storage(
            &command.drop_item,
            ItemStorageTarget::CurrentRoom,
            &room_id,
        ) {
            state.add_item(&command.drop_item);
        }
    }
    if command.has_effect(CommandEffect::AttackTarget) {
        let Some(target_actor_id) = command_context.target_actor_id else {
            return;
        };
        let combat = &content.settings.combat;
        // Mechanism backstop: allies are never valid targets, even if a pack
        // omits the planning-side message.
        if state.stance(target_actor_id) == ActorStance::Allied {
            return;
        }
        let player_attack = state.actor_stat(&combat.player_actor_id, &combat.attack_stat_id);
        let target_defense = state.actor_stat(target_actor_id, &combat.defense_stat_id);
        let base_damage = (player_attack - target_defense).max(combat.minimum_damage);
        let allied_participants = room_allies(state, content, command_context.room_id);
        let ally_damage = allied_participants
            .iter()
            .map(|actor_id| state.actor_stat(actor_id, &combat.attack_stat_id).max(0))
            .sum::<i32>();
        let total_damage = base_damage + ally_damage;
        let remaining = adjust_actor_stat(
            state,
            target_actor_id,
            &combat.health_stat_id,
            -total_damage,
        );
        let target_name = actor_display_name(content, target_actor_id);
        if let Some(line) = content.render_message(
            "combat.attack_hit",
            &[
                ("actor", target_name.as_str()),
                ("damage", total_damage.to_string().as_str()),
                ("remaining", remaining.to_string().as_str()),
            ],
        ) {
            lines.push(line);
        }
        for ally_id in &allied_participants {
            let ally_damage = state.actor_stat(ally_id, &combat.attack_stat_id).max(0);
            let ally_name = actor_display_name(content, ally_id);
            if let Some(line) = content.render_message(
                "combat.ally_joins_attack",
                &[
                    ("actor", ally_name.as_str()),
                    ("damage", ally_damage.to_string().as_str()),
                ],
            ) {
                lines.push(line);
            }
        }
        if remaining <= 0 {
            if let Some(line) =
                content.render_message("combat.actor_defeated", &[("actor", target_name.as_str())])
            {
                lines.push(line);
            }
            state.set_relationship(target_actor_id, ActorRelationship::default());
            return;
        }
        let mut relationship = state.relationship(target_actor_id);
        if relationship.stance != ActorStance::Allied && relationship.stance != ActorStance::Hostile
        {
            relationship.stance = ActorStance::Hostile;
            state.set_relationship(target_actor_id, relationship);
            // Grace window: the freshly woken mob does not strike until its
            // attack cooldown elapses.
            let interval = content
                .actor(target_actor_id)
                .map(|actor| actor.attack_interval_minutes(combat.default_attack_interval_minutes))
                .unwrap_or(combat.default_attack_interval_minutes);
            state.next_hostile_strike_at.insert(
                target_actor_id.to_string(),
                state.current_time_minutes + interval,
            );
            if let Some(line) = content.render_message(
                "combat.actor_wakes_hostile",
                &[("actor", target_name.as_str())],
            ) {
                lines.push(line);
            }
        }
    }
}

pub(super) fn actor_display_name(content: &ContentPack, actor_id: &str) -> String {
    content
        .actor(actor_id)
        .map(|actor| actor.name.clone())
        .unwrap_or_else(|| actor_id.to_string())
}

fn adjust_actor_stat(state: &mut WorldState, actor_id: &str, stat: &str, delta: i32) -> i32 {
    state
        .adjust_actor_stat(actor_id, stat, delta)
        .unwrap_or_else(|error| eprintln!("[cinder] combat stat error: {error}"));
    state.actor_stat(actor_id, stat)
}

/// Ends the game when the player actor's health stat has reached zero. The
/// defeat narration is pack-authored via `settings.combat.player_defeat_text`.
pub(super) fn defeat_player_if_dead(
    state: &mut WorldState,
    content: &ContentPack,
    lines: &mut Vec<String>,
) {
    if state.phase != crate::engine::state::GamePhase::Active {
        return;
    }
    let combat = &content.settings.combat;
    if state.actor_stat(&combat.player_actor_id, &combat.health_stat_id) > 0 {
        return;
    }
    lines.push(super::observation::render_story_text(
        &combat.player_defeat_text,
        state,
    ));
    state.phase = crate::engine::state::GamePhase::GameEnded;
}

/// Allies whose strength adds to a player attack: allied, alive, and in the
/// same room as the attack.
fn room_allies(state: &WorldState, content: &ContentPack, room_id: &str) -> Vec<String> {
    state
        .relationships
        .iter()
        .filter(|(_, relationship)| relationship.stance == ActorStance::Allied)
        .map(|(actor_id, _)| actor_id.clone())
        .filter(|actor_id| {
            !state.actor_is_defeated(actor_id, &content.settings.combat.health_stat_id)
                && state.actor_room_id(
                    actor_id,
                    content
                        .actor(actor_id)
                        .map(|actor| actor.room_id.as_str())
                        .unwrap_or_default(),
                ) == room_id
        })
        .collect()
}

/// Generic encirclement scan: after an item is dropped into a room, find any
/// living, neutral actor marked `conversion_trigger: encirclement` whose room
/// is now fully surrounded by neighbors containing that item, and fire the
/// `actor.encircled` hook for each. The hook's *effect* is content-authored
/// (e.g. convert the actor to an ally follower) — the engine only decides
/// *that* the encirclement completed.
fn run_encirclement_rules(
    state: &mut WorldState,
    content: &ContentPack,
    item_id: &str,
    lines: &mut Vec<String>,
) {
    for actor in &content.actors {
        if actor.conversion_trigger != Some(ConversionTrigger::Encirclement) {
            continue;
        }
        let relationship = state.relationship(&actor.id);
        if relationship.stance != ActorStance::Neutral || relationship.follows_player {
            continue;
        }
        if state.actor_is_defeated(&actor.id, &content.settings.combat.health_stat_id) {
            continue;
        }
        let room_id = state.actor_room_id(&actor.id, &actor.room_id).to_string();
        let neighbors = content.adjacent_room_ids(&room_id);
        if neighbors.is_empty() {
            continue;
        }
        if !neighbors
            .iter()
            .all(|n| state.has_item_in_storage(item_id, ItemStorageTarget::CurrentRoom, n))
        {
            continue;
        }
        let actor_name = actor_display_name(content, &actor.id);
        apply_narrating_world_hook_effects(
            state,
            content,
            hook_ids::ACTOR_ENCIRCLED,
            json!({
                "actor_id": actor.id,
                "actor_name": actor_name,
                "room_id": room_id,
                "item_id": item_id,
            }),
            lines,
        )
        .unwrap_or_else(|error| eprintln!("[cinder] hook warning (actor.encircled): {error}"));
    }
}
