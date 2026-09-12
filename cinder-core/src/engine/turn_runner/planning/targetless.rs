use super::super::types::PlannedTurn;
use super::PlanningContext;
use crate::content::types::{ActionDefinition, CommandEffect, ContentPack};
use crate::engine::events::WorldEvent;
use crate::engine::turn_policies::{command_availability_issue, command_unavailable_message};

pub(super) fn plan_targetless_command(
    content: &ContentPack,
    action: &ActionDefinition,
    context: &PlanningContext<'_>,
    planned: &mut PlannedTurn,
) -> bool {
    if let Some(issue) = command_availability_issue(content, context.planner_state, action) {
        planned.events.push(WorldEvent::ActionRejected {
            message: command_unavailable_message(content, action, &issue),
        });
        return false;
    }
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
        let already_equipped = content
            .item(&action.item_id)
            .is_some_and(|item| context.planner_state.item_is_equipped(item));
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
            .is_some_and(|item| context.planner_state.item_is_equipped(item));
        if !equipped {
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
