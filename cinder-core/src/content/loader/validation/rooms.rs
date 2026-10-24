use super::{ContentValidator, Diagnostics};
use crate::content::types::ContentPack;

/// Checks that every item a room or a feature starts with exists in `items.json`.
pub(super) struct RoomItemsValidator;

impl ContentValidator for RoomItemsValidator {
    fn validate(&self, pack: &ContentPack, diagnostics: &mut Diagnostics) {
        for room in &pack.rooms {
            for item_id in room.items.keys() {
                if pack.item(item_id).is_none() {
                    diagnostics.push(format!(
                        "room '{}' starts with unknown item '{item_id}'",
                        room.id
                    ));
                }
            }
            for feature in &room.features {
                for item_id in feature.items.keys() {
                    if pack.item(item_id).is_none() {
                        diagnostics.push(format!(
                            "feature '{}' in room '{}' holds unknown item '{item_id}'",
                            feature.id, room.id
                        ));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::types::{RoomDefinition, RoomFeatureDefinition};
    use crate::engine::test_fixtures::minimal_test_pack;

    fn room_with(items: &[&str], feature_items: &[&str]) -> RoomDefinition {
        RoomDefinition {
            id: "den".to_string(),
            title: "Den".to_string(),
            summary: String::new(),
            inspect_text: String::new(),
            allow_rest: false,
            features: vec![RoomFeatureDefinition {
                id: "shelf".to_string(),
                label: "shelf".to_string(),
                aliases: Vec::new(),
                allow_rest: false,
                items: feature_items.iter().map(|id| (id.to_string(), 1)).collect(),
                items_text: String::new(),
                inspect_text: String::new(),
            }],
            exits: Vec::new(),
            descriptions: Vec::new(),
            items: items.iter().map(|id| (id.to_string(), 1)).collect(),
        }
    }

    #[test]
    fn an_unknown_item_on_a_feature_or_in_a_room_is_reported() {
        let mut pack = minimal_test_pack();
        pack.rooms.push(room_with(&["ghost"], &["phantom"]));
        let mut diagnostics = Diagnostics::default();
        RoomItemsValidator.validate(&pack, &mut diagnostics);
        let message = diagnostics.finish().expect_err("two errors").to_string();
        assert!(message.contains("unknown item 'ghost'"), "{message}");
        assert!(message.contains("unknown item 'phantom'"), "{message}");
    }

    #[test]
    fn known_items_pass() {
        let pack = minimal_test_pack();
        let mut diagnostics = Diagnostics::default();
        RoomItemsValidator.validate(&pack, &mut diagnostics);
        assert!(diagnostics.finish().is_ok());
    }
}
