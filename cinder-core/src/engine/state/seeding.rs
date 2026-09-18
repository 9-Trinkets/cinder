use super::WorldState;
use super::consumable_key;
use crate::content::types::{ContentPack, StatDefinition};
use std::collections::BTreeMap;

pub(super) fn seeded_pair_stats(
    content: &ContentPack,
    pair_stat_defs: &BTreeMap<String, StatDefinition>,
) -> BTreeMap<String, BTreeMap<String, i32>> {
    let mut pair_stats = BTreeMap::new();
    for actor in &content.actors {
        for (other_actor_id, authored_stats) in &actor.initial_pair_stats {
            let key = WorldState::conversation_key(&actor.id, other_actor_id);
            let entry = pair_stats.entry(key).or_insert_with(BTreeMap::new);
            for (stat_key, stat_value) in authored_stats {
                if let Some(definition) = pair_stat_defs.get(stat_key) {
                    entry.insert(stat_key.clone(), definition.clamp(*stat_value));
                }
            }
        }
    }
    pair_stats
}

pub(super) fn seeded_actor_stats(
    content: &ContentPack,
    actor_stat_defs: &BTreeMap<String, StatDefinition>,
) -> BTreeMap<String, BTreeMap<String, i32>> {
    content
        .actors
        .iter()
        .map(|actor| {
            let mut stats = actor_stat_defs
                .iter()
                .map(|(key, definition)| (key.clone(), definition.default))
                .collect::<BTreeMap<_, _>>();
            for (stat_key, stat_value) in &actor.initial_stats {
                if let Some(definition) = actor_stat_defs.get(stat_key) {
                    stats.insert(stat_key.clone(), definition.clamp(*stat_value));
                }
            }
            (actor.id.clone(), stats)
        })
        .collect()
}

pub(super) fn seeded_actor_levels(content: &ContentPack) -> BTreeMap<String, u32> {
    content
        .actors
        .iter()
        .map(|actor| (actor.id.clone(), actor.level.max(1)))
        .collect()
}

pub(super) fn seeded_feature_consumable_stock(content: &ContentPack) -> BTreeMap<String, u32> {
    let mut stock = BTreeMap::new();
    for room in &content.rooms {
        for feature in &room.features {
            for consumable in &feature.consumables {
                stock.insert(
                    consumable_key(&room.id, &feature.id, &consumable.id),
                    consumable.initial_stock,
                );
            }
        }
    }
    stock
}

pub(super) fn seeded_actor_equipment(
    content: &ContentPack,
) -> BTreeMap<String, BTreeMap<String, String>> {
    let mut equipment = BTreeMap::new();
    for actor in &content.actors {
        if !actor.initial_equipment.is_empty() {
            equipment.insert(actor.id.clone(), actor.initial_equipment.clone());
        }
    }
    equipment
}

pub(super) fn seeded_actor_inventories(
    content: &ContentPack,
) -> BTreeMap<String, std::collections::HashMap<String, u32>> {
    let mut inventories = BTreeMap::new();
    for actor in &content.actors {
        if !actor.initial_inventory.is_empty() {
            inventories.insert(
                actor.id.clone(),
                actor.initial_inventory.clone().into_iter().collect(),
            );
        }
    }
    inventories
}
