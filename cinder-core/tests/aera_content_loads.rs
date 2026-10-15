//! Integration tests for `aera` content pack loading.

use cinder_core::content::loader::load_named_pack;

#[test]
fn aera_pack_loads_and_validates() {
    let pack =
        load_named_pack("aera", Some("en")).unwrap_or_else(|e| panic!("aera failed to load: {e}"));
    assert!(!pack.actors.is_empty());
    assert!(!pack.rooms.is_empty());
    assert_eq!(pack.maps.len(), 1);
    assert_eq!(
        pack.map_for_room("lounge")
            .map(|map| (map.id.as_str(), map.rooms.len())),
        Some(("sharehouse", 8))
    );
}
