//! Loads the in-repo `layla` pack end-to-end through the content loader and
//! validator. Guards real content regressions: drop tables, equipment slots,
//! and action hooks must keep resolving against the engine.

use cinder_core::content::loader::load_named_pack;
use cinder_core::content::types::DropSpec;

#[test]
fn layla_pack_loads_and_validates() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    assert!(!pack.actors.is_empty());
    assert!(!pack.actions.is_empty());
}

#[test]
fn elf_chess_mobs_declarations_resolve() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");

    assert!(pack.settings.equipment_slots.contains("off-hand"));
    assert!(pack.settings.equipment_slots.contains("cloak"));
    assert!(pack.item("leaf-crook").unwrap().equip_slots.len() == 2);
    assert!(pack.item("leaf-cloak").is_some());

    let pawn = pack.actor("elf-pawn-1").unwrap();
    let DropSpec::Weighted(pool) = &pawn.drops["pawn-kit"] else {
        panic!("pawn kit must parse as a weighted pool");
    };
    assert!(pack.item(&pool.entries[0].item_id).is_some());

    let queen = pack.actor("elf-queen-4").unwrap();
    assert!(matches!(queen.drops["leaf-ring"], DropSpec::Chance(_)));

    let king = pack.actor("elf-king-5").unwrap();
    assert!(matches!(king.drops["drain-scroll"], DropSpec::Always(1)));
    assert!(matches!(king.drops["leaf-cloak"], DropSpec::Chance(_)));
}