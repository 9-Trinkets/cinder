//! Integration tests for handing an item to an ally.
//!
//! Giving an equippable item to a follower equips it for them; giving anything
//! else drops it in their pack. The split is driven entirely by
//! `ItemDefinition::equip_slots`, so an item authored as a weapon but left
//! without slots silently falls through to the pack and reads as a bug.

use cinder_core::content::loader::load_named_pack;
use cinder_core::content::types::ItemKind;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::{ActorStance, WorldState};

/// A Floor 4 garrison ally that carries bolts of its own.
const ALLY_ID: &str = "garrison_sentry_gate";

fn runtime_with_bolt_in_room() -> (CinderRuntime, cinder_core::content::types::ContentPack) {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    // Put Layla and the ally together so the gift resolves.
    state.current_room_id = "fortress_gate".to_string();
    state
        .actor_room_overrides
        .insert(ALLY_ID.to_string(), "fortress_gate".to_string());
    // The garrison starts neutral; gifts only resolve for party members.
    state.set_stance(ALLY_ID, ActorStance::Allied);
    state.add_item("steam-crossbow-bolt");
    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");
    (runtime, pack)
}

#[test]
fn steam_crossbow_bolt_is_authored_as_an_equippable_weapon() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let bolt = pack
        .items
        .iter()
        .find(|item| item.id == "steam-crossbow-bolt")
        .expect("the bolt must exist");

    assert_eq!(bolt.kind, ItemKind::Weapon);
    assert!(
        bolt.is_equippable(),
        "the bolt occupies the weapon slot like every other weapon"
    );
    assert_eq!(
        bolt.occupied_slots(),
        ["weapon".to_string()],
        "the bolt must claim the weapon slot"
    );
    assert_eq!(
        bolt.stat_bonuses.get("attack"),
        Some(&2),
        "garrison issue steel should outpace a conscript's leaf spear (1) \
         without matching an officer's brass saber (3)"
    );
}

#[test]
fn giving_the_bolt_to_an_ally_auto_equips_it() {
    let (runtime, _pack) = runtime_with_bolt_in_room();

    let outcome = runtime
        .run_turn("give steam crossbow bolt to garrison sentry")
        .expect("giving the bolt succeeds");

    let after = runtime.export_state().expect("state exported");
    assert_eq!(
        after.actor_equipped_item(ALLY_ID, "weapon"),
        Some("steam-crossbow-bolt"),
        "the ally should be holding the bolt, not carrying it loosely"
    );
    assert!(
        outcome.text().contains("party.give_auto_equipped")
            || outcome.lines.iter().any(|l| l.text.contains("equip")),
        "the give should narrate the auto-equip: {:?}",
        outcome.lines
    );
}

#[test]
fn giving_a_non_equippable_item_still_lands_in_the_pack() {
    // The contrast case: keys carry no slots, so they must not equip.
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "fortress_gate".to_string();
    state
        .actor_room_overrides
        .insert(ALLY_ID.to_string(), "fortress_gate".to_string());
    state.set_stance(ALLY_ID, ActorStance::Allied);
    state.add_item("courtyard-cage-key");

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");
    runtime
        .run_turn("give courtyard cage key to garrison sentry")
        .expect("giving the key succeeds");

    let after = runtime.export_state().expect("state exported");
    assert_eq!(
        after.actor_equipped_item(ALLY_ID, "weapon"),
        None,
        "a key must never occupy the weapon slot"
    );
    assert!(
        after.actor_has_item(ALLY_ID, "courtyard-cage-key"),
        "the key belongs in the ally's pack"
    );
}
