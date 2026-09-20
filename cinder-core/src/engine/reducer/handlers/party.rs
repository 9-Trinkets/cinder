use crate::content::types::{ContentPack, PartyOrderKind};
use crate::engine::narrative::NarrativeLines;
use crate::engine::reducer::beat_advance::advance_objective_for_signal;
use crate::engine::state::WorldState;

use super::super::command_effects::actor_display_name;
use super::feedback::push_message;

pub(crate) fn handle_party_order_assigned(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    order: PartyOrderKind,
    lines: &mut NarrativeLines,
) {
    let was_in_room = state.actor_is_in_room(content, actor_id, &state.current_room_id);
    if let Err(error) = state.assign_party_order(content, actor_id, order.clone()) {
        eprintln!("[cinder] party order error: {error}");
        return;
    }
    let actor = actor_display_name(content, actor_id);
    let player_room = state.current_room_id.clone();
    let key = if !was_in_room && (order == "guard" || order == "follow") {
        state.mark_actor_room_visited(actor_id, &player_room);
        state
            .actor_room_overrides
            .insert(actor_id.to_string(), player_room);
        let recall_key = format!("party.order_{order}_recalled");
        if content.message(&recall_key).is_some() {
            recall_key
        } else {
            format!("party.order_{order}_assigned")
        }
    } else {
        format!("party.order_{order}_assigned")
    };
    push_message(lines, content, &key, &[("actor", actor.as_str())]);
}

pub(crate) fn handle_player_followed_actor(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: Option<&str>,
    lines: &mut NarrativeLines,
) {
    state.followed_actor_id = actor_id.map(ToString::to_string);
    let feedback_text = match actor_id {
        Some(id) => {
            let actor_name = content
                .actor(id)
                .map(|a| crate::engine::state::display_actor_name(state, a))
                .unwrap_or_else(|| id.to_string());
            content
                .ui_text
                .follow_actor_transcript
                .replace("{title}", &actor_name)
        }
        None => content.ui_text.follow_actor_stop_transcript.clone(),
    };
    if !feedback_text.trim().is_empty() {
        lines.narration(feedback_text);
    }
}

pub(crate) fn handle_item_transferred(
    state: &mut WorldState,
    content: &ContentPack,
    item_id: &str,
    from_actor_id: &str,
    to_actor_id: &str,
    initiator_actor_id: Option<&str>,
    lines: &mut NarrativeLines,
) {
    let Some(item) = content.item(item_id) else {
        return;
    };
    let item_label = &item.label;

    // 1. Remove from giver
    if content.is_player_actor(from_actor_id) {
        if !state.remove_item(item_id) {
            return;
        }
    } else {
        let is_equipped = state
            .actor_equipment
            .get(from_actor_id)
            .map(|eq| eq.values().any(|v| v == item_id))
            .unwrap_or(false);

        if is_equipped {
            if let Some(equip) = state.actor_equipment.get_mut(from_actor_id) {
                let slots_to_remove: Vec<String> = equip
                    .iter()
                    .filter(|(_, id)| *id == item_id)
                    .map(|(slot, _)| slot.clone())
                    .collect();
                for slot in slots_to_remove {
                    equip.remove(&slot);
                }
            }
        } else if state.actor_has_item(from_actor_id, item_id) {
            if !state.actor_remove_item(from_actor_id, item_id) {
                return;
            }
        } else {
            return;
        }
    }

    // 2. Add to recipient
    let mut auto_equipped = false;
    if content.is_player_actor(to_actor_id) {
        state.add_item(item_id);
    } else {
        let is_equippable = item.is_equippable()
            && item
                .occupied_slots()
                .iter()
                .all(|slot| content.settings.equipment_slots.contains(slot));

        if is_equippable {
            auto_equipped = true;
            let actor_equipment = state.actor_equipment.entry(to_actor_id.to_string()).or_default();
            let replaced: std::collections::BTreeSet<String> = item
                .occupied_slots()
                .iter()
                .filter_map(|slot| actor_equipment.get(slot).cloned())
                .filter(|old_id| old_id != item_id)
                .collect();
            let dangling_slots: Vec<String> = actor_equipment
                .iter()
                .filter(|(_, occupant)| replaced.contains(*occupant))
                .map(|(slot, _)| slot.clone())
                .collect();
            for slot in dangling_slots {
                actor_equipment.remove(&slot);
            }
            for slot in item.occupied_slots() {
                actor_equipment.insert(slot.clone(), item_id.to_string());
            }
            for old_item_id in replaced {
                state.actor_add_item(to_actor_id, &old_item_id);
            }

            let to_name = actor_display_name(content, to_actor_id);
            if !item.equip_hook.is_empty()
                && let Err(error) = crate::engine::hooks::apply_narrating_world_hook_effects(
                    state,
                    content,
                    &item.equip_hook,
                    serde_json::json!({
                        "actor_id": to_actor_id,
                        "actor_name": to_name,
                        "item_id": item.id,
                        "item_label": item.label,
                    }),
                    lines,
                ) {
                    eprintln!("[cinder] hook warning ({}): {error}", item.equip_hook);
                }
        } else {
            state.actor_add_item(to_actor_id, item_id);
        }
    }

    // 3. Narration
    if content.is_player_actor(from_actor_id) {
        let to_name = actor_display_name(content, to_actor_id);
        let give_msg = content
            .render_message("party.give_success", &[("actor", &to_name), ("item", item_label)])
            .unwrap_or_else(|| format!("You give the {item_label} to {to_name}."));
        lines.narration(give_msg);

        if auto_equipped {
            let equip_msg = content
                .render_message("party.give_auto_equipped", &[("actor", &to_name), ("item", item_label)])
                .unwrap_or_else(|| format!("{to_name} equips the {item_label}."));
            lines.narration(equip_msg);
        }
    } else if content.is_player_actor(to_actor_id) {
        let is_player_take = initiator_actor_id
            .map(|id| content.is_player_actor(id))
            .unwrap_or(false);
        let from_name = actor_display_name(content, from_actor_id);

        if is_player_take {
            let take_msg = content
                .render_message("party.take_success", &[("actor", &from_name), ("item", item_label)])
                .unwrap_or_else(|| format!("You take the {item_label} from {from_name}."));
            lines.narration(take_msg);
        } else {
            let gift_msg = content
                .render_message("party.npc_give_player", &[("actor", &from_name), ("item", item_label)])
                .unwrap_or_else(|| format!("{from_name} gives you the {item_label}."));
            lines.narration(gift_msg);
        }
    } else {
        let from_name = actor_display_name(content, from_actor_id);
        let to_name = actor_display_name(content, to_actor_id);
        lines.narration(format!("{from_name} gives the {item_label} to {to_name}."));
        if auto_equipped {
            lines.narration(format!("{to_name} equips the {item_label}."));
        }
    }

    // 4. Signals
    lines.extend_narration(advance_objective_for_signal(
        state,
        content,
        &format!("item_transferred:{item_id}"),
    ));
    if content.is_player_actor(to_actor_id) {
        lines.extend_narration(advance_objective_for_signal(
            state,
            content,
            &format!("item_acquired:{item_id}"),
        ));
        lines.extend_narration(advance_objective_for_signal(
            state,
            content,
            "item_acquired",
        ));
    }
}
