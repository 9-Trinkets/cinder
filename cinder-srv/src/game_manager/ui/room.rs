use cinder_core::content::types::{ConsumableDefinition, ContentPack};
use cinder_core::engine::runtime::CinderRuntime;
use std::collections::BTreeSet;

use super::{ConsumableInfo, RoomConsumableGroup};

pub(super) fn highlighted_consumable(
    crafted_consumable_ids: &BTreeSet<String>,
    consumable: &ConsumableDefinition,
) -> bool {
    crafted_consumable_ids.contains(&consumable.id) || consumable.initial_stock > 0
}

pub(super) fn build_room_consumables(
    runtime: &CinderRuntime,
    content: &ContentPack,
    current_room_id: &str,
) -> Vec<RoomConsumableGroup> {
    let crafted_consumable_ids = content.crafted_current_room_item_ids();
    content
        .room_consumables(current_room_id)
        .into_iter()
        .filter(|c| remaining_in_room(runtime, current_room_id, c) > 0)
        .fold(Vec::<RoomConsumableGroup>::new(), |mut groups, c| {
            let remaining = remaining_in_room(runtime, current_room_id, &c);
            if let Some(group) = groups
                .iter_mut()
                .find(|g| g.feature_label == c.feature.label)
            {
                group.items.push(room_consumable(
                    &crafted_consumable_ids,
                    c,
                    remaining,
                ));
            } else {
                groups.push(RoomConsumableGroup {
                    feature_label: c.feature.label.clone(),
                    items: vec![room_consumable(
                        &crafted_consumable_ids,
                        c,
                        remaining,
                    )],
                });
            }
            groups
        })
}

pub(super) fn crafted_consumable_labels(
    content: &ContentPack,
    current_room_id: &str,
) -> Vec<String> {
    let crafted_consumable_ids = content.crafted_current_room_item_ids();
    content
        .room_consumables(current_room_id)
        .into_iter()
        .filter(|c| highlighted_consumable(&crafted_consumable_ids, c.consumable))
        .map(|c| c.consumable.label.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn remaining_in_room(
    runtime: &CinderRuntime,
    current_room_id: &str,
    c: &cinder_core::content::types::RoomConsumableRef<'_>,
) -> u32 {
    runtime
        .current_room_item_count(&c.consumable.id)
        .unwrap_or(0)
        + runtime
            .feature_consumable_count(current_room_id, &c.feature.id, &c.consumable.id)
            .unwrap_or(0)
}

fn room_consumable(
    crafted_consumable_ids: &BTreeSet<String>,
    c: cinder_core::content::types::RoomConsumableRef<'_>,
    remaining: u32,
) -> ConsumableInfo {
    ConsumableInfo {
        id: c.consumable.id.clone(),
        label: c.consumable.label.clone(),
        kind: format!("{:?}", c.consumable.kind).to_lowercase(),
        stock: remaining,
        is_crafted: highlighted_consumable(crafted_consumable_ids, c.consumable),
    }
}
