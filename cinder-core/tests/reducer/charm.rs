use super::common::*;
use cinder_core::content::types::{
    ActionDefinition, ActionItemCreation, ActionItemStorageTarget, CharmRule,
    CombatSettingsDefinition, CommandEffect, CommandTargetMode, ContentPack, ItemDefinition,
    ItemKind, ItemStorageTarget, PackMessage, StatDefinition,
};
use cinder_core::engine::reducer::ReducerOutput;
use cinder_core::engine::state::{ActorStance, WorldState};
use serde_json::json;
use std::collections::BTreeMap;

const CHARM_REFUSED: &str = "THE RING DOES NOT HOLD.";

/// Pack with the `IntAndLevel` charm rule, intelligence stat, player int 5,
/// and a trace action that drops a charm-sigil into the current room.
fn charm_test_pack() -> ContentPack {
    let mut pack = reducer_test_pack();
    pack.settings.combat = CombatSettingsDefinition {
        player_actor_id: ACTOR_A_ID.to_string(),
        health_stat_id: "stamina".to_string(),
        attack_stat_id: "confidence".to_string(),
        defense_stat_id: "hunger".to_string(),
        ..CombatSettingsDefinition::default()
    };
    pack.settings.charm_rule = CharmRule::IntAndLevel;
    pack.settings.equipment_slots = ["ring".to_string()].into_iter().collect();
    pack.stats.actor.insert(
        "intelligence".to_string(),
        StatDefinition {
            default: 3,
            ..StatDefinition::default()
        },
    );
    let alex = pack
        .actors
        .iter_mut()
        .find(|actor| actor.id == ACTOR_A_ID)
        .expect("player actor present");
    alex.initial_stats.insert("intelligence".to_string(), 5);
    pack.actions.push(ActionDefinition {
        id: "trace".to_string(),
        command: "trace".to_string(),
        target_mode: CommandTargetMode::None,
        item_creation: Some(ActionItemCreation {
            creates_item: "charm-sigil".to_string(),
            storage: ActionItemStorageTarget::CurrentRoom,
            ..ActionItemCreation::default()
        }),
        event_text: "{actor_name} draws chalk across the floor.".to_string(),
        ..ActionDefinition::default()
    });
    pack.items.push(ItemDefinition {
        id: "int-ring".to_string(),
        label: "ring of the open mind".to_string(),
        description: "A warm ring of dark bone.".to_string(),
        kind: ItemKind::Trinket,
        equip_slot: "ring".to_string(),
        stat_bonuses: BTreeMap::from([("intelligence".to_string(), 4)]),
        use_hook: String::new(),
        equip_hook: String::new(),
        look_description: String::new(),
        trace_mark: false,
        consumed_on_surround_conversion: false,
    });
    pack.actions.push(ActionDefinition {
        id: "equip-ring".to_string(),
        command: "equip-ring".to_string(),
        target_mode: CommandTargetMode::None,
        effects: vec![CommandEffect::EquipItem],
        item_id: "int-ring".to_string(),
        event_text: "{actor_name} slips the ring on.".to_string(),
        ..ActionDefinition::default()
    });
    pack.messages.insert(
        "conversion.encircled".to_string(),
        PackMessage::Narration("The {actor} turns toward you, no longer hostile.".to_string()),
    );
    pack.hooks.insert(
        "actor.surrounded".to_string(),
        effect_hook(vec![json!({
            "kind": "convert_actor_to_ally",
            "actor_id": "$input.actor_id",
            "follows_player": true,
            "messages": ["conversion.encircled"],
        })]),
    );
    rebuild_test_pack_indexes(&mut pack);
    pack
}

fn add_charm_target(pack: &mut ContentPack, id: &str, intelligence: i32, level: u32) {
    let mut target = test_actor(id, id, LOUNGE_ID);
    target.initial_stats.insert("intelligence".to_string(), intelligence);
    target.level = level;
    pack.actors.push(target);
    rebuild_test_pack_indexes(pack);
}

fn trace_sigil(state: &mut WorldState, pack: &ContentPack) -> ReducerOutput {
    drive_actor_command(
        state,
        pack,
        "trace",
        ActorCommandInput {
            actor_id: ACTOR_A_ID,
            actor_name: ACTOR_A_NAME,
            room_id: KITCHEN_ID,
            target_room_id: None,
            target_actor_id: None,
            target_actor_name: None,
            context_label: None,
            feature_id: None,
            consumable_id: None,
            freeform_text: None,
        },
    )
}

#[test]
fn charm_rule_converts_a_weak_golem_at_start() {
    let mut pack = charm_test_pack();
    add_charm_target(&mut pack, "golem-child", 1, 1);
    let mut state = WorldState::new(&pack);
    state.current_room_id = KITCHEN_ID.to_string();

    let output = trace_sigil(&mut state, &pack);

    assert_eq!(state.stance("golem-child"), ActorStance::Allied);
    assert!(
        output
            .lines
            .iter()
            .any(|line| line.text.contains("turns toward you")),
        "expected conversion narration, got {:?}",
        output.lines
    );
    assert!(
        !output
            .lines
            .iter()
            .any(|line| line.text.contains(CHARM_REFUSED)),
        "a level-1 int-1 golem must convert without a refusal, got {:?}",
        output.lines
    );
}

#[test]
fn charm_rule_refuses_a_strong_pawn_at_start() {
    let mut pack = charm_test_pack();
    // Elf pawn, int 3, level 3 -> resistance 9 > player score (int 5 + level 1).
    add_charm_target(&mut pack, "elf-pawn", 3, 3);
    let mut state = WorldState::new(&pack);
    state.current_room_id = KITCHEN_ID.to_string();

    let output = trace_sigil(&mut state, &pack);

    assert_ne!(state.stance("elf-pawn"), ActorStance::Allied);
    assert!(
        output
            .lines
            .iter()
            .any(|line| line.text.contains(CHARM_REFUSED)),
        "expected a cold refusal line, got {:?}",
        output.lines
    );
}

#[test]
fn shaman_is_not_charmable_at_start_without_a_hard_gate() {
    let mut pack = charm_test_pack();
    // Goblin shaman, int 6, level 6 -> resistance 18, permanently above the
    // player's ceiling. Immunity is numeric: no hook condition excludes it.
    let mut shaman = test_actor("goblin-shaman", "goblin shaman", LOUNGE_ID);
    shaman.initial_stats.insert("intelligence".to_string(), 6);
    shaman.level = 6;
    pack.actors.push(shaman);
    rebuild_test_pack_indexes(&mut pack);
    let mut state = WorldState::new(&pack);
    state.current_room_id = KITCHEN_ID.to_string();

    let output = trace_sigil(&mut state, &pack);

    assert_ne!(state.stance("goblin-shaman"), ActorStance::Allied);
    assert!(
        output
            .lines
            .iter()
            .any(|line| line.text.contains(CHARM_REFUSED)),
        "expected a cold refusal line, got {:?}",
        output.lines
    );
}

#[test]
fn player_level_three_plus_int_grants_unlock_previously_strong_targets() {
    let mut pack = charm_test_pack();
    add_charm_target(&mut pack, "elf-pawn", 3, 3);
    add_charm_target(&mut pack, "elf-knight", 4, 4);
    let mut state = WorldState::new(&pack);
    state.current_room_id = KITCHEN_ID.to_string();
    // Level 3, int 7 (base 5 + two level-ups): score 10.
    state.actor_level.insert(ACTOR_A_ID.to_string(), 3);
    state
        .actor_stats
        .entry(ACTOR_A_ID.to_string())
        .or_default()
        .insert("intelligence".to_string(), 7);

    let output = trace_sigil(&mut state, &pack);

    // Pawn (resistance 9) converts; knight (resistance 12) still holds.
    assert_eq!(state.stance("elf-pawn"), ActorStance::Allied);
    assert_ne!(state.stance("elf-knight"), ActorStance::Allied);
    let refused = output
        .lines
        .iter()
        .filter(|line| line.text.contains(CHARM_REFUSED))
        .count();
    assert_eq!(refused, 1, "exactly the knight is refused, got {:?}", output.lines);
}

#[test]
fn equipping_an_intelligence_ring_widens_charm_range() {
    let mut pack = charm_test_pack();
    // Pawn: resistance 9. Player score at start (int 5 + level 1) is 6 -> not
    // enough; with the ring (int 9 + level 1) it is 10 -> converts.
    let mut pawn = test_actor("elf-pawn", "elf pawn", LOUNGE_ID);
    pawn.initial_stats.insert("intelligence".to_string(), 3);
    pawn.level = 3;
    pack.actors.push(pawn);
    rebuild_test_pack_indexes(&mut pack);
    let mut state = WorldState::new(&pack);
    state.current_room_id = KITCHEN_ID.to_string();
    state.add_item("int-ring");

    let refused = trace_sigil(&mut state, &pack);
    assert_ne!(state.stance("elf-pawn"), ActorStance::Allied);
    assert!(
        refused
            .lines
            .iter()
            .any(|line| line.text.contains(CHARM_REFUSED)),
        "expected refusal before the ring, got {:?}",
        refused.lines
    );

    let equipped = drive_actor_command(
        &mut state,
        &pack,
        "equip-ring",
        ActorCommandInput {
            actor_id: ACTOR_A_ID,
            actor_name: ACTOR_A_NAME,
            room_id: KITCHEN_ID,
            target_room_id: None,
            target_actor_id: None,
            target_actor_name: None,
            context_label: None,
            feature_id: None,
            consumable_id: None,
            freeform_text: None,
        },
    );
    assert_eq!(state.equipped_item("ring"), Some("int-ring"));
    // The pawn is no longer encircled (sigil stayed in the kitchen); draw again.
    let _ = equipped;
    state
        .add_item_to_storage("charm-sigil", ItemStorageTarget::CurrentRoom, LOUNGE_ID);
    let converted = trace_sigil(&mut state, &pack);

    assert_eq!(state.stance("elf-pawn"), ActorStance::Allied);
    assert!(
        converted
            .lines
            .iter()
            .any(|line| line.text.contains("turns toward you")),
        "expected conversion with the ring equipped, got {:?}",
        converted.lines
    );
}