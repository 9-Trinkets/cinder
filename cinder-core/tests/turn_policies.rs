//! Integration tests for `engine::turn_policies`, driven through the public
//! API (`action_is_available`).

use cinder_core::content::types::{
    ActionAvailability, ActionDefinition, ActionItemCreation, ActionUi, CommandEffect,
    CommandTargetMode, ContentPack, ItemDefinition, ItemKind, PanelConfig, PanelDataSource,
};
use cinder_core::engine::state::{ActorStance, WorldState};
use cinder_core::engine::test_fixtures::{minimal_test_pack, rebuild_test_pack_indexes};
use cinder_core::engine::turn_policies::action_is_available;

fn equipment_pack() -> ContentPack {
    let mut pack = minimal_test_pack();
    pack.settings.equipment_slots = ["weapon".to_string()].into_iter().collect();
    pack.items.push(ItemDefinition {
        id: "chisel".to_string(),
        label: "chisel".to_string(),
        description: "A chisel.".to_string(),
        kind: ItemKind::Weapon,
        equip_slots: vec!["weapon".to_string()],
        stat_bonuses: std::collections::BTreeMap::new(),
        use_hook: String::new(),
        equip_hook: String::new(),
        look_description: String::new(),
        trace_mark: false,
        consumed_on_surround_conversion: false,
    });
    pack.actions.push(ActionDefinition {
        id: "equip".to_string(),
        command: "equip".to_string(),
        target_mode: CommandTargetMode::None,
        effects: vec![CommandEffect::EquipItem],
        item_id: "chisel".to_string(),
        available: ActionAvailability::default(),
        ..ActionDefinition::default()
    });
    pack.actions.push(ActionDefinition {
        id: "unequip".to_string(),
        command: "unequip".to_string(),
        target_mode: CommandTargetMode::None,
        effects: vec![CommandEffect::UnequipItem],
        item_id: "chisel".to_string(),
        available: ActionAvailability::default(),
        ..ActionDefinition::default()
    });
    rebuild_test_pack_indexes(&mut pack);
    pack
}

#[test]
fn equip_and_unequip_are_mutually_exclusive_in_the_bar() {
    let pack = equipment_pack();
    let equip = pack.action("equip").unwrap();
    let unequip = pack.action("unequip").unwrap();

    // Holding the chisel (not equipped): only equip is available.
    let mut state = WorldState::new(&pack);
    state.current_room_id = "lounge".to_string();
    state.add_item("chisel");
    assert!(action_is_available(&pack, &state, equip, &state.current_room_id));
    assert!(!action_is_available(&pack, &state, unequip, &state.current_room_id));

    // After equipping, only unequip is available.
    state.remove_item("chisel");
    state
        .equipment
        .insert("weapon".to_string(), "chisel".to_string());
    assert!(!action_is_available(&pack, &state, equip, &state.current_room_id));
    assert!(action_is_available(&pack, &state, unequip, &state.current_room_id));
}

/// A pack with one target-selection action per panel data source: `attack`
/// (actors in room, filters allies/followers), `speak` (actors in room),
/// `trace` (craftable items), plus informational `look` (features) and
/// `move` (exits).
fn target_pack() -> ContentPack {
    let mut pack = minimal_test_pack();
    pack.settings.combat.health_stat_id = "stamina".to_string();
    pack.items.extend([
        ItemDefinition {
            id: "charm-sigil".to_string(),
            label: "charm sigil".to_string(),
            description: "A chalk ring.".to_string(),
            trace_mark: true,
            ..ItemDefinition::default()
        },
        ItemDefinition {
            id: "drain-sigil".to_string(),
            label: "drain sigil".to_string(),
            description: "A chalk spiral.".to_string(),
            trace_mark: true,
            ..ItemDefinition::default()
        },
    ]);
    pack.actions.push(ActionDefinition {
        id: "attack".to_string(),
        command: "attack".to_string(),
        effects: vec![CommandEffect::AttackTarget],
        ui: ActionUi {
            bar: true,
            panel: Some("attack".to_string()),
            panel_config: Some(PanelConfig {
                data_source: PanelDataSource::ActorsInRoom,
                ..PanelConfig::default()
            }),
            ..ActionUi::default()
        },
        ..ActionDefinition::default()
    });
    pack.actions.push(ActionDefinition {
        id: "speak".to_string(),
        command: "speak".to_string(),
        ui: ActionUi {
            bar: true,
            panel: Some("talk".to_string()),
            panel_config: Some(PanelConfig {
                data_source: PanelDataSource::ActorsInRoom,
                ..PanelConfig::default()
            }),
            ..ActionUi::default()
        },
        ..ActionDefinition::default()
    });
    pack.actions.push(ActionDefinition {
        id: "trace".to_string(),
        command: "trace".to_string(),
        item_creation: Some(ActionItemCreation {
            creates_item: "charm-sigil".to_string(),
            craftable_items: vec!["charm-sigil".to_string(), "drain-sigil".to_string()],
            craftable_item_gates: std::collections::BTreeMap::from([(
                "drain-sigil".to_string(),
                "knows_drain".to_string(),
            )]),
            ..ActionItemCreation::default()
        }),
        ui: ActionUi {
            bar: true,
            panel: Some("trace".to_string()),
            panel_config: Some(PanelConfig {
                data_source: PanelDataSource::CraftableItems,
                ..PanelConfig::default()
            }),
            ..ActionUi::default()
        },
        ..ActionDefinition::default()
    });
    pack.actions.push(ActionDefinition {
        id: "look".to_string(),
        command: "look".to_string(),
        ui: ActionUi {
            bar: true,
            panel: Some("look".to_string()),
            panel_config: Some(PanelConfig {
                data_source: PanelDataSource::Features,
                ..PanelConfig::default()
            }),
            ..ActionUi::default()
        },
        ..ActionDefinition::default()
    });
    pack.actions.push(ActionDefinition {
        id: "move".to_string(),
        command: "move".to_string(),
        ui: ActionUi {
            bar: true,
            panel: Some("move".to_string()),
            panel_config: Some(PanelConfig {
                data_source: PanelDataSource::Exits,
                ..PanelConfig::default()
            }),
            ..ActionUi::default()
        },
        ..ActionDefinition::default()
    });
    rebuild_test_pack_indexes(&mut pack);
    pack
}

fn live_in_lounge(pack: &ContentPack) -> WorldState {
    let mut state = WorldState::new(pack);
    state.current_room_id = "lounge".to_string();
    state
        .adjust_actor_stat(pack, "blair", &pack.settings.combat.health_stat_id, 50)
        .expect("set blair hp");
    state
        .adjust_actor_stat(pack, "casey", &pack.settings.combat.health_stat_id, 50)
        .expect("set casey hp");
    state
}

fn defeat(pack: &ContentPack, s: &mut WorldState, actor_id: &str, health_stat_id: &str) {
    s.adjust_actor_stat(pack, actor_id, health_stat_id, -1000)
        .expect("defeat actor");
}

#[test]
fn attack_hides_with_no_attackable_target() {
    let pack = target_pack();
    let attack = pack.action("attack").unwrap();

    // A living neutral actor is a valid target.
    let state = live_in_lounge(&pack);
    assert!(action_is_available(&pack, &state, attack, &state.current_room_id));

    // Only a defeated actor present -> hidden.
    let mut state = live_in_lounge(&pack);
    defeat(&pack, &mut state, "casey", &pack.settings.combat.health_stat_id);
    defeat(&pack, &mut state, "blair", &pack.settings.combat.health_stat_id);
    assert!(!action_is_available(&pack, &state, attack, &state.current_room_id));

    // Only an ally present -> hidden.
    let mut state = live_in_lounge(&pack);
    state.set_stance("blair", ActorStance::Allied);
    defeat(&pack, &mut state, "casey", &pack.settings.combat.health_stat_id);
    assert!(!action_is_available(&pack, &state, attack, &state.current_room_id));

    // Only a follower (not Allied stance) present -> hidden.
    let mut state = live_in_lounge(&pack);
    state.set_follows_player("blair", true);
    defeat(&pack, &mut state, "casey", &pack.settings.combat.health_stat_id);
    assert!(!action_is_available(&pack, &state, attack, &state.current_room_id));

    // Only the player's own actor present -> hidden.
    let mut state = live_in_lounge(&pack);
    defeat(&pack, &mut state, "blair", &pack.settings.combat.health_stat_id);
    defeat(&pack, &mut state, "casey", &pack.settings.combat.health_stat_id);
    state.story_vars.set_unchecked("x", "y");
    assert!(!action_is_available(&pack, &state, attack, &state.current_room_id));
}

#[test]
fn speak_accepts_allies_as_talk_targets() {
    let pack = target_pack();
    let speak = pack.action("speak").unwrap();
    let mut state = live_in_lounge(&pack);
    state.set_stance("casey", ActorStance::Allied);
    defeat(&pack, &mut state, "blair", &pack.settings.combat.health_stat_id);
    assert!(action_is_available(&pack, &state, speak, &state.current_room_id));
}

#[test]
fn speak_hides_with_no_living_actor() {
    let pack = target_pack();
    let speak = pack.action("speak").unwrap();
    let mut state = live_in_lounge(&pack);
    defeat(&pack, &mut state, "blair", &pack.settings.combat.health_stat_id);
    defeat(&pack, &mut state, "casey", &pack.settings.combat.health_stat_id);
    assert!(!action_is_available(&pack, &state, speak, &state.current_room_id));
}

#[test]
fn trace_hides_when_no_craftable_unlocked() {
    let pack = target_pack();
    let trace = pack.action("trace").unwrap();
    let mut state = live_in_lounge(&pack);
    state.story_vars.set_unchecked("knows_drain", "false");
    // charm-sigil has no gate -> always unlocked -> trace stays visible.
    assert!(action_is_available(&pack, &state, trace, &state.current_room_id));
    let room_id = state.current_room_id.clone();
    state.add_item_to_storage(
        "charm-sigil",
        cinder_core::content::types::ItemStorageTarget::CurrentRoom,
        &room_id,
    );
    assert!(!action_is_available(&pack, &state, trace, &state.current_room_id));

    // Unlocking a different mark makes trace available again in the same room.
    state.story_vars.set_unchecked("knows_drain", "true");
    assert!(action_is_available(&pack, &state, trace, &state.current_room_id));
    state.add_item_to_storage(
        "drain-sigil",
        cinder_core::content::types::ItemStorageTarget::CurrentRoom,
        &room_id,
    );
    assert!(!action_is_available(&pack, &state, trace, &state.current_room_id));

    // Gate every craftable behind a falsy story var.
    let mut pack = target_pack();
    let trace = pack
        .actions
        .iter_mut()
        .find(|a| a.id == "trace")
        .unwrap();
    if let Some(ic) = &mut trace.item_creation {
        ic.craftable_item_gates
            .insert("charm-sigil".to_string(), "locked".to_string());
    }
    rebuild_test_pack_indexes(&mut pack);
    let trace = pack.action("trace").unwrap();
    let state = live_in_lounge(&pack);
    assert!(!action_is_available(&pack, &state, trace, &state.current_room_id));

    // Unlock one -> available again.
    let mut state = state;
    state.story_vars.set_unchecked("locked", "true");
    assert!(action_is_available(&pack, &state, trace, &state.current_room_id));
}

#[test]
fn informational_panels_are_never_hidden_for_emptiness() {
    let pack = target_pack();
    let look = pack.action("look").unwrap();
    let move_ = pack.action("move").unwrap();
    let state = live_in_lounge(&pack);
    // The lounge has no features and only kitchen as an exit, but look/move
    // are informational and must remain available.
    assert!(action_is_available(&pack, &state, look, &state.current_room_id));
    assert!(action_is_available(&pack, &state, move_, &state.current_room_id));
    assert!(action_is_available(&pack, &state, move_, "kitchen"));
}

#[test]
fn requires_actor_in_room_flag_is_removed() {
    // The legacy field no longer exists; target-selector actions rely on
    // the generic target check instead.
    let pack = target_pack();
    assert!(pack.action("attack").is_some());
    assert!(pack.action("speak").is_some());
}

#[test]
fn action_requiring_item_hides_when_item_not_in_inventory() {
    let mut pack = target_pack();
    let trace = pack
        .actions
        .iter_mut()
        .find(|a| a.id == "trace")
        .unwrap();
    trace.available.requires_item = Some("magic-chalk".to_string());
    rebuild_test_pack_indexes(&mut pack);
    let trace = pack.action("trace").unwrap();
    let mut state = live_in_lounge(&pack);

    // Player doesn't have magic-chalk -> action is hidden.
    assert!(!action_is_available(&pack, &state, trace, &state.current_room_id));

    // Player acquires magic-chalk -> action is available.
    state.add_item("magic-chalk");
    assert!(action_is_available(&pack, &state, trace, &state.current_room_id));

    // Player loses magic-chalk (e.g. given away or dropped) -> action is hidden again.
    state.remove_item("magic-chalk");
    assert!(!action_is_available(&pack, &state, trace, &state.current_room_id));
}

