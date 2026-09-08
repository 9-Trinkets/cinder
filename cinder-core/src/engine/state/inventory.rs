use super::*;
use crate::content::types::ItemStorageTarget;

impl WorldState {
    pub fn has_item(&self, item_id: &str) -> bool {
        self.player_inventory.get(item_id).copied().unwrap_or(0) > 0
    }

    pub fn item_count(&self, item_id: &str) -> u32 {
        self.player_inventory.get(item_id).copied().unwrap_or(0)
    }

    pub fn add_item(&mut self, item_id: &str) {
        *self
            .player_inventory
            .entry(item_id.to_string())
            .or_insert(0) += 1;
    }

    pub fn remove_item(&mut self, item_id: &str) -> bool {
        let mut has = false;
        if let Some(count) = self.player_inventory.get_mut(item_id)
            && *count > 0
        {
            *count -= 1;
            has = true;
        }
        if let Some(0) = self.player_inventory.get(item_id) {
            self.player_inventory.remove(item_id);
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
            ItemStorageTarget::PlayerInventory => self.add_item(item_id),
            ItemStorageTarget::CurrentRoom => {
                *self
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
            ItemStorageTarget::PlayerInventory => self.remove_item(item_id),
            ItemStorageTarget::CurrentRoom => {
                let key = room_item_key(current_room_id, item_id);
                let mut removed = false;
                if let Some(count) = self.room_item_stock.get_mut(&key)
                    && *count > 0
                {
                    *count -= 1;
                    removed = true;
                }
                if let Some(0) = self.room_item_stock.get(&key) {
                    self.room_item_stock.remove(&key);
                    self.room_item_charges.remove(&key);
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
        let had = self.room_item_stock.remove(&key).is_some();
        self.room_item_charges.remove(&key);
        had
    }

    /// Loose items lying in `room_id` (from `ItemStorageTarget::CurrentRoom`),
    /// as `(item_id, count)`.
    pub fn loose_room_items(&self, room_id: &str) -> Vec<(String, u32)> {
        let prefix = format!("{room_id}::");
        self.room_item_stock
            .iter()
            .filter(|(key, _)| key.starts_with(&prefix))
            .map(|(key, count)| (key[prefix.len()..].to_string(), *count))
            .collect()
    }

    pub fn remaining_consumable_stock(
        &self,
        room_id: &str,
        feature_id: &str,
        consumable_id: &str,
    ) -> u32 {
        self.feature_consumable_stock
            .get(&consumable_key(room_id, feature_id, consumable_id))
            .copied()
            .unwrap_or(0)
    }

    pub fn consume_feature_consumable(
        &mut self,
        room_id: &str,
        feature_id: &str,
        consumable_id: &str,
    ) -> bool {
        let key = consumable_key(room_id, feature_id, consumable_id);
        let Some(stock) = self.feature_consumable_stock.get_mut(&key) else {
            return false;
        };
        if *stock == 0 {
            return false;
        }
        *stock -= 1;
        true
    }

    pub fn restock_feature_consumable(
        &mut self,
        room_id: &str,
        feature_id: &str,
        consumable_id: &str,
        amount: u32,
    ) -> bool {
        let key = consumable_key(room_id, feature_id, consumable_id);
        let entry = self.feature_consumable_stock.entry(key).or_insert(0);
        *entry += amount;
        true
    }
}
