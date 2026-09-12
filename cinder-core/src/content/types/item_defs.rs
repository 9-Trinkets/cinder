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
    /// Equipment slots this item occupies when equipped (e.g. `["weapon"]`).
    /// One-handed items list a single slot; two-hand weapons list both hands
    /// (e.g. `["weapon", "off-hand"]`). Every slot must be declared in
    /// `settings.equipment_slots`; an empty list means not equippable. The
    /// legacy singular `equip_slot` key is still accepted.
    #[serde(
        default,
        alias = "equip_slot",
        deserialize_with = "deserialize_equip_slots"
    )]
    pub equip_slots: Vec<String>,
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
    /// Prose shown in a room observation when this item lies loose there,
    /// in place of the generic "On the ground: {label}" line. Lets non-object
    /// room marks (e.g. chalk drawings) read as part of the room, not loot.
    #[serde(default)]
    pub look_description: String,
    /// Whether this item is a persistent room mark (e.g. a chalk sigil)
    /// rather than a portable object. Trace marks cannot be picked up via the
    /// generic `take <item>` command and, once created in a room, cannot be
    /// dropped or relocated there by the player.
    #[serde(default)]
    pub trace_mark: bool,
    /// Whether completing a `actor.surrounded` conversion with this item spends
    /// it: the item is removed from the room where it was just placed and that
    /// placement stops converting further actors. Gives charm sigils their
    /// one-shot behavior.
    #[serde(default, skip_serializing_if = "is_false")]
    pub consumed_on_surround_conversion: bool,
}

impl ItemDefinition {
    pub fn is_equippable(&self) -> bool {
        !self.equip_slots.is_empty()
    }

    /// Slots this item occupies while equipped.
    pub fn occupied_slots(&self) -> &[String] {
        &self.equip_slots
    }

    pub fn is_takeable(&self) -> bool {
        !self.trace_mark
    }
}

fn deserialize_equip_slots<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany {
        One(String),
        Many(Vec<String>),
    }
    Ok(match OneOrMany::deserialize(deserializer)? {
        OneOrMany::One(slot) => vec![slot],
        OneOrMany::Many(slots) => slots,
    })
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[cfg(test)]
mod tests {
    use super::ItemDefinition;

    #[test]
    fn legacy_singular_equip_slot_key_still_deserializes() {
        let item: ItemDefinition =
            serde_json::from_str(r#"{"id":"x","label":"x","description":"","equip_slot":"weapon"}"#)
                .unwrap();
        assert_eq!(item.equip_slots, vec!["weapon".to_string()]);
    }

    #[test]
    fn two_hand_weapons_use_the_list_form() {
        let item: ItemDefinition = serde_json::from_str(
            r#"{"id":"x","label":"x","description":"","equip_slots":["weapon","off-hand"]}"#,
        )
        .unwrap();
        assert_eq!(
            item.occupied_slots(),
            &["weapon".to_string(), "off-hand".to_string()]
        );
    }

    #[test]
    fn empty_equipment_is_not_equippable() {
        let item: ItemDefinition =
            serde_json::from_str(r#"{"id":"x","label":"x","description":""}"#).unwrap();
        assert!(!item.is_equippable());
    }
}
