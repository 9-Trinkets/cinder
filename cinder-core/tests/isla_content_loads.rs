use cinder_core::content::loader::load_named_pack;

#[test]
fn isla_pack_loads_and_validates() {
    let pack = load_named_pack("isla", Some("en"))
        .unwrap_or_else(|e| panic!("isla pack failed to load: {e}"));
    assert!(!pack.rooms.is_empty(), "isla: rooms absent");
    assert!(!pack.actors.is_empty(), "isla: actors absent");
}
