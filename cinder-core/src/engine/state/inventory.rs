use super::*;
use crate::content::types::ItemStorageTarget;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

/// Inventory and equipment holdings: the player's inventory, per-room item
/// stock and charges, and per-actor inventories and equipment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    #[serde(default)]
    pub player_inventory: HashMap<String, u32>,
    #[serde(default)]
    pub room_item_stock: BTreeMap<String, u32>,
    /// Remaining activations of charged room items, room id → item id → charges.
    #[serde(default)]
    pub room_item_charges: BTreeMap<String, u32>,
    /// Per-actor inventory (non-player actors), actor id → item id → count.
    #[serde(default)]
    pub actor_inventories: BTreeMap<String, HashMap<String, u32>>,
    /// Per-actor equipment (non-player actors), actor id → slot id → item id.
    #[serde(default)]
    pub actor_equipment: BTreeMap<String, BTreeMap<String, String>>,
    /// Player equipment, slot id → item id.
    #[serde(default)]
    pub equipment: BTreeMap<String, String>,
}

impl WorldState {
    pub fn has_item(&self, item_id: &str) -> bool {
        self.inventory
            .player_inventory
            .get(item_id)
            .copied()
            .unwrap_or(0)
            > 0
    }

    pub fn item_count(&self, item_id: &str) -> u32 {
        self.inventory
            .player_inventory
            .get(item_id)
            .copied()
            .unwrap_or(0)
    }

    pub fn add_item(&mut self, item_id: &str) {
        *self
            .inventory
            .player_inventory
            .entry(item_id.to_string())
            .or_insert(0) += 1;
    }

    pub fn remove_item(&mut self, item_id: &str) -> bool {
        let mut has = false;
        if let Some(count) = self.inventory.player_inventory.get_mut(item_id)
            && *count > 0
        {
            *count -= 1;
            has = true;
        }
        if let Some(0) = self.inventory.player_inventory.get(item_id) {
            self.inventory.player_inventory.remove(item_id);
        }
        has
    }

    pub fn actor_has_item(&self, actor_id: &str, item_id: &str) -> bool {
        self.actor_item_count(actor_id, item_id) > 0
    }

    pub fn actor_item_count(&self, actor_id: &str, item_id: &str) -> u32 {
        self.inventory
            .actor_inventories
            .get(actor_id)
            .and_then(|inv| inv.get(item_id))
            .copied()
            .unwrap_or(0)
    }

    pub fn actor_inventory(&self, actor_id: &str) -> Vec<(String, u32)> {
        if let Some(inv) = self.inventory.actor_inventories.get(actor_id) {
            let mut items: Vec<(String, u32)> =
                inv.iter().map(|(id, count)| (id.clone(), *count)).collect();
            items.sort_by(|a, b| a.0.cmp(&b.0));
            items
        } else {
            Vec::new()
        }
    }

    pub fn drain_actor_inventory(&mut self, actor_id: &str) -> Vec<(String, u32)> {
        if let Some(inv) = self.inventory.actor_inventories.remove(actor_id) {
            let mut items: Vec<(String, u32)> = inv.into_iter().filter(|(_, c)| *c > 0).collect();
            items.sort_by(|a, b| a.0.cmp(&b.0));
            items
        } else {
            Vec::new()
        }
    }

    pub fn actor_add_item(&mut self, actor_id: &str, item_id: &str) {
        *self
            .inventory
            .actor_inventories
            .entry(actor_id.to_string())
            .or_default()
            .entry(item_id.to_string())
            .or_insert(0) += 1;
    }

    pub fn actor_remove_item(&mut self, actor_id: &str, item_id: &str) -> bool {
        let mut has = false;
        if let Some(inv) = self.inventory.actor_inventories.get_mut(actor_id) {
            if let Some(count) = inv.get_mut(item_id)
                && *count > 0
            {
                *count -= 1;
                has = true;
            }
            if let Some(0) = inv.get(item_id) {
                inv.remove(item_id);
            }
        }
        has
    }

    pub fn has_item_in_storage(
        &self,
        item_id: &str,
        storage: ItemStorageTarget,
        current_room_id: &str,
    ) -> bool {
        self.item_count_in_storage(item_id, storage, current_room_id) > 0
    }

    pub fn item_count_in_storage(
        &self,
        item_id: &str,
        storage: ItemStorageTarget,
        current_room_id: &str,
    ) -> u32 {
        match storage {
            ItemStorageTarget::PlayerInventory => self.item_count(item_id),
            ItemStorageTarget::CurrentRoom => self
                .inventory
                .room_item_stock
                .get(&room_item_key(current_room_id, item_id))
                .copied()
                .unwrap_or(0),
        }
    }

    pub fn add_item_to_storage(
        &mut self,
        item_id: &str,
        storage: ItemStorageTarget,
        current_room_id: &str,
    ) {
        match storage {
            ItemStorageTarget::PlayerInventory => {
                let player_id = self.player_actor_id.clone();
                self.add_item(item_id);
                self.actor_add_item(&player_id, item_id);
            }
            ItemStorageTarget::CurrentRoom => {
                *self
                    .inventory
                    .room_item_stock
                    .entry(room_item_key(current_room_id, item_id))
                    .or_insert(0) += 1;
            }
        }
    }

    pub fn remove_item_from_storage(
        &mut self,
        item_id: &str,
        storage: ItemStorageTarget,
        current_room_id: &str,
    ) -> bool {
        match storage {
            ItemStorageTarget::PlayerInventory => {
                let player_id = self.player_actor_id.clone();
                let removed = self.remove_item(item_id);
                self.actor_remove_item(&player_id, item_id);
                removed
            }
            ItemStorageTarget::CurrentRoom => {
                let key = room_item_key(current_room_id, item_id);
                let mut removed = false;
                if let Some(count) = self.inventory.room_item_stock.get_mut(&key)
                    && *count > 0
                {
                    *count -= 1;
                    removed = true;
                }
                if let Some(0) = self.inventory.room_item_stock.get(&key) {
                    self.inventory.room_item_stock.remove(&key);
                    self.inventory.room_item_charges.remove(&key);
                }
                removed
            }
        }
    }

    /// Removes every loose instance of `item_id` from `room_id` at once,
    /// dropping the item's activation charges with it. Used when a charged
    /// room item is spent (a drain sigil that ran out of activations).
    pub fn remove_items_from_room(&mut self, room_id: &str, item_id: &str) -> bool {
        let key = room_item_key(room_id, item_id);
        let had = self.inventory.room_item_stock.remove(&key).is_some();
        self.inventory.room_item_charges.remove(&key);
        had
    }

    /// Loose items lying in `room_id` (from `ItemStorageTarget::CurrentRoom`),
    /// as `(item_id, count)`.
    pub fn loose_room_items(&self, room_id: &str) -> Vec<(String, u32)> {
        let prefix = format!("{room_id}::");
        self.inventory
            .room_item_stock
            .iter()
            .filter(|(key, _)| key.starts_with(&prefix))
            .map(|(key, count)| (key[prefix.len()..].to_string(), *count))
            .collect()
    }

    /// How many of `item_id` the feature holds right now.
    pub fn feature_item_count(&self, room_id: &str, feature_id: &str, item_id: &str) -> u32 {
        self.feature_item_stock
            .get(&feature_item_key(room_id, feature_id, item_id))
            .copied()
            .unwrap_or(0)
    }

    /// Takes one `item_id` out of the feature. False when it holds none.
    pub fn remove_feature_item(&mut self, room_id: &str, feature_id: &str, item_id: &str) -> bool {
        let key = feature_item_key(room_id, feature_id, item_id);
        let Some(stock) = self.feature_item_stock.get_mut(&key) else {
            return false;
        };
        if *stock == 0 {
            return false;
        }
        *stock -= 1;
        true
    }

    /// Puts `amount` more of `item_id` into the feature.
    pub fn add_feature_item(
        &mut self,
        room_id: &str,
        feature_id: &str,
        item_id: &str,
        amount: u32,
    ) {
        *self
            .feature_item_stock
            .entry(feature_item_key(room_id, feature_id, item_id))
            .or_insert(0) += amount;
    }

    /// What the feature holds right now, as `(item_id, count)`.
    pub fn feature_items(&self, room_id: &str, feature_id: &str) -> Vec<(String, u32)> {
        let prefix = format!("{room_id}::{feature_id}::");
        self.feature_item_stock
            .iter()
            .filter(|(key, count)| key.starts_with(&prefix) && **count > 0)
            .map(|(key, count)| (key[prefix.len()..].to_string(), *count))
            .collect()
    }

    /// Everything the player could pick up from the features of `room_id`, as
    /// `(feature_id, item_id, count)`. Consumables are used in place, so they are
    /// left out.
    pub fn takeable_feature_items(
        &self,
        content: &ContentPack,
        room_id: &str,
    ) -> Vec<(String, String, u32)> {
        let Some(room) = content.room(room_id) else {
            return Vec::new();
        };
        room.features
            .iter()
            .flat_map(|feature| {
                self.feature_items(room_id, &feature.id)
                    .into_iter()
                    .filter(|(item_id, _)| content.item(item_id).is_some_and(|i| i.is_takeable()))
                    .map(|(item_id, count)| (feature.id.clone(), item_id, count))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// Adds an item to the player's inventory and actor inventory simultaneously.
    pub fn acquire_player_item(&mut self, content: &ContentPack, item_id: &str) {
        let player_id = content.settings.combat.player_actor_id.clone();
        self.add_item(item_id);
        self.actor_add_item(&player_id, item_id);
    }
}
