use super::common::*;
use cinder_core::content::types::{ItemDefinition, ItemKind};
use cinder_core::engine::events::{TimestampedWorldEvent, WorldEvent};
use cinder_core::engine::reducer::apply_events;
use cinder_core::engine::state::{ActorStance, WorldState};
use std::collections::BTreeMap;

fn party_item_pack() -> cinder_core::content::types::ContentPack {
    let mut pack = reducer_test_pack();
    pack.settings.equipment_slots = std::collections::BTreeSet::from([
        "weapon".to_string(),
        "off-hand".to_string(),
        "ring".to_string(),
    ]);
    pack.items.extend([
        ItemDefinition {
            id: "herb".to_string(),
            label: "healing herb".to_string(),
            kind: ItemKind::Potion,
            ..ItemDefinition::default()
        },
        ItemDefinition {
            id: "iron-sword".to_string(),
            label: "iron sword".to_string(),
            kind: ItemKind::Weapon,
            equip_slots: vec!["weapon".to_string()],
            stat_bonuses: BTreeMap::from([("strength".to_string(), 3)]),
            ..ItemDefinition::default()
        },
        ItemDefinition {
            id: "bronze-dagger".to_string(),
            label: "bronze dagger".to_string(),
            kind: ItemKind::Weapon,
            equip_slots: vec!["weapon".to_string()],
            stat_bonuses: BTreeMap::from([("strength".to_string(), 1)]),
            ..ItemDefinition::default()
        },
        ItemDefinition {
            id: "wood-shield".to_string(),
            label: "wooden shield".to_string(),
            kind: ItemKind::Armor,
            equip_slots: vec!["off-hand".to_string()],
            stat_bonuses: BTreeMap::from([("stamina".to_string(), 2)]),
            ..ItemDefinition::default()
        },
        ItemDefinition {
            id: "greatbow".to_string(),
            label: "greatbow".to_string(),
            kind: ItemKind::Weapon,
            equip_slots: vec!["weapon".to_string(), "off-hand".to_string()],
            stat_bonuses: BTreeMap::from([("strength".to_string(), 5)]),
            ..ItemDefinition::default()
        },
    ]);
    pack
}

fn party_state_with_follower(pack: &cinder_core::content::types::ContentPack) -> WorldState {
    let mut state = WorldState::new(pack);
    state.set_stance(ACTOR_A_ID, ActorStance::Allied);
    state.set_follows_player(ACTOR_A_ID, true);
    state
}

#[test]
fn give_item_adds_to_follower_inventory_when_not_equippable() {
    let pack = party_item_pack();
    let mut state = party_state_with_follower(&pack);
    state.add_item("herb");

    let output = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::PlayerGaveItemToPartyMember {
            actor_id: ACTOR_A_ID.to_string(),
            item_id: "herb".to_string(),
        })],
    );

    assert!(!state.has_item("herb"));
    assert_eq!(state.actor_item_count(ACTOR_A_ID, "herb"), 1);
    assert!(output.lines.iter().any(|l| l.text.contains("You give the healing herb to Alex.")));
}

#[test]
fn give_item_auto_equips_and_grants_stat_bonus() {
    let pack = party_item_pack();
    let mut state = party_state_with_follower(&pack);
    state.add_item("iron-sword");

    let base_str = state.effective_actor_stat(&pack, ACTOR_A_ID, "strength");

    let output = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::PlayerGaveItemToPartyMember {
            actor_id: ACTOR_A_ID.to_string(),
            item_id: "iron-sword".to_string(),
        })],
    );

    assert!(!state.has_item("iron-sword"));
    assert_eq!(state.actor_item_count(ACTOR_A_ID, "iron-sword"), 0);
    assert_eq!(
        state.actor_equipped_item(ACTOR_A_ID, "weapon"),
        Some("iron-sword")
    );
    assert_eq!(
        state.effective_actor_stat(&pack, ACTOR_A_ID, "strength"),
        base_str + 3
    );
    assert!(output.lines.iter().any(|l| l.text.contains("You give the iron sword to Alex.")));
    assert!(output.lines.iter().any(|l| l.text.contains("Alex equips the iron sword.")));
}

#[test]
fn give_item_replaces_existing_gear_and_returns_old_item_to_follower_inventory() {
    let pack = party_item_pack();
    let mut state = party_state_with_follower(&pack);
    state.actor_equipment.insert(
        ACTOR_A_ID.to_string(),
        BTreeMap::from([("weapon".to_string(), "bronze-dagger".to_string())]),
    );
    state.add_item("iron-sword");

    apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::PlayerGaveItemToPartyMember {
            actor_id: ACTOR_A_ID.to_string(),
            item_id: "iron-sword".to_string(),
        })],
    );

    assert_eq!(
        state.actor_equipped_item(ACTOR_A_ID, "weapon"),
        Some("iron-sword")
    );
    assert_eq!(state.actor_item_count(ACTOR_A_ID, "bronze-dagger"), 1);
}

#[test]
fn give_two_handed_weapon_replaces_both_weapon_and_offhand() {
    let pack = party_item_pack();
    let mut state = party_state_with_follower(&pack);
    state.actor_equipment.insert(
        ACTOR_A_ID.to_string(),
        BTreeMap::from([
            ("weapon".to_string(), "iron-sword".to_string()),
            ("off-hand".to_string(), "wood-shield".to_string()),
        ]),
    );
    state.add_item("greatbow");

    apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::PlayerGaveItemToPartyMember {
            actor_id: ACTOR_A_ID.to_string(),
            item_id: "greatbow".to_string(),
        })],
    );

    assert_eq!(
        state.actor_equipped_item(ACTOR_A_ID, "weapon"),
        Some("greatbow")
    );
    assert_eq!(
        state.actor_equipped_item(ACTOR_A_ID, "off-hand"),
        Some("greatbow")
    );
    assert_eq!(state.actor_item_count(ACTOR_A_ID, "iron-sword"), 1);
    assert_eq!(state.actor_item_count(ACTOR_A_ID, "wood-shield"), 1);
}

#[test]
fn take_item_from_follower_inventory() {
    let pack = party_item_pack();
    let mut state = party_state_with_follower(&pack);
    state.actor_add_item(ACTOR_A_ID, "herb");

    let output = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::PlayerTookItemFromPartyMember {
            actor_id: ACTOR_A_ID.to_string(),
            item_id: "herb".to_string(),
        })],
    );

    assert_eq!(state.actor_item_count(ACTOR_A_ID, "herb"), 0);
    assert!(state.has_item("herb"));
    assert!(output.lines.iter().any(|l| l.text.contains("You take the healing herb from Alex.")));
}

#[test]
fn take_item_from_follower_equipment_unequips_and_returns_to_player() {
    let pack = party_item_pack();
    let mut state = party_state_with_follower(&pack);
    state.actor_equipment.insert(
        ACTOR_A_ID.to_string(),
        BTreeMap::from([("weapon".to_string(), "iron-sword".to_string())]),
    );

    let base_str = state.effective_actor_stat(&pack, ACTOR_A_ID, "strength");

    let output = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::PlayerTookItemFromPartyMember {
            actor_id: ACTOR_A_ID.to_string(),
            item_id: "iron-sword".to_string(),
        })],
    );

    assert_eq!(state.actor_equipped_item(ACTOR_A_ID, "weapon"), None);
    assert!(state.has_item("iron-sword"));
    assert_eq!(
        state.effective_actor_stat(&pack, ACTOR_A_ID, "strength"),
        base_str - 3
    );
    assert!(output.lines.iter().any(|l| l.text.contains("You take the iron sword from Alex.")));
}
