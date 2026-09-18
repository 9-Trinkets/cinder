use crate::content::types::{ContentPack, PartyOrderKind};
use crate::engine::narrative::NarrativeLines;
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
    let key = format!("party.order_{order}_assigned");
    if let Err(error) = state.assign_party_order(content, actor_id, order) {
        eprintln!("[cinder] party order error: {error}");
        return;
    }
    let actor = actor_display_name(content, actor_id);
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

pub(crate) fn handle_player_gave_item_to_party_member(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    item_id: &str,
    lines: &mut NarrativeLines,
) {
    let Some(item) = content.item(item_id) else {
        return;
    };
    if !state.remove_item(item_id) {
        return;
    }
    let actor_name = actor_display_name(content, actor_id);
    let item_label = &item.label;

    let is_equippable = item.is_equippable()
        && item
            .occupied_slots()
            .iter()
            .all(|slot| content.settings.equipment_slots.contains(slot));

    if is_equippable {
        let actor_equipment = state.actor_equipment.entry(actor_id.to_string()).or_default();
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
            state.actor_add_item(actor_id, &old_item_id);
        }

        if !item.equip_hook.is_empty()
            && let Err(error) = crate::engine::hooks::apply_narrating_world_hook_effects(
                state,
                content,
                &item.equip_hook,
                serde_json::json!({
                    "actor_id": actor_id,
                    "actor_name": actor_name,
                    "item_id": item.id,
                    "item_label": item.label,
                }),
                lines,
            ) {
                eprintln!("[cinder] hook warning ({}): {error}", item.equip_hook);
            }

        let give_msg = content
            .render_message("party.give_success", &[("actor", &actor_name), ("item", item_label)])
            .unwrap_or_else(|| format!("You give the {item_label} to {actor_name}."));
        lines.narration(give_msg);

        let equip_msg = content
            .render_message("party.give_auto_equipped", &[("actor", &actor_name), ("item", item_label)])
            .unwrap_or_else(|| format!("{actor_name} equips the {item_label}."));
        lines.narration(equip_msg);
    } else {
        state.actor_add_item(actor_id, item_id);
        let give_msg = content
            .render_message("party.give_success", &[("actor", &actor_name), ("item", item_label)])
            .unwrap_or_else(|| format!("You give the {item_label} to {actor_name}."));
        lines.narration(give_msg);
    }
}

pub(crate) fn handle_player_took_item_from_party_member(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    item_id: &str,
    lines: &mut NarrativeLines,
) {
    let Some(item) = content.item(item_id) else {
        return;
    };
    let actor_name = actor_display_name(content, actor_id);
    let item_label = &item.label;

    let is_equipped = state
        .actor_equipment
        .get(actor_id)
        .map(|eq| eq.values().any(|v| v == item_id))
        .unwrap_or(false);

    if is_equipped {
        if let Some(equip) = state.actor_equipment.get_mut(actor_id) {
            let slots_to_remove: Vec<String> = equip
                .iter()
                .filter(|(_, id)| *id == item_id)
                .map(|(slot, _)| slot.clone())
                .collect();
            for slot in slots_to_remove {
                equip.remove(&slot);
            }
        }
        state.add_item(item_id);
        let take_msg = content
            .render_message("party.take_success", &[("actor", &actor_name), ("item", item_label)])
            .unwrap_or_else(|| format!("You take the {item_label} from {actor_name}."));
        lines.narration(take_msg);
    } else if state.actor_has_item(actor_id, item_id)
        && state.actor_remove_item(actor_id, item_id) {
            state.add_item(item_id);
            let take_msg = content
                .render_message("party.take_success", &[("actor", &actor_name), ("item", item_label)])
                .unwrap_or_else(|| format!("You take the {item_label} from {actor_name}."));
            lines.narration(take_msg);
        }
}
