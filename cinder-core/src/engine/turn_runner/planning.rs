use super::types::{PendingDialogue, PlannedTurn};
use crate::content::types::{
    ActionDefinition, ActionItemConsumerTarget, CommandEffect, CommandOutcomeMode, ContentPack,
    ItemStorageTarget, PlayerCommandTargetMode,
};
use crate::engine::commands::{resolve_actor_reference_input, unknown_target_token};
use crate::engine::dialogue_grounding::viewer_participant_id;
use crate::engine::events::{ObservationMode, WorldEvent};
use crate::engine::state::{ActorStance, WorldState, display_actor_name};
use crate::engine::turn_policies::{command_availability_issue, command_unavailable_message};
use std::collections::BTreeMap;

fn to_item_storage(storage: crate::content::types::ActionItemStorageTarget) -> ItemStorageTarget {
    match storage {
        crate::content::types::ActionItemStorageTarget::PlayerInventory => {
            ItemStorageTarget::PlayerInventory
        }
        crate::content::types::ActionItemStorageTarget::CurrentRoom => {
            ItemStorageTarget::CurrentRoom
        }
    }
}

pub(super) struct PlanningContext<'a> {
    pub(super) raw_input: &'a str,
    pub(super) current_room_id: &'a str,
    pub(super) planner_state: &'a WorldState,
    pub(super) channel_surfing_only: bool,
    pub(super) turn_number: u32,
}

fn pending_dialogue_for(
    content: &ContentPack,
    context: &PlanningContext<'_>,
    actor_id: String,
    other_person_message: Option<String>,
) -> PendingDialogue {
    PendingDialogue {
        actor_id,
        current_room_id: context.current_room_id.to_string(),
        raw_input: context.raw_input.to_string(),
        other_person_id: viewer_participant_id(content),
        other_person_name: content.opening.title.clone(),
        other_person_message,
        turn_number: context.turn_number,
    }
}

fn content_event_for_command(
    action: &ActionDefinition,
    payload: BTreeMap<String, String>,
) -> WorldEvent {
    let content_event = action
        .content_event
        .as_ref()
        .unwrap_or_else(|| panic!("action '{}' should define a content_event", action.id));
    WorldEvent::ContentEvent {
        event_id: content_event.id.clone(),
        payload,
    }
}

fn first_actor_in_room<'a>(
    content: &'a ContentPack,
    context: &PlanningContext<'_>,
) -> Option<&'a crate::content::types::ActorDefinition> {
    content.actors.iter().find(|actor| {
        !context
            .planner_state
            .actor_is_defeated(&actor.id, &content.settings.combat.health_stat_id)
            && context
                .planner_state
                .actor_room_id(&actor.id, &actor.room_id)
                == context.current_room_id
    })
}

fn resolved_created_item_id(
    content: &ContentPack,
    action: &ActionDefinition,
    input: Option<&str>,
    context: &PlanningContext<'_>,
) -> Option<String> {
    let item_creation = action.item_creation.as_ref()?;
    let item_id = &item_creation.creates_item;
    if item_creation.creates_item_resolve_from_target {
        let input_val = input.unwrap_or_default().trim();
        return Some(
            resolve_actor_reference_input(
                content,
                context.planner_state,
                context.current_room_id,
                input_val,
            )
            .map(|resolved| format!("clip-{}", resolved.actor_id))
            .unwrap_or_else(|| item_id.clone()),
        );
    }
    Some(
        (!item_creation.creates_item_story_var.is_empty())
            .then_some(item_creation.creates_item_story_var.as_str())
            .and_then(|var_key| context.planner_state.story_vars.get(var_key))
            .map(|value| value.to_string())
            .unwrap_or_else(|| item_id.clone()),
    )
}

pub(super) fn plan_content_command(
    content: &ContentPack,
    action: &ActionDefinition,
    input: Option<&str>,
    context: &PlanningContext<'_>,
    planned: &mut PlannedTurn,
) -> bool {
    if let Some(issue) = command_availability_issue(content, context.planner_state, action) {
        planned.events.push(WorldEvent::ActionRejected {
            message: command_unavailable_message(content, action, &issue),
        });
        return false;
    }

    // Check room restrictions
    if !action.available.allowed_rooms.is_empty()
        && !action
            .available
            .allowed_rooms
            .contains(&context.current_room_id.to_string())
    {
        let needed = action
            .available
            .allowed_rooms
            .first()
            .and_then(|id| content.room(id))
            .map(|r| r.title.as_str())
            .unwrap_or("another room");
        let command_lower = action.command.to_lowercase();
        planned.events.push(WorldEvent::ActionRejected {
            message: content
                .render_message(
                    "error.cannot_command_here",
                    &[("command", command_lower.as_str()), ("room", needed)],
                )
                .unwrap_or_default(),
        });
        return false;
    }

    // Check item requirement (consumes_item, consumes_any, or requires_any)
    if let Some(item_id) = &action.available.consumes_item {
        if !context.planner_state.has_item_in_storage(
            item_id,
            to_item_storage(action.available.consumes_item_storage.clone()),
            context.current_room_id,
        ) {
            let label = content
                .item(item_id)
                .map(|i| i.label.as_str())
                .unwrap_or(item_id);
            planned.events.push(WorldEvent::ActionRejected {
                message: content
                    .render_message("error.missing_item", &[("label", label)])
                    .unwrap_or_default(),
            });
            return false;
        }
        if action.item_consumer == ActionItemConsumerTarget::FirstActorInRoom
            && first_actor_in_room(content, context).is_none()
        {
            planned.events.push(WorldEvent::ActionRejected {
                message: "There is no one here to serve.".to_string(),
            });
            return false;
        }
    }
    if !action.available.requires_any.is_empty() || !action.available.consumes_any.is_empty() {
        let all_required: Vec<_> = action
            .available
            .requires_any
            .iter()
            .chain(action.available.consumes_any.iter())
            .collect();
        let has_any = all_required.iter().any(|id| {
            let storage = if action
                .available
                .consumes_any
                .iter()
                .any(|candidate| candidate == *id)
            {
                to_item_storage(action.available.consumes_any_storage.clone())
            } else {
                to_item_storage(action.available.requires_any_storage.clone())
            };
            context
                .planner_state
                .has_item_in_storage(id, storage, context.current_room_id)
        });
        if !has_any {
            planned.events.push(WorldEvent::ActionRejected {
                message: content
                    .render_message("error.nothing_to_consume", &[])
                    .unwrap_or_default(),
            });
            return false;
        }
        if !action.available.consumes_any.is_empty()
            && action.item_consumer == ActionItemConsumerTarget::FirstActorInRoom
            && first_actor_in_room(content, context).is_none()
        {
            planned.events.push(WorldEvent::ActionRejected {
                message: "There is no one here to serve.".to_string(),
            });
            return false;
        }
    }

    let metadata = action
        .player_command
        .as_ref()
        .unwrap_or_else(|| panic!("action '{}' should define player_command", action.id));
    let mut payload = BTreeMap::new();
    if let Some(input_metadata) = &metadata.input {
        let value = input.unwrap_or_default().trim();
        if input_metadata.required && value.is_empty() {
            planned.events.push(WorldEvent::ActionRejected {
                message: content.render_template(
                    &content.presentation.error_text.unknown_input,
                    &[
                        ("raw_input", context.raw_input),
                        ("available_commands", metadata.usage.as_str()),
                    ],
                ),
            });
            return false;
        }
        if !value.is_empty() {
            payload.insert(input_metadata.payload_key.clone(), value.to_string());
        }
    }

    // Content event (narrative) first, then item events
    planned
        .events
        .push(content_event_for_command(action, payload));
    if !action.sets_bundle_progress.is_empty() || !action.clears_bundle_progress.is_empty() {
        planned
            .events
            .push(WorldEvent::CommandBundleProgressApplied {
                command_id: action.id.clone(),
            });
    }

    if let Some(item_id) = resolved_created_item_id(content, action, input, context) {
        planned.events.push(WorldEvent::ItemAcquired {
            item_id,
            storage: action
                .item_creation
                .as_ref()
                .map(|ic| to_item_storage(ic.storage.clone()))
                .unwrap_or_default(),
        });
    }
    if let Some(item_id) = &action.available.consumes_item {
        let (consumer_id, consumer_name) = match action.item_consumer {
            ActionItemConsumerTarget::None => (None, None),
            ActionItemConsumerTarget::Player => (
                Some(content.settings.combat.player_actor_id.clone()),
                Some("You".to_string()),
            ),
            ActionItemConsumerTarget::FirstActorInRoom => {
                let recipient =
                    first_actor_in_room(content, context).expect("actor should be in room");
                (Some(recipient.id.clone()), Some(recipient.name.clone()))
            }
        };
        planned.events.push(WorldEvent::ItemConsumed {
            item_id: item_id.clone(),
            storage: to_item_storage(action.available.consumes_item_storage.clone()),
            consumer_id,
            consumer_name,
        });
    }
    if !action.available.consumes_any.is_empty()
        && let Some(item_id) = action.available.consumes_any.iter().find(|id| {
            context.planner_state.has_item_in_storage(
                id,
                to_item_storage(action.available.consumes_any_storage.clone()),
                context.current_room_id,
            )
        })
    {
        let (consumer_id, consumer_name) = match action.item_consumer {
            ActionItemConsumerTarget::None => (None, None),
            ActionItemConsumerTarget::Player => (
                Some(content.settings.combat.player_actor_id.clone()),
                Some("You".to_string()),
            ),
            ActionItemConsumerTarget::FirstActorInRoom => {
                let recipient =
                    first_actor_in_room(content, context).expect("actor should be in room");
                (Some(recipient.id.clone()), Some(recipient.name.clone()))
            }
        };
        planned.events.push(WorldEvent::ItemConsumed {
            item_id: item_id.clone(),
            storage: to_item_storage(action.available.consumes_any_storage.clone()),
            consumer_id,
            consumer_name,
        });
    }

    metadata.advances_time
}

pub(super) fn plan_observe_room(context: &PlanningContext<'_>, planned: &mut PlannedTurn) -> bool {
    planned.events.push(WorldEvent::CurrentRoomObserved {
        room_id: context.current_room_id.to_string(),
        mode: ObservationMode::Detailed,
    });
    false
}

pub(super) fn plan_observe_target(
    content: &ContentPack,
    target: &str,
    context: &PlanningContext<'_>,
    planned: &mut PlannedTurn,
) -> bool {
    if let Some(actor) = content.resolve_actor(target).or_else(|| {
        content.actors.iter().find(|actor| {
            display_actor_name(context.planner_state, actor).eq_ignore_ascii_case(target)
        })
    }) {
        let actor_name = display_actor_name(context.planner_state, actor);
        if context
            .planner_state
            .actor_room_id(&actor.id, &actor.room_id)
            == context.current_room_id
        {
            planned.events.push(WorldEvent::ActorObserved {
                actor_id: actor.id.clone(),
            });
        } else {
            planned.events.push(WorldEvent::ActionRejected {
                message: content.render_template(
                    &content.presentation.error_text.actor_not_here,
                    &[("actor_name", actor_name.as_str())],
                ),
            });
        }
    } else if let Some(feature) = content.resolve_feature_in_room(context.current_room_id, target) {
        planned.events.push(WorldEvent::FeatureObserved {
            room_id: context.current_room_id.to_string(),
            feature_id: feature.id.clone(),
        });
    } else if let Some(item) =
        content.resolve_item_in_scope(context.planner_state, context.current_room_id, target)
    {
        planned.events.push(WorldEvent::ItemObserved {
            item_id: item.id.clone(),
        });
    } else {
        planned.events.push(WorldEvent::ActionRejected {
            message: content.render_template(
                &content.presentation.error_text.feature_unknown,
                &[("target", target)],
            ),
        });
    }

    false
}

pub(super) fn plan_move_to_room_target(
    content: &ContentPack,
    target: &str,
    advances_time: bool,
    context: &PlanningContext<'_>,
    planned: &mut PlannedTurn,
) -> bool {
    if let Some(exit) = content.resolve_exit(context.current_room_id, target) {
        planned.events.push(WorldEvent::PlayerMoved {
            from_room_id: context.current_room_id.to_string(),
            to_room_id: exit.room_id.clone(),
        });
        planned.events.push(WorldEvent::CurrentRoomObserved {
            room_id: exit.room_id.clone(),
            mode: ObservationMode::Summary,
        });
        advances_time
    } else {
        planned.events.push(WorldEvent::ActionRejected {
            message: content.render_template(
                &content.presentation.error_text.cannot_go,
                &[("target", target)],
            ),
        });
        false
    }
}

/// Attacking an ally is never allowed: emit a locale-authored rejection when
/// the pack defines one, otherwise a silent rejection (the mechanism still
/// blocks the attack and its time advance).
fn ally_attack_rejection(
    content: &ContentPack,
    state: &WorldState,
    action: &ActionDefinition,
    actor_id: &str,
    actor_name: &str,
) -> Option<WorldEvent> {
    if !action.has_effect(CommandEffect::AttackTarget) {
        return None;
    }
    if state.stance(actor_id) != ActorStance::Allied {
        return None;
    }
    Some(WorldEvent::ActionRejected {
        message: content
            .render_message("combat.cannot_attack_ally", &[("actor", actor_name)])
            .unwrap_or_default(),
    })
}

pub(super) fn plan_targeted_state_command(
    content: &ContentPack,
    action: &ActionDefinition,
    input: Option<&str>,
    context: &PlanningContext<'_>,
    planned: &mut PlannedTurn,
) -> bool {
    let metadata = action
        .player_command
        .as_ref()
        .unwrap_or_else(|| panic!("action '{}' should define player_command", action.id));
    match metadata.target_mode {
        PlayerCommandTargetMode::RoomReference => plan_move_to_room_target(
            content,
            input.unwrap_or_default().trim(),
            metadata.advances_time,
            context,
            planned,
        ),
        PlayerCommandTargetMode::ActorOrFeatureReference => {
            plan_observe_target(content, input.unwrap_or_default().trim(), context, planned)
        }
        PlayerCommandTargetMode::ActorReference => {
            let remainder = input.unwrap_or_default().trim();
            if remainder.is_empty() {
                let actors_here: Vec<_> = content
                    .actors
                    .iter()
                    .filter(|actor| {
                        context
                            .planner_state
                            .actor_room_id(&actor.id, &actor.room_id)
                            == context.current_room_id
                    })
                    .collect();
                if actors_here.len() == 1 {
                    let actor = &actors_here[0];
                    if let Some(rejection) = ally_attack_rejection(
                        content,
                        context.planner_state,
                        action,
                        &actor.id,
                        &actor.name,
                    ) {
                        planned.events.push(rejection);
                        return false;
                    }
                    let room_id = context.current_room_id.to_string();
                    let actor_name = content.opening.title.as_str();
                    planned.events.push(WorldEvent::ActorCommandUsed {
                        actor_id: "player".to_string(),
                        actor_name: actor_name.to_string(),
                        room_id,
                        command_id: action.id.clone(),
                        target_room_id: None,
                        target_actor_id: Some(actor.id.clone()),
                        target_actor_name: Some(actor.name.clone()),
                        context_label: None,
                        feature_id: None,
                        consumable_id: None,
                        freeform_text: None,
                    });
                    if metadata.advances_time {
                        planned.events.push(WorldEvent::TurnStarted {
                            turn_number: context.turn_number,
                            raw_input: context.raw_input.to_string(),
                            advances_time: true,
                        });
                    }
                    true
                } else if actors_here.is_empty() {
                    planned.events.push(WorldEvent::ActionRejected {
                        message: "There's no one here to target.".to_string(),
                    });
                    false
                } else {
                    let names: Vec<&str> = actors_here.iter().map(|a| a.name.as_str()).collect();
                    planned.events.push(WorldEvent::ActionRejected {
                        message: format!("Who do you want to target? Try: {}", names.join(", ")),
                    });
                    false
                }
            } else if let Some(resolved) = resolve_actor_reference_input(
                content,
                context.planner_state,
                context.current_room_id,
                remainder,
            ) {
                if resolved.actor_in_room {
                    if let Some(rejection) = ally_attack_rejection(
                        content,
                        context.planner_state,
                        action,
                        &resolved.actor_id,
                        &resolved.actor_name,
                    ) {
                        planned.events.push(rejection);
                        return false;
                    }
                    let room_id = context.current_room_id.to_string();
                    let actor_name = content.opening.title.as_str();
                    planned.events.push(WorldEvent::ActorCommandUsed {
                        actor_id: "player".to_string(),
                        actor_name: actor_name.to_string(),
                        room_id,
                        command_id: action.id.clone(),
                        target_room_id: None,
                        target_actor_id: Some(resolved.actor_id),
                        target_actor_name: Some(resolved.actor_name),
                        context_label: None,
                        feature_id: None,
                        consumable_id: None,
                        freeform_text: None,
                    });
                    if metadata.advances_time {
                        planned.events.push(WorldEvent::TurnStarted {
                            turn_number: context.turn_number,
                            raw_input: context.raw_input.to_string(),
                            advances_time: true,
                        });
                    }
                    true
                } else {
                    planned.events.push(WorldEvent::ActionRejected {
                        message: content.render_template(
                            &content.presentation.error_text.actor_not_here,
                            &[("actor_name", resolved.actor_name.as_str())],
                        ),
                    });
                    false
                }
            } else {
                planned.events.push(WorldEvent::ActionRejected {
                    message: content.render_template(
                        &content.presentation.error_text.actor_unknown,
                        &[("target", unknown_target_token(remainder).as_str())],
                    ),
                });
                false
            }
        }
        other => panic!(
            "stateful player command '{}' has unsupported target_mode '{other:?}'",
            action.id
        ),
    }
}

pub(super) fn plan_targetless_command(
    content: &ContentPack,
    action: &ActionDefinition,
    context: &PlanningContext<'_>,
    planned: &mut PlannedTurn,
) -> bool {
    let metadata = action
        .player_command
        .as_ref()
        .unwrap_or_else(|| panic!("action '{}' should define player_command", action.id));
    // Packs with a finite per-tag supply refuse placement once that tag's pool
    // is empty.
    if action.has_effect(CommandEffect::DropItem)
        && !context.planner_state.has_item(&action.item_id)
    {
        planned.events.push(WorldEvent::ActionRejected {
            message: content
                .render_message(&format!("item.{}.none_held", action.item_id), &[])
                .or_else(|| content.render_message("item.none_held", &[]))
                .unwrap_or_default(),
        });
        return false;
    }
    if action.has_effect(CommandEffect::EquipItem) {
        let item_label = content
            .item(&action.item_id)
            .map(|item| item.label.as_str())
            .unwrap_or_default();
        let already_equipped = content.item(&action.item_id).is_some_and(|item| {
            context.planner_state.equipped_item(&item.equip_slot) == Some(action.item_id.as_str())
        });
        if already_equipped {
            planned.events.push(WorldEvent::ActionRejected {
                message: content
                    .render_message("equipment.already_equipped", &[("item", item_label)])
                    .unwrap_or_default(),
            });
            return false;
        }
        if !context.planner_state.has_item(&action.item_id) {
            planned.events.push(WorldEvent::ActionRejected {
                message: content
                    .render_message(&format!("item.{}.none_held", action.item_id), &[])
                    .or_else(|| content.render_message("item.none_held", &[]))
                    .unwrap_or_default(),
            });
            return false;
        }
    }
    if action.has_effect(CommandEffect::UnequipItem) {
        let equipped = content
            .item(&action.item_id)
            .and_then(|item| context.planner_state.equipment.get(&item.equip_slot));
        if equipped.map(String::as_str) != Some(action.item_id.as_str()) {
            planned.events.push(WorldEvent::ActionRejected {
                message: content
                    .render_message(
                        "equipment.not_equipped",
                        &[(
                            "item",
                            content
                                .item(&action.item_id)
                                .map(|item| item.label.as_str())
                                .unwrap_or_default(),
                        )],
                    )
                    .unwrap_or_default(),
            });
            return false;
        }
    }
    if action.has_effect(CommandEffect::UseItem) && !context.planner_state.has_item(&action.item_id)
    {
        planned.events.push(WorldEvent::ActionRejected {
            message: content
                .render_message(&format!("item.{}.none_held", action.item_id), &[])
                .or_else(|| content.render_message("item.none_held", &[]))
                .unwrap_or_default(),
        });
        return false;
    }
    let room_id = context.current_room_id.to_string();
    let actor_name = content.opening.title.as_str();
    planned.events.push(WorldEvent::ActorCommandUsed {
        actor_id: "player".to_string(),
        actor_name: actor_name.to_string(),
        room_id,
        command_id: action.id.clone(),
        target_room_id: None,
        target_actor_id: None,
        target_actor_name: None,
        context_label: None,
        feature_id: None,
        consumable_id: None,
        freeform_text: None,
    });
    if metadata.advances_time {
        planned.events.push(WorldEvent::TurnStarted {
            turn_number: context.turn_number,
            raw_input: action.id.clone(),
            advances_time: true,
        });
    }
    true
}

pub(super) fn plan_command_effects(
    content: &ContentPack,
    action: &ActionDefinition,
    input: Option<&str>,
    context: &PlanningContext<'_>,
    planned: &mut PlannedTurn,
) -> bool {
    if action.has_effect(CommandEffect::ObserveRoom) {
        let target = input.unwrap_or_default().trim();
        if target.is_empty() {
            plan_observe_room(context, planned)
        } else {
            plan_observe_target(content, target, context, planned)
        }
    } else if action.has_any_effect(&[
        CommandEffect::ObserveFeature,
        CommandEffect::ObserveActor,
        CommandEffect::MoveActor,
        CommandEffect::AttackTarget,
    ]) {
        plan_targeted_state_command(content, action, input, context, planned)
    } else if action.has_any_effect(&[
        CommandEffect::DropItem,
        CommandEffect::PickUpItem,
        CommandEffect::EquipItem,
        CommandEffect::UnequipItem,
        CommandEffect::UseItem,
    ]) {
        plan_targetless_command(content, action, context, planned)
    } else {
        panic!(
            "player command '{}' uses command effects without a supported planner effect",
            action.id
        )
    }
}

pub(super) fn plan_dialogue_command(
    content: &ContentPack,
    action: &ActionDefinition,
    input: Option<&str>,
    context: &PlanningContext<'_>,
    planned: &mut PlannedTurn,
) -> bool {
    let metadata = action
        .player_command
        .as_ref()
        .unwrap_or_else(|| panic!("action '{}' should define player_command", action.id));
    if context.channel_surfing_only {
        planned.events.push(WorldEvent::UnknownInput {
            raw_input: context.raw_input.to_string(),
        });
        return false;
    }

    match metadata.target_mode {
        PlayerCommandTargetMode::ActorReference => {
            let remainder = input.unwrap_or_default();
            if let Some(resolved) = resolve_actor_reference_input(
                content,
                context.planner_state,
                context.current_room_id,
                remainder,
            ) {
                if resolved.actor_in_room {
                    planned.pending_dialogue = Some(pending_dialogue_for(
                        content,
                        context,
                        resolved.actor_id,
                        resolved.player_message,
                    ));
                    metadata.advances_time
                } else {
                    planned.events.push(WorldEvent::ActionRejected {
                        message: content.render_template(
                            &content.presentation.error_text.actor_not_here,
                            &[("actor_name", resolved.actor_name.as_str())],
                        ),
                    });
                    false
                }
            } else {
                planned.events.push(WorldEvent::ActionRejected {
                    message: content.render_template(
                        &content.presentation.error_text.actor_unknown,
                        &[("target", unknown_target_token(remainder).as_str())],
                    ),
                });
                false
            }
        }
        PlayerCommandTargetMode::FirstActorInRoom => {
            let actors_here: Vec<_> = content
                .actors
                .iter()
                .filter(|actor| {
                    context
                        .planner_state
                        .actor_room_id(&actor.id, &actor.room_id)
                        == context.current_room_id
                })
                .collect();
            if let Some(actor) = actors_here.first() {
                planned.pending_dialogue = Some(pending_dialogue_for(
                    content,
                    context,
                    actor.id.clone(),
                    None,
                ));
                metadata.advances_time
            } else {
                planned.events.push(WorldEvent::ActionRejected {
                    message: "There is no one here to listen to.".to_string(),
                });
                false
            }
        }
        other => {
            panic!(
                "dialogue player command '{}' has unsupported target_mode '{other:?}'",
                action.id,
            )
        }
    }
}

pub(super) fn plan_authored_command(
    content: &ContentPack,
    command_id: &str,
    input: Option<&str>,
    context: PlanningContext<'_>,
    planned: &mut PlannedTurn,
) -> bool {
    let action = content
        .command(command_id)
        .unwrap_or_else(|| panic!("missing command definition '{command_id}'"));
    if action.outcome_mode == CommandOutcomeMode::Dialogue {
        plan_dialogue_command(content, action, input, &context, planned)
    } else if !action.effects.is_empty() {
        plan_command_effects(content, action, input, &context, planned)
    } else {
        plan_content_command(content, action, input, &context, planned)
    }
}
