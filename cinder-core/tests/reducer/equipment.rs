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
            trace_mark: false,
        });
    pack.messages.insert(
        "conversion.core".to_string(),
        "The {actor} bows its head and falls in behind you.".to_string(),
    );
    pack.hooks.insert(
        "item.core_equipped".to_string(),
        effect_hook(vec![json!({
            "kind": "set_stance_by_tag",
            "tag": "golem",
            "stance": "allied",
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
        "equip-core",
        ActorCommandInput {
            actor_id: ACTOR_A_ID,
            actor_name: ACTOR_A_NAME,
            room_id: LOUNGE_ID,
            target_room_id: None,
            target_actor_id: None,
            target_actor_name: None,
            context_label: None,
            feature_id: None,
            consumable_id: None,
            freeform_text: None,
        },
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
fn converting_tagged_actors_to_neutral_stance_clears_the_ally_label() {
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
            id: "king-crown".to_string(),
            label: "king crown".to_string(),
            description: "The dead king's crown.".to_string(),
            kind: cinder_core::content::types::ItemKind::Trinket,
            equip_slot: "trinket".to_string(),
            stat_bonuses: BTreeMap::new(),
            use_hook: String::new(),
            equip_hook: "item.crown_equipped".to_string(),
            look_description: String::new(),
            trace_mark: false,
        });
    pack.hooks.insert(
        "item.crown_equipped".to_string(),
        effect_hook(vec![json!({
            "kind": "set_stance_by_tag",
            "tag": "elf",
            "stance": "neutral",
            "follows_player": false,
            "messages": [],
        })]),
    );
    pack.actions.push(ActionDefinition {
        id: "equip-crown".to_string(),
        command: "equip-crown".to_string(),
        target_mode: CommandTargetMode::None,
        effects: vec![CommandEffect::EquipItem],
        item_id: "king-crown".to_string(),
        event_text: "{actor_name} claims the king's crown.".to_string(),
        ..ActionDefinition::default()
    });
    // One surviving hostile elf and one dead elf.
    let mut elf_living = test_actor("elf-guard", "elf guard", KITCHEN_ID);
    elf_living.tags = vec!["elf".to_string()];
    elf_living.initial_stats = BTreeMap::from([("stamina".to_string(), 8)]);
    let mut elf_dead = test_actor("elf-fallen", "fallen elf", KITCHEN_ID);
    elf_dead.tags = vec!["elf".to_string()];
    elf_dead.initial_stats = BTreeMap::from([("stamina".to_string(), 0)]);
    pack.actors.extend([elf_living, elf_dead]);
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    state.set_stance("elf-guard", ActorStance::Hostile);
    state.add_item("king-crown");

    drive_actor_command(
        &mut state,
        &pack,
        "equip-crown",
        ActorCommandInput {
            actor_id: ACTOR_A_ID,
            actor_name: ACTOR_A_NAME,
            room_id: LOUNGE_ID,
            target_room_id: None,
            target_actor_id: None,
            target_actor_name: None,
            context_label: None,
            feature_id: None,
            consumable_id: None,
            freeform_text: None,
        },
    );

    // The surviving elf stands down to neutral; the dead elf is left untouched.
    assert_eq!(state.stance("elf-guard"), ActorStance::Neutral);
    assert!(!state.relationship("elf-guard").follows_player);
    assert_eq!(state.stance("elf-fallen"), ActorStance::Neutral);

    // A hostile actor turned neutral renders without the "(ally)" suffix.
    pack.presentation.presentation_text.ally_suffix = " (ally)".to_string();
    pack.presentation.presentation_text.hostile_suffix = " (enemy)".to_string();
    pack.presentation.presentation_text.room_observation =
        "{room_title} {body} {people}".to_string();
    pack.presentation.presentation_text.people = "Here: {people}.".to_string();
    state.current_room_id = KITCHEN_ID.to_string();
    let text = cinder_core::engine::reducer::apply_events(
        &mut state,
        &pack,
        &[cinder_core::engine::events::TimestampedWorldEvent::now(
            cinder_core::engine::events::WorldEvent::CurrentRoomObserved {
                room_id: KITCHEN_ID.to_string(),
                mode: cinder_core::engine::events::ObservationMode::Summary,
            },
        )],
    )
    .lines
    .to_text();
    assert!(!text.contains("elf guard (ally)"), "got: {text}");
    assert!(!text.contains("elf guard (enemy)"), "got: {text}");
    assert!(text.contains("elf guard"), "got: {text}");
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
        "equip-chisel",
        ActorCommandInput {
            actor_id: ACTOR_A_ID,
            actor_name: ACTOR_A_NAME,
            room_id: LOUNGE_ID,
            target_room_id: None,
            target_actor_id: None,
            target_actor_name: None,
            context_label: None,
            feature_id: None,
            consumable_id: None,
            freeform_text: None,
        },
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
        "unequip-chisel",
        ActorCommandInput {
            actor_id: ACTOR_A_ID,
            actor_name: ACTOR_A_NAME,
            room_id: LOUNGE_ID,
            target_room_id: None,
            target_actor_id: None,
            target_actor_name: None,
            context_label: None,
            feature_id: None,
            consumable_id: None,
            freeform_text: None,
        },
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
        "equip-chisel",
        ActorCommandInput {
            actor_id: ACTOR_A_ID,
            actor_name: ACTOR_A_NAME,
            room_id: LOUNGE_ID,
            target_room_id: None,
            target_actor_id: None,
            target_actor_name: None,
            context_label: None,
            feature_id: None,
            consumable_id: None,
            freeform_text: None,
        },
    );
    drive_actor_command(
        &mut state,
        &pack,
        "equip-steel",
        ActorCommandInput {
            actor_id: ACTOR_A_ID,
            actor_name: ACTOR_A_NAME,
            room_id: LOUNGE_ID,
            target_room_id: None,
            target_actor_id: None,
            target_actor_name: None,
            context_label: None,
            feature_id: None,
            consumable_id: None,
            freeform_text: None,
        },
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
        "equip-chisel",
        ActorCommandInput {
            actor_id: ACTOR_A_ID,
            actor_name: ACTOR_A_NAME,
            room_id: LOUNGE_ID,
            target_room_id: None,
            target_actor_id: None,
            target_actor_name: None,
            context_label: None,
            feature_id: None,
            consumable_id: None,
            freeform_text: None,
        },
    );
    assert_eq!(state.equipped_item("weapon"), Some("iron-chisel"));
    assert_eq!(state.item_count("iron-chisel"), 1);

    // Equipping the same item again is rejected: it already fills the slot.
    let second = drive_actor_command(
        &mut state,
        &pack,
        "equip-chisel",
        ActorCommandInput {
            actor_id: ACTOR_A_ID,
            actor_name: ACTOR_A_NAME,
            room_id: LOUNGE_ID,
            target_room_id: None,
            target_actor_id: None,
            target_actor_name: None,
            context_label: None,
            feature_id: None,
            consumable_id: None,
            freeform_text: None,
        },
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
        "use-salve",
        ActorCommandInput {
            actor_id: ACTOR_A_ID,
            actor_name: ACTOR_A_NAME,
            room_id: LOUNGE_ID,
            target_room_id: None,
            target_actor_id: None,
            target_actor_name: None,
            context_label: None,
            feature_id: None,
            consumable_id: None,
            freeform_text: None,
        },
    );

    assert_eq!(state.actor_stat(ACTOR_A_ID, "stamina"), stamina_before + 2);
    assert_eq!(state.item_count("herb-salve"), 1);
}
