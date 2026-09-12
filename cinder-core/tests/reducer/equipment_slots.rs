//! Multi-slot equipment: a two-hand weapon occupies both hands, swapping frees
//! whole occupied sets, and unequip clears every occupied slot back to the
//! inventory.

use super::common::*;
use cinder_core::content::types::{
    ActionDefinition, CommandEffect, CommandTargetMode, ContentPack, ItemDefinition, ItemKind,
};
use cinder_core::engine::state::WorldState;
use std::collections::{BTreeMap, BTreeSet};

fn gear_pack() -> ContentPack {
    let mut pack = reducer_test_pack();
    pack.settings.combat = cinder_core::content::types::CombatSettingsDefinition {
        player_actor_id: ACTOR_A_ID.to_string(),
        health_stat_id: "stamina".to_string(),
        attack_stat_id: "confidence".to_string(),
        defense_stat_id: "hunger".to_string(),
        ..cinder_core::content::types::CombatSettingsDefinition::default()
    };
    pack.settings.equipment_slots = ["weapon".to_string(), "off-hand".to_string(), "gloves".to_string()]
        .into_iter()
        .collect();
    pack.items.push(ItemDefinition {
        id: "greatbow".to_string(),
        label: "iron-elm greatbow".to_string(),
        description: "Takes both hands to draw.".to_string(),
        kind: ItemKind::Weapon,
        equip_slots: vec!["weapon".to_string(), "off-hand".to_string()],
        stat_bonuses: BTreeMap::from([("confidence".to_string(), 4)]),
        ..ItemDefinition::default()
    });
    pack.items.push(ItemDefinition {
        id: "dagger".to_string(),
        label: "antler dagger".to_string(),
        description: "Fits one hand.".to_string(),
        kind: ItemKind::Weapon,
        equip_slots: vec!["weapon".to_string()],
        stat_bonuses: BTreeMap::from([("confidence".to_string(), 1)]),
        ..ItemDefinition::default()
    });
    pack.items.push(ItemDefinition {
        id: "loose-greave".to_string(),
        label: "loose greave".to_string(),
        description: "Free-clutched in the off hand.".to_string(),
        kind: ItemKind::Armor,
        equip_slots: vec!["off-hand".to_string()],
        stat_bonuses: BTreeMap::from([("hunger".to_string(), 1)]),
        ..ItemDefinition::default()
    });
    pack.items.push(ItemDefinition {
        id: "woven-gloves".to_string(),
        label: "woven gloves".to_string(),
        description: "Layered leaves.".to_string(),
        kind: ItemKind::Armor,
        equip_slots: vec!["gloves".to_string()],
        stat_bonuses: BTreeMap::new(),
        ..ItemDefinition::default()
    });
    for (id, usage, item_id, equip) in [
        ("equip-greatbow", "equip greatbow", "greatbow", true),
        ("unequip-greatbow", "stow greatbow", "greatbow", false),
        ("equip-dagger", "equip dagger", "dagger", true),
        ("unequip-dagger", "stow dagger", "dagger", false),
        ("equip-greave", "equip greave", "loose-greave", true),
        ("unequip-greave", "stow greave", "loose-greave", false),
        ("equip-gloves", "equip gloves", "woven-gloves", true),
        ("unequip-gloves", "stow gloves", "woven-gloves", false),
    ] {
        pack.actions.push(ActionDefinition {
            id: id.to_string(),
            command: usage.to_uppercase(),
            label: usage.to_string(),
            target_mode: CommandTargetMode::None,
            effects: vec![if equip {
                CommandEffect::EquipItem
            } else {
                CommandEffect::UnequipItem
            }],
            item_id: item_id.to_string(),
            event_text: format!("{ACTOR_A_NAME} fusses with the {item_id}."),
            ..ActionDefinition::default()
        });
    }
    rebuild_test_pack_indexes(&mut pack);
    pack
}

fn fresh_state(pack: &ContentPack) -> WorldState {
    let mut state = WorldState::new(pack);
    state.current_room_id = LOUNGE_ID.to_string();
    state
}

fn drive(pack: &ContentPack, state: &mut WorldState, command_id: &str) {
    drive_actor_command(
        state,
        pack,
        command_id,
        ActorCommandInput {
            actor_id: ACTOR_A_ID,
            actor_name: ACTOR_A_NAME,
            room_id: LOUNGE_ID,
            ..ActorCommandInput::default()
        },
    );
}

fn equipped(pack: &ContentPack, state: &WorldState, item_id: &str) -> bool {
    pack.item(item_id).is_some_and(|item| state.item_is_equipped(item))
}

#[test]
fn two_hand_weapon_occupies_both_slots_and_bonus_applies_once() {
    let pack = gear_pack();
    let mut state = fresh_state(&pack);
    state.add_item("greatbow");

    drive(&pack, &mut state, "equip-greatbow");

    assert_eq!(state.equipment.get("weapon").map(String::as_str), Some("greatbow"));
    assert_eq!(state.equipment.get("off-hand").map(String::as_str), Some("greatbow"));
    assert_eq!(state.equipped_stat_bonus(&pack, "confidence"), 4);
    assert!(!state.has_item("greatbow"));
}

#[test]
fn one_hand_equip_swaps_out_a_two_hand_and_frees_the_off_hand() {
    let pack = gear_pack();
    let mut state = fresh_state(&pack);
    state.add_item("greatbow");
    state.add_item("dagger");
    drive(&pack, &mut state, "equip-greatbow");

    drive(&pack, &mut state, "equip-dagger");

    assert_eq!(state.equipment.get("weapon").map(String::as_str), Some("dagger"));
    assert!(!state.equipment.contains_key("off-hand"));
    assert!(state.has_item("greatbow"), "the freed two-hand weapon returns to inventory");
    assert!(!state.has_item("dagger"));
}

#[test]
fn two_hand_equip_swaps_out_a_one_hand_pair_returning_both() {
    let pack = gear_pack();
    let mut state = fresh_state(&pack);
    state.add_item("greatbow");
    state.add_item("dagger");
    state.add_item("loose-greave");
    drive(&pack, &mut state, "equip-dagger");
    drive(&pack, &mut state, "equip-greave");
    assert_eq!(state.equipment.get("off-hand").map(String::as_str), Some("loose-greave"));

    drive(&pack, &mut state, "equip-greatbow");

    assert_eq!(state.equipment.get("weapon").map(String::as_str), Some("greatbow"));
    assert_eq!(state.equipment.get("off-hand").map(String::as_str), Some("greatbow"));
    assert!(state.has_item("dagger"));
    assert!(state.has_item("loose-greave"));
    assert_eq!(state.equipped_stat_bonus(&pack, "confidence"), 4);
}

#[test]
fn unequip_clears_every_occupied_slot_back_to_inventory() {
    let pack = gear_pack();
    let mut state = fresh_state(&pack);
    state.add_item("greatbow");
    drive(&pack, &mut state, "equip-greatbow");

    drive(&pack, &mut state, "unequip-greatbow");

    assert!(state.equipment.is_empty());
    assert!(state.has_item("greatbow"));
    assert_eq!(state.equipped_stat_bonus(&pack, "confidence"), 0);
}

#[test]
fn equip_is_rejected_when_a_declared_slot_vanishes_from_settings() {
    let mut pack = gear_pack();
    let mut state = fresh_state(&pack);
    state.add_item("woven-gloves");
    pack.settings.equipment_slots = ["weapon".to_string(), "off-hand".to_string()]
        .into_iter()
        .collect::<BTreeSet<String>>();

    drive(&pack, &mut state, "equip-gloves");

    assert!(!equipped(&pack, &state, "woven-gloves"));
    assert!(state.has_item("woven-gloves"), "rejected equip keeps the item held");
}

#[test]
fn already_equipped_item_rejects_a_second_equip() {
    let pack = gear_pack();
    let mut state = fresh_state(&pack);
    state.add_item("greatbow");
    drive(&pack, &mut state, "equip-greatbow");

    drive(&pack, &mut state, "equip-greatbow");

    assert!(equipped(&pack, &state, "greatbow"));
    assert!(!state.has_item("greatbow"));
}