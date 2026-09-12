use crate::engine::turn_runner::types::PlannedTurn;
use crate::content::types::ContentPack;
use crate::engine::events::WorldEvent;
use crate::engine::state::WorldState;

/// Plans a generic `take <item>` command. Resolves the bare target against
/// loose items lying in the current room (matching by id or label) and moves
/// the first match to the player's inventory. Emits an `ActionRejected` when
/// nothing in the room matches.
pub(super) fn plan_take_command(
    content: &ContentPack,
    planner_state: &WorldState,
    current_room_id: &str,
    target: &str,
    planned: &mut PlannedTurn,
) -> bool {
    let trimmed = target.trim();
    let loose = planner_state.loose_room_items(current_room_id);
    if loose.is_empty() || trimmed.is_empty() {
        planned.events.push(WorldEvent::ActionRejected {
            message: content
                .render_message("item.take_nothing", &[])
                .unwrap_or_default(),
        });
        return false;
    }
    let target_lower = trimmed.to_ascii_lowercase();
    let mut matched = None;
    let mut trace_label = None;
    for (item_id, _count) in &loose {
        let Some(item) = content.item(item_id) else {
            continue;
        };
        let named = item.id.eq_ignore_ascii_case(&target_lower)
            || item.label.eq_ignore_ascii_case(&target_lower);
        if !named {
            continue;
        }
        if item.is_takeable() {
            matched = Some(item_id);
        } else {
            trace_label = Some(item.label.as_str());
        }
        break;
    }
    if let Some(label) = trace_label {
        planned.events.push(WorldEvent::ActionRejected {
            message: content
                .render_message("item.takedenied", &[("label", label)])
                .unwrap_or_default(),
        });
        return false;
    }
    match matched {
        Some(item_id) => {
            planned.events.push(WorldEvent::PlayerTookItem {
                item_id: item_id.clone(),
            });
            true
        }
        None => {
            planned.events.push(WorldEvent::ActionRejected {
                message: content
                    .render_message("item.take_not_here", &[])
                    .unwrap_or_default(),
            });
            false
        }
    }
}

/// Plans a generic `drop <item>` command. Resolves the bare target against the
/// player's inventory (matching by id or label) and moves the first match from
/// the inventory into the current room. Equipped items must be unequipped
/// first and are never droppable. Emits an `ActionRejected` when the item is
/// missing or currently equipped.
pub(super) fn plan_drop_command(
    content: &ContentPack,
    planner_state: &WorldState,
    target: &str,
    planned: &mut PlannedTurn,
) -> bool {
    let trimmed = target.trim();
    if trimmed.is_empty() {
        planned.events.push(WorldEvent::ActionRejected {
            message: content
                .render_message("item.drop_nothing", &[])
                .unwrap_or_default(),
        });
        return false;
    }
    let target_lower = trimmed.to_ascii_lowercase();
    let item_matches = |(item_id, label): (&str, &str)| {
        item_id.eq_ignore_ascii_case(&target_lower) || label.eq_ignore_ascii_case(&target_lower)
    };
    let target_is_equipped = planner_state.equipment.values().any(|equipped_id| {
        content
            .item(equipped_id)
            .is_some_and(|item| item_matches((equipped_id.as_str(), item.label.as_str())))
    });
    if target_is_equipped {
        planned.events.push(WorldEvent::ActionRejected {
            message: content
                .render_message("item.drop_equipped", &[])
                .unwrap_or_default(),
        });
        return false;
    }
    let matched = planner_state
        .player_inventory
        .iter()
        .find_map(|(item_id, _)| {
            content.item(item_id).and_then(|item| {
                if !item.is_takeable() {
                    return None;
                }
                if item_matches((item_id.as_str(), item.label.as_str())) {
                    Some((item_id.clone(), item.label.clone()))
                } else {
                    None
                }
            })
        });
    match matched {
        Some((item_id, _label)) => {
            planned
                .events
                .push(WorldEvent::PlayerDroppedItem { item_id });
            true
        }
        None => {
            planned.events.push(WorldEvent::ActionRejected {
                message: content
                    .render_message("item.drop_not_have", &[])
                    .unwrap_or_default(),
            });
            false
        }
    }
}