use super::super::common::*;
use super::{attack_action, attack_input, transcript};
use cinder_core::content::types::{CombatSettingsDefinition, DropSpec, ItemDefinition, LevelDefinition};
use cinder_core::engine::state::{ActorStance, WorldState};
use std::collections::BTreeMap;

#[test]
fn defeating_an_actor_scatters_its_drops_into_the_room() {
    let mut pack = equipment_test_pack();
    attack_action(&mut pack);
    let mut golem = test_actor("golem", "dark golem", LOUNGE_ID);
    golem.attackable = true;
    golem.initial_stats = BTreeMap::from([("stamina".to_string(), 1)]);
    golem.drops = BTreeMap::from([("herb-salve".to_string(), DropSpec::Always(2))]);
    pack.actors.push(golem);
    pack.items.push(ItemDefinition {
        id: "stone-marker".to_string(),
        label: "stone marker".to_string(),
        description: "A flat stone.".to_string(),
        ..ItemDefinition::default()
    });
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();

    let lines = drive_actor_command(
        &mut state,
        &pack,
        "attack",
        attack_input(Some("golem"), Some("dark golem")),
    )
    .lines;

    assert!(state.actor_is_defeated("golem", "stamina"));
    assert_eq!(
        state.loose_room_items(LOUNGE_ID),
        vec![("herb-salve".to_string(), 2)]
    );
    let output_text = transcript(&lines);
    assert!(
        output_text.contains("Left behind") && output_text.contains("herb salve"),
        "got: {output_text}"
    );
}

#[test]
fn defeating_an_actor_awards_full_xp_to_every_party_member_with_own_curve() {
    let mut pack = reducer_test_pack();
    pack.settings.combat = CombatSettingsDefinition {
        player_actor_id: ACTOR_A_ID.to_string(),
        health_stat_id: "stamina".to_string(),
        attack_stat_id: "confidence".to_string(),
        defense_stat_id: "hunger".to_string(),
        ..CombatSettingsDefinition::default()
    };
    // The default table levels anyone at 10 XP (L1 -> L2, +5 stamina). Blair
    // gets a per-actor override needing 20 XP, so she does NOT level here.
    pack.levels.default = vec![LevelDefinition {
        exp_required: 10,
        stat_changes: BTreeMap::from([("stamina".to_string(), 5)]),
        unlocks: vec!["power_slice".to_string()],
    }];
    pack.levels.actors = BTreeMap::from([(
        ACTOR_B_ID.to_string(),
        vec![LevelDefinition {
            exp_required: 20,
            stat_changes: BTreeMap::from([("stamina".to_string(), 2)]),
            unlocks: vec![],
        }],
    )]);
    attack_action(&mut pack);
    // A one-hp, 10-xp mob in the lounge.
    let mut goblin = test_actor("goblin", "goblin", LOUNGE_ID);
    goblin.attackable = true;
    goblin.initial_stats = BTreeMap::from([("stamina".to_string(), 1)]);
    goblin.xp_drop = 10;
    pack.actors.push(goblin);
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    state.set_relationship(
        ACTOR_B_ID,
        cinder_core::engine::state::ActorRelationship {
            stance: ActorStance::Allied,
            follows_player: true,
        },
    );

    let lines = drive_actor_command(
        &mut state,
        &pack,
        "attack",
        attack_input(Some("goblin"), Some("goblin")),
    )
    .lines;

    // Full 10 XP goes to the player (leveled 1->2 on the default curve, +5
    // stamina) and to follower Blair — who keeps it on her own 20-XP curve,
    // so she stays level 1 with 10/20 and no stat bonus.
    assert!(state.actor_is_defeated("goblin", "stamina"));
    assert_eq!(state.actor_xp(ACTOR_A_ID), 0);
    assert_eq!(state.actor_level(ACTOR_A_ID), 2);
    assert_eq!(state.actor_xp(ACTOR_B_ID), 10);
    assert_eq!(state.actor_level(ACTOR_B_ID), 1);
    // Narration is per actor: only the leveling actor is named with its own
    // new level.
    let transcripts = lines
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>();
    assert!(
        transcripts
            .iter()
            .any(|t| t.contains(ACTOR_A_NAME) && t.contains("Level 2")),
        "expected per-actor narration naming Alex at Level 2, got: {transcripts:?}"
    );
    assert!(
        !transcripts
            .iter()
            .any(|t| t.contains(ACTOR_B_NAME) && t.contains("Level 2")),
        "Blair did not level; she should not be narrated, got: {transcripts:?}"
    );
    let alex_stamina = state.actor_stat(ACTOR_A_ID, "stamina");
    let blair_stamina = state.actor_stat(ACTOR_B_ID, "stamina");
    assert_eq!(
        alex_stamina,
        blair_stamina + 5,
        "Alex should be +5 stamina; Alex={alex_stamina}, Blair={blair_stamina}"
    );
}

#[test]
fn fully_resisted_attack_deals_zero_and_narrates_no_effect() {
    let mut pack = equipment_test_pack();
    attack_action(&mut pack);
    pack.messages.insert(
        "combat.attack_hit".to_string(),
        cinder_core::content::types::PackMessage::Narration(
            "{actor} takes {damage} damage ({remaining} remaining).".to_string(),
        ),
    );
    pack.messages.insert(
        "combat.no_effect".to_string(),
        cinder_core::content::types::PackMessage::Narration(
            "The {actor} is wholly unharmed by {kind}.".to_string(),
        ),
    );
    let mut golem = test_actor("golem", "obsidian golem", LOUNGE_ID);
    golem.attackable = true;
    golem.initial_stats = BTreeMap::from([("stamina".to_string(), 10)]);
    golem.resistances = BTreeMap::from([("physical".to_string(), 999)]);
    pack.actors.push(golem);
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();

    let lines = drive_actor_command(
        &mut state,
        &pack,
        "attack",
        attack_input(Some("golem"), Some("obsidian golem")),
    )
    .lines;

    assert_eq!(state.actor_stat("golem", "stamina"), 10);
    assert!(!state.actor_is_defeated("golem", "stamina"));
    assert_eq!(state.stance("golem"), ActorStance::Hostile);
    let output_text = transcript(&lines);
    assert!(
        output_text.contains("wholly unharmed by physical"),
        "expected no_effect narration, got: {output_text}"
    );
    assert!(
        !output_text.contains("takes"),
        "expected no attack_hit narration, got: {output_text}"
    );
}

#[test]
fn partial_resistance_reduces_attack_damage() {
    let mut pack = equipment_test_pack();
    attack_action(&mut pack);
    pack.messages.insert(
        "combat.attack_hit".to_string(),
        cinder_core::content::types::PackMessage::Narration(
            "{actor} takes {damage} damage ({remaining} remaining).".to_string(),
        ),
    );
    let mut golem = test_actor("golem", "cinder golem", LOUNGE_ID);
    golem.attackable = true;
    golem.initial_stats = BTreeMap::from([("stamina".to_string(), 10)]);
    golem.resistances = BTreeMap::from([("physical".to_string(), 2)]);
    pack.actors.push(golem);
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    // Attack 6 vs defense 0 → 6 raw; resistance 2 → 4 dealt.
    state
        .actor_stats
        .entry(ACTOR_A_ID.to_string())
        .or_default()
        .insert("confidence".to_string(), 6);
    state.current_room_id = LOUNGE_ID.to_string();

    drive_actor_command(
        &mut state,
        &pack,
        "attack",
        attack_input(Some("golem"), Some("cinder golem")),
    );

    assert_eq!(state.actor_stat("golem", "stamina"), 6);
}