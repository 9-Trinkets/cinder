use crate::content::types::{ActionDefinition, ContentPack, ItemDefinition};
use crate::engine::hooks::apply_narrating_world_hook_effects;
use crate::engine::narrative::NarrativeLines;
use crate::engine::state::WorldState;
use serde_json::json;
use std::collections::BTreeSet;

use super::combat::actor_display_name;
use super::handlers::push_rendered_message;

/// Equips one unit of the action's item. Multi-slot items (e.g. two-hand
/// weapons) occupy every declared slot; the previous occupants of all those
/// slots return to inventory, so a two-hander swapped for a pair frees both
/// hands and vice-versa. Bonuses remain derived from equipment.
pub(super) fn apply_equip(
    state: &mut WorldState,
    content: &ContentPack,
    item_id: &str,
    lines: &mut NarrativeLines,
) {
    let Some(item) = content.item(item_id) else {
        return;
    };
    if !item.is_equippable()
        || !state.has_item(item_id)
        || !item
            .occupied_slots()
            .iter()
            .all(|slot| content.settings.equipment_slots.contains(slot))
    {
        return;
    }
    render_item_equipment_text(content, item, "equipped", lines);
    if !state.remove_item(item_id) {
        return;
    }
    // Free every previously-equipped item that shares one of the new item's
    // slots, across all of that old item's slots. Swapping a two-hand weapon
    // for a one-hander returns the bow entirely instead of orphaning an
    // off-hand, and the reverse clears both hands.
    let item = content.item(item_id).expect("guard checked the item");
    let replaced: BTreeSet<String> = item
        .occupied_slots()
        .iter()
        .filter_map(|slot| state.equipment.get(slot).cloned())
        .filter(|old_item_id| old_item_id != item_id)
        .collect();
    let dangling_slots: Vec<String> = state
        .equipment
        .iter()
        .filter(|(_, occupant)| replaced.contains(*occupant))
        .map(|(slot, _)| slot.clone())
        .collect();
    for slot in dangling_slots {
        state.equipment.remove(&slot);
    }
    for slot in item.occupied_slots() {
        state.equipment.insert(slot.clone(), item_id.to_string());
    }
    for old_item_id in replaced {
        state.add_item(&old_item_id);
    }
    if !item.equip_hook.is_empty() {
        let player_id = content.settings.combat.player_actor_id.clone();
        if let Err(error) = apply_narrating_world_hook_effects(
            state,
            content,
            &item.equip_hook,
            json!({
                "actor_id": player_id,
                "actor_name": actor_display_name(content, &player_id),
                "item_id": item.id,
                "item_label": item.label,
            }),
            lines,
        ) {
            eprintln!("[cinder] hook warning ({}): {error}", item.equip_hook);
        }
    }
    if let Some(line) = render_equipment_message(content, state, "equipment.equipped", item) {
        push_rendered_message(
            lines,
            content,
            line,
            content.message_voice("equipment.equipped"),
        );
    }
}

pub(super) fn apply_unequip(
    state: &mut WorldState,
    content: &ContentPack,
    item_id: &str,
    lines: &mut NarrativeLines,
) {
    let Some(item) = content.item(item_id) else {
        return;
    };
    if !state.item_is_equipped(item) {
        return;
    }
    render_item_equipment_text(content, item, "unequipped", lines);
    for slot in item.occupied_slots() {
        state.equipment.remove(slot);
    }
    state.add_item(item_id);
    if let Some(line) = render_equipment_message(content, state, "equipment.unequipped", item) {
        push_rendered_message(
            lines,
            content,
            line,
            content.message_voice("equipment.unequipped"),
        );
    }
}

fn render_item_equipment_text(
    content: &ContentPack,
    item: &ItemDefinition,
    state: &str,
    lines: &mut NarrativeLines,
) {
    let key = format!("equipment.{}.{}", item.id, state);
    let Some(template) = content.message(&key) else {
        return;
    };
    let actor_id = &content.settings.combat.player_actor_id;
    let actor_name = actor_display_name(content, actor_id);
    lines.narration(content.render_template(
        template,
        &[
            ("actor_name", actor_name.as_str()),
            ("item", item.label.as_str()),
        ],
    ));
}

/// Consumes one unit and lets the item's content-authored hook define its
/// effect.
pub(super) fn apply_use_item(
    state: &mut WorldState,
    content: &ContentPack,
    command: &ActionDefinition,
    lines: &mut NarrativeLines,
) {
    let Some(item) = content.item(&command.item_id) else {
        return;
    };
    if item.use_hook.is_empty() || !state.remove_item(&command.item_id) {
        return;
    }
    let player_id = content.settings.combat.player_actor_id.clone();
    if let Err(error) = apply_narrating_world_hook_effects(
        state,
        content,
        &item.use_hook,
        json!({
            "actor_id": player_id,
            "actor_name": actor_display_name(content, &player_id),
            "item_id": item.id,
            "item_label": item.label,
        }),
        lines,
    ) {
        eprintln!("[cinder] hook warning ({}): {error}", item.use_hook);
    }
    if let Some(line) = content.render_message("item.used", &[("item", item.label.as_str())]) {
        lines.narration(line);
    }
}

fn render_equipment_message(
    content: &ContentPack,
    state: &WorldState,
    key: &str,
    item: &ItemDefinition,
) -> Option<String> {
    let combat = &content.settings.combat;
    let mut bonuses = item
        .stat_bonuses
        .iter()
        .map(|(stat_id, delta)| {
            (
                stat_id.clone(),
                format!(
                    "{delta:+} ({})",
                    state.effective_actor_stat(content, &combat.player_actor_id, stat_id)
                ),
            )
        })
        .collect::<Vec<_>>();
    bonuses.sort();
    let bonus_text = bonuses
        .iter()
        .map(|(stat_id, text)| format!("{stat_id} {text}"))
        .collect::<Vec<_>>()
        .join(", ");
    content.render_message(
        key,
        &[
            ("item", item.label.as_str()),
            ("bonuses", bonus_text.as_str()),
        ],
    )
}
