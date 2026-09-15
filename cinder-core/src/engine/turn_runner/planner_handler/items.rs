use crate::content::types::ContentPack;
use crate::engine::events::WorldEvent;
use crate::engine::state::WorldState;
use crate::engine::turn_runner::types::PlannedTurn;

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
    if !content.settings.allow_player_item_transfers {
        planned.events.push(WorldEvent::UnknownInput {
            raw_input: format!("take {target}"),
        });
        return false;
    }
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
    if !content.settings.allow_player_item_transfers {
        planned.events.push(WorldEvent::UnknownInput {
            raw_input: format!("drop {target}"),
        });
        return false;
    }
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

pub(super) fn plan_equip_command(
    content: &ContentPack,
    planner_state: &WorldState,
    target: &str,
    planned: &mut PlannedTurn,
) -> bool {
    let target = target.trim();
    let candidates = matching_items(content, target);
    let held = candidates
        .iter()
        .copied()
        .filter(|item| planner_state.has_item(&item.id))
        .collect::<Vec<_>>();
    if held.len() > 1 {
        reject_equipment_command(content, planned, "equipment.ambiguous_item", target);
        return false;
    }
    if let Some(item) = held.first().copied() {
        if !item.is_equippable() {
            reject_equipment_command(content, planned, "equipment.not_equippable", &item.label);
            return false;
        }
        planned.events.push(WorldEvent::PlayerEquippedItem {
            item_id: item.id.clone(),
        });
        return true;
    }
    if candidates
        .iter()
        .any(|item| planner_state.item_is_equipped(item))
    {
        let label = candidates
            .iter()
            .find(|item| planner_state.item_is_equipped(item))
            .map(|item| item.label.as_str())
            .unwrap_or(target);
        reject_equipment_command(content, planned, "equipment.already_equipped", label);
        return false;
    }
    let Some(item) = candidates.first() else {
        reject_equipment_command(content, planned, "equipment.item_not_held", target);
        return false;
    };
    if !item.is_equippable() {
        reject_equipment_command(content, planned, "equipment.not_equippable", &item.label);
        return false;
    }
    reject_equipment_command(content, planned, "equipment.item_not_held", &item.label);
    false
}

pub(super) fn plan_unequip_command(
    content: &ContentPack,
    planner_state: &WorldState,
    target: &str,
    planned: &mut PlannedTurn,
) -> bool {
    let target = target.trim();
    let equipped_ids = planner_state
        .equipment
        .values()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    let matched = matching_items(content, target)
        .into_iter()
        .filter(|item| equipped_ids.contains(item.id.as_str()))
        .collect::<Vec<_>>();
    if matched.len() > 1 {
        reject_equipment_command(content, planned, "equipment.ambiguous_item", target);
        return false;
    }
    let Some(item) = matched.first() else {
        reject_equipment_command(content, planned, "equipment.not_equipped", target);
        return false;
    };
    planned.events.push(WorldEvent::PlayerUnequippedItem {
        item_id: item.id.clone(),
    });
    true
}

fn matching_items<'a>(
    content: &'a ContentPack,
    target: &str,
) -> Vec<&'a crate::content::types::ItemDefinition> {
    let exact = content
        .items
        .iter()
        .filter(|item| {
            item.id.eq_ignore_ascii_case(target) || item.label.eq_ignore_ascii_case(target)
        })
        .collect::<Vec<_>>();
    if !exact.is_empty() {
        return exact;
    }
    let target_tokens = reference_tokens(target);
    if target_tokens.is_empty() {
        return Vec::new();
    }
    content
        .items
        .iter()
        .filter(|item| {
            let mut item_tokens = reference_tokens(&item.id);
            item_tokens.extend(reference_tokens(&item.label));
            target_tokens
                .iter()
                .all(|target_token| item_tokens.contains(target_token))
        })
        .collect()
}

fn reference_tokens(reference: &str) -> std::collections::BTreeSet<String> {
    reference
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(str::to_ascii_lowercase)
        .filter(|token| !matches!(token.as_str(), "a" | "an" | "the"))
        .collect()
}

fn reject_equipment_command(
    content: &ContentPack,
    planned: &mut PlannedTurn,
    message_key: &str,
    item_label: &str,
) {
    planned.events.push(WorldEvent::ActionRejected {
        message: content
            .render_message(message_key, &[("item", item_label)])
            .unwrap_or_default(),
    });
}
