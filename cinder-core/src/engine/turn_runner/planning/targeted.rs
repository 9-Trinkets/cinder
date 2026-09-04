use super::super::types::PlannedTurn;
use super::observe::{plan_move_to_room_target, plan_observe_target};
use super::PlanningContext;
use crate::content::types::{
    ActionDefinition, CommandEffect, ContentPack, PlayerCommandTargetMode,
};
use crate::engine::commands::{resolve_actor_reference_input, unknown_target_token};
use crate::engine::events::WorldEvent;
use crate::engine::state::{ActorStance, WorldState};

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
