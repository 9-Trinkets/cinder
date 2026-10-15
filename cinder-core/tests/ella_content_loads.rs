use cinder_core::content::loader::load_named_pack;

#[test]
fn ella_pack_loads_and_validates() {
    let pack = load_named_pack("ella", Some("en"))
        .unwrap_or_else(|e| panic!("ella pack failed to load: {e}"));
    assert!(pack.skills.strict, "ella must opt into strict skills");
    assert!(!pack.rooms.is_empty(), "ella: rooms absent");
    assert!(!pack.actors.is_empty(), "ella: actors absent");
}
