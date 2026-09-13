use cinder_core::content::types::ContentPack;
use cinder_core::engine::state::WorldState;
use std::collections::BTreeSet;

use super::PanelOptionData;

/// Builds one bounded equipment picker from held and equipped items.
pub(crate) fn build_equipment_panel_options(
    content: &ContentPack,
    state: &WorldState,
) -> Vec<PanelOptionData> {
    let mut options = Vec::new();
    let equipped_ids: BTreeSet<&str> = state.equipment.values().map(String::as_str).collect();

    for item_id in &equipped_ids {
        let Some(item) = content.item(item_id) else {
            continue;
        };
        options.push(PanelOptionData {
            id: format!("unequip:{item_id}"),
            title: item.label.clone(),
            subtitle: Some(format!("Unequip: {}", item.occupied_slots().join(" + "))),
            command: Some(format!("unequip {item_id}")),
            disabled: false,
            selected: false,
        });
    }

    for (item_id, count) in &state.player_inventory {
        let Some(item) = content.item(item_id) else {
            continue;
        };
        if *count == 0 || !item.is_equippable() || equipped_ids.contains(item_id.as_str()) {
            continue;
        }
        let replaced: BTreeSet<&str> = item
            .occupied_slots()
            .iter()
            .filter_map(|slot| state.equipment.get(slot).map(String::as_str))
            .collect();
        let replacement = if replaced.is_empty() {
            String::new()
        } else {
            let labels = replaced
                .iter()
                .map(|id| content.item_label(id).to_string())
                .collect::<Vec<_>>()
                .join(", ");
            format!(" | replaces {labels}")
        };
        options.push(PanelOptionData {
            id: format!("equip:{item_id}"),
            title: item.label.clone(),
            subtitle: Some(format!(
                "Equip: {}{replacement}",
                item.occupied_slots().join(" + ")
            )),
            command: Some(format!("equip {item_id}")),
            disabled: false,
            selected: false,
        });
    }

    options
}
