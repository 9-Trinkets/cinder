use crate::content::types::{ActionDefinition, ContentPack, ItemDefinition};
use crate::engine::hooks::apply_narrating_world_hook_effects;
use crate::engine::narrative::NarrativeLines;
use crate::engine::state::WorldState;
use serde_json::json;

use super::combat::actor_display_name;

/// Equips one unit of the action's item, returning the previous occupant of
/// the slot to inventory. Bonuses remain derived from equipment.
pub(super) fn apply_equip(
    state: &mut WorldState,
    content: &ContentPack,
    command: &ActionDefinition,
    lines: &mut NarrativeLines,
) {
    let Some(item) = content.item(&command.item_id) else {
        return;
    };
    if !item.is_equippable() || !state.has_item(&command.item_id) {
        return;
    }
    let previous = state.equipment.get(&item.equip_slot).cloned();
    if !state.remove_item(&command.item_id) {
        return;
    }
    state
        .equipment
        .insert(item.equip_slot.clone(), command.item_id.clone());
    if let Some(old_item_id) = previous.filter(|old| old.as_str() != command.item_id) {
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
        lines.narration(line);
    }
}

pub(super) fn apply_unequip(
    state: &mut WorldState,
    content: &ContentPack,
    command: &ActionDefinition,
    lines: &mut NarrativeLines,
) {
    let Some(item) = content.item(&command.item_id) else {
        return;
    };
    if state.equipment.get(&item.equip_slot).map(String::as_str) != Some(command.item_id.as_str()) {
        return;
    }
    state.equipment.remove(&item.equip_slot);
    state.add_item(&command.item_id);
    if let Some(line) = render_equipment_message(content, state, "equipment.unequipped", item) {
        lines.narration(line);
    }
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
