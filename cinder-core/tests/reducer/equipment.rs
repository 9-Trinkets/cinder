use super::common::*;
use cinder_core::content::types::{ActionDefinition, CommandEffect, CommandTargetMode};
use cinder_core::engine::state::{ActorStance, WorldState};
use serde_json::json;
use std::collections::BTreeMap;

#[test]
fn equipping_an_item_with_equip_hook_converts_surviving_tagged_actors() {
    let mut pack = reducer_test_pack();
    pack.settings.combat = cinder_core::content::types::CombatSettingsDefinition {
        player_actor_id: ACTOR_A_ID.to_string(),
        health_stat_id: "stamina".to_string(),
        attack_stat_id: "confidence".to_string(),
        defense_stat_id: "hunger".to_string(),
        ..cinder_core::content::types::CombatSettingsDefinition::default()
    };
    pack.settings.equipment_slots = ["trinket".to_string()].into_iter().collect();
    pack.items
        .push(cinder_core::content::types::ItemDefinition {
            id: "warden-core".to_string(),
            label: "warden core".to_string(),
            description: "A warm stone heart.".to_string(),
            kind: cinder_core::content::types::ItemKind::Trinket,
            equip_slot: "trinket".to_string(),
            stat_bonuses: BTreeMap::new(),
            use_hook: String::new(),
            equip_hook: "item.core_equipped".to_string(),
            look_description: String::new(),
        });
    pack.messages.insert(
        "conversion.core".to_string(),
        "The {actor} bows its head and falls in behind you.".to_string(),
    );
    pack.hooks.insert(
        "item.core_equipped".to_string(),
        effect_hook(vec![json!({
            "kind": "convert_allies_by_tag",
            "tag": "golem",
            "follows_player": true,
            "messages": ["conversion.core"],
        })]),
    );
    pack.actions.push(ActionDefinition {
        id: "equip-core".to_string(),
        command: "equip-core".to_string(),
        target_mode: CommandTargetMode::None,
        effects: vec![CommandEffect::EquipItem],
        item_id: "warden-core".to_string(),
        event_text: "{actor_name} grasps the warden core.".to_string(),
        ..ActionDefinition::default()
    });
    // Two living golems, one already allied (encircled earlier), one dead.
    let mut golem_living = test_actor("statue-live", "granite statue", KITCHEN_ID);
    golem_living.tags = vec!["golem".to_string()];
    golem_living.initial_stats = BTreeMap::from([("stamina".to_string(), 8)]);
    let mut golem_dead = test_actor("statue-dead", "dust statue", KITCHEN_ID);
    golem_dead.tags = vec!["golem".to_string()];
    golem_dead.initial_stats = BTreeMap::from([("stamina".to_string(), 0)]);
    let mut golem_allied = test_actor("statue-ally", "charmed statue", KITCHEN_ID);
    golem_allied.tags = vec!["golem".to_string()];
    golem_allied.initial_stats = BTreeMap::from([("stamina".to_string(), 8)]);
    pack.actors.extend([golem_living, golem_dead, golem_allied]);
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    state.set_stance("statue-ally", ActorStance::Allied);
    state.add_item("warden-core");

    let lines = drive_actor_command(
        &mut state,
        &pack,
        ACTOR_A_ID,
        ACTOR_A_NAME,
        LOUNGE_ID,
        "equip-core",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .lines;

    // Living golem converted to an ally follower; dead and already-allied skipped.
    assert_eq!(state.stance("statue-live"), ActorStance::Allied);
    assert!(state.relationship("statue-live").follows_player);
    assert_eq!(state.stance("statue-dead"), ActorStance::Neutral);
    assert_eq!(state.stance("statue-ally"), ActorStance::Allied);
    // Already-allied actors are left untouched (not re-followed by the hook).
    assert!(!state.relationship("statue-ally").follows_player);
    assert!(
        lines
            .iter()
            .any(|line| line.text.contains("granite statue")),
        "got: {lines:?}"
    );
    assert!(
        !lines.iter().any(|line| line.text.contains("dust statue")),
        "got: {lines:?}"
    );
}

#[test]
fn equip_and_unequip_change_effective_stats_and_inventory() {
    let pack = equipment_test_pack();
    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    // Alex's confidence base.
    let base_attack = state.actor_stat(ACTOR_A_ID, "confidence");
    state.add_item("iron-chisel");

    drive_actor_command(
        &mut state,
        &pack,
        ACTOR_A_ID,
        ACTOR_A_NAME,
        LOUNGE_ID,
        "equip-chisel",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    );

    assert!(!state.has_item("iron-chisel"));
    assert_eq!(state.equipped_item("weapon"), Some("iron-chisel"));
    assert_eq!(
        state.effective_actor_stat(&pack, ACTOR_A_ID, "confidence"),
        base_attack + 2
    );
    // Non-player actors never receive equipment bonuses.
    assert_eq!(
        state.effective_actor_stat(&pack, ACTOR_B_ID, "confidence"),
        state.actor_stat(ACTOR_B_ID, "confidence")
    );

    drive_actor_command(
        &mut state,
        &pack,
        ACTOR_A_ID,
        ACTOR_A_NAME,
        LOUNGE_ID,
        "unequip-chisel",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    );

    assert!(state.has_item("iron-chisel"));
    assert_eq!(state.equipped_item("weapon"), None);
    assert_eq!(
        state.effective_actor_stat(&pack, ACTOR_A_ID, "confidence"),
        base_attack
    );
}

#[test]
fn equipping_a_second_weapon_replaces_the_first() {
    let mut pack = equipment_test_pack();
    pack.actions.push(ActionDefinition {
        id: "equip-steel".to_string(),
        command: "equip-steel".to_string(),
        target_mode: CommandTargetMode::None,
        effects: vec![CommandEffect::EquipItem],
        item_id: "steel-chisel".to_string(),
        event_text: "{actor_name} readies the steel chisel.".to_string(),
        ..ActionDefinition::default()
    });
    rebuild_test_pack_indexes(&mut pack);
    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    state.add_item("iron-chisel");
    state.add_item("steel-chisel");
    let base_attack = state.actor_stat(ACTOR_A_ID, "confidence");

    drive_actor_command(
        &mut state,
        &pack,
        ACTOR_A_ID,
        ACTOR_A_NAME,
        LOUNGE_ID,
        "equip-chisel",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    );
    drive_actor_command(
        &mut state,
        &pack,
        ACTOR_A_ID,
        ACTOR_A_NAME,
        LOUNGE_ID,
        "equip-steel",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    );

    assert_eq!(state.equipped_item("weapon"), Some("steel-chisel"));
    assert!(
        state.has_item("iron-chisel"),
        "old weapon returns to inventory"
    );
    assert!(!state.has_item("steel-chisel"));
    assert_eq!(
        state.effective_actor_stat(&pack, ACTOR_A_ID, "confidence"),
        base_attack + 4
    );
}

#[test]
fn cannot_equip_an_item_already_in_its_slot_even_with_spares() {
    let pack = equipment_test_pack();
    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    // Two copies: one equipped, one spare in the inventory.
    state.add_item("iron-chisel");
    state.add_item("iron-chisel");

    drive_actor_command(
        &mut state,
        &pack,
        ACTOR_A_ID,
        ACTOR_A_NAME,
        LOUNGE_ID,
        "equip-chisel",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    );
    assert_eq!(state.equipped_item("weapon"), Some("iron-chisel"));
    assert_eq!(state.item_count("iron-chisel"), 1);

    // Equipping the same item again is rejected: it already fills the slot.
    let second = drive_actor_command(
        &mut state,
        &pack,
        ACTOR_A_ID,
        ACTOR_A_NAME,
        LOUNGE_ID,
        "equip-chisel",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    );
    assert!(
        second.lines.is_empty(),
        "equipping an already-equipped item must be rejected"
    );
    assert_eq!(state.equipped_item("weapon"), Some("iron-chisel"));
    assert_eq!(state.item_count("iron-chisel"), 1);
}

#[test]
fn using_a_potion_consumes_it_and_fires_its_use_hook() {
    let pack = equipment_test_pack();
    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    state.add_item("herb-salve");
    state.add_item("herb-salve");
    let stamina_before = state.actor_stat(ACTOR_A_ID, "stamina");

    drive_actor_command(
        &mut state,
        &pack,
        ACTOR_A_ID,
        ACTOR_A_NAME,
        LOUNGE_ID,
        "use-salve",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    );

    assert_eq!(state.actor_stat(ACTOR_A_ID, "stamina"), stamina_before + 2);
    assert_eq!(state.item_count("herb-salve"), 1);
}
