use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// What an item is for. Drives UI grouping; behavior itself stays in the
/// action/effect/hook layer so packs can repurpose any kind.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    #[default]
    Misc,
    Weapon,
    Armor,
    Trinket,
    Potion,
    Key,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ItemDefinition {
    pub id: String,
    pub label: String,
    pub description: String,
    #[serde(default)]
    pub kind: ItemKind,
    /// Equipment slot this item occupies when equipped (e.g. "weapon"). Must
    /// be declared in `settings.equipment_slots`; empty means not equippable.
    #[serde(default)]
    pub equip_slot: String,
    /// Stat id → bonus applied while this item is equipped. Keys must
    /// reference stats declared by the pack.
    #[serde(default)]
    pub stat_bonuses: BTreeMap<String, i32>,
    /// Hook fired once per use when a `UseItem` action targets this item.
    /// Rules typically adjust actor stats (potions healing hp/mp).
    #[serde(default)]
    pub use_hook: String,
    /// Hook fired when this item is equipped (once per equip). Rules can make
    /// lasting world changes, e.g. converting surviving tagged actors into
    /// followers.
    #[serde(default)]
    pub equip_hook: String,
}

impl ItemDefinition {
    pub fn is_equippable(&self) -> bool {
        !self.equip_slot.is_empty()
    }
}
