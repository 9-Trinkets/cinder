//! Integration tests for `aera` content pack loading.

use cinder_core::content::loader::load_named_pack;

#[test]
fn aera_pack_loads_and_validates() {
    let pack =
        load_named_pack("aera", Some("en")).unwrap_or_else(|e| panic!("aera failed to load: {e}"));
    assert!(!pack.rooms.is_empty());
    assert_eq!(pack.maps.len(), 4);
    assert_eq!(
        pack.map_for_room("lounge")
            .map(|map| (map.id.as_str(), map.rooms.len())),
        Some(("ground-floor", 5))
    );
    assert_eq!(
        pack.map_for_room("studio")
            .map(|map| (map.id.as_str(), map.rooms.len())),
        Some(("second-floor", 2))
    );
    assert_eq!(
        pack.map_for_room("rooftop")
            .map(|map| (map.id.as_str(), map.rooms.len())),
        Some(("third-floor", 2))
    );
}
