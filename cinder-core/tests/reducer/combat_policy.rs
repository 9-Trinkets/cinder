use super::common::*;
use cinder_core::content::types::{
    ActionDefinition, AllyAttackDefinition, AllyAttackMode, AllyAttackParticipants,
    CombatSettingsDefinition, CommandEffect, CommandTargetMode, ContentPack, LevelDefinition,
    XpDistributionDefinition, XpDistributionMode, XpRecipientMode,
};
use cinder_core::engine::state::{ActorRelationship, ActorStance, WorldState};
use std::collections::BTreeMap;

fn policy_test_pack() -> ContentPack {
    let mut pack = reducer_test_pack();
    pack.settings.combat = CombatSettingsDefinition {
        player_actor_id: ACTOR_A_ID.to_string(),
        health_stat_id: "stamina".to_string(),
        attack_stat_id: "confidence".to_string(),
        defense_stat_id: "hunger".to_string(),
        ..CombatSettingsDefinition::default()
    };
    pack.actions.push(ActionDefinition {
        id: "attack".to_string(),
        command: "attack".to_string(),
        target_mode: CommandTargetMode::Actor,
        effects: vec![CommandEffect::AttackTarget],
        event_text: "{actor_name} strikes {target_actor_name}.".to_string(),
        ..ActionDefinition::default()
    });
    pack
}

fn add_goblin(pack: &mut ContentPack, health: i32, xp_drop: u32) {
    let mut goblin = test_actor("goblin", "goblin", LOUNGE_ID);
    goblin.attackable = true;
    goblin.initial_stats =
        BTreeMap::from([("stamina".to_string(), health), ("hunger".to_string(), 0)]);
    goblin.xp_drop = xp_drop;
    pack.actors.push(goblin);
    rebuild_test_pack_indexes(pack);
}

fn attack_goblin(state: &mut WorldState, pack: &ContentPack) {
    drive_actor_command(
        state,
        pack,
        "attack",
        ActorCommandInput {
            actor_id: ACTOR_A_ID,
            actor_name: ACTOR_A_NAME,
            room_id: LOUNGE_ID,
            target_room_id: None,
            target_actor_id: Some("goblin"),
            target_actor_name: Some("goblin"),
            context_label: None,
            feature_id: None,
            consumable_id: None,
            freeform_text: None,
        },
    );
}

fn add_follower(state: &mut WorldState, actor_id: &str) {
    state.set_relationship(
        actor_id,
        ActorRelationship {
            stance: ActorStance::Allied,
            follows_player: true,
        },
    );
}

#[test]
fn xp_can_be_split_evenly_between_player_and_followers() {
    let mut pack = policy_test_pack();
    pack.settings.combat.xp_distribution = XpDistributionDefinition {
        recipients: XpRecipientMode::PlayerAndFollowers,
        mode: XpDistributionMode::SplitEvenly,
    };
    pack.levels.default = vec![LevelDefinition {
        exp_required: 100,
        ..Default::default()
    }];
    add_goblin(&mut pack, 1, 11);

    let mut state = WorldState::new(&pack);
    add_follower(&mut state, ACTOR_B_ID);
    attack_goblin(&mut state, &pack);

    assert_eq!(state.actor_xp(ACTOR_A_ID), 6);
    assert_eq!(state.actor_xp(ACTOR_B_ID), 5);
}

#[test]
fn xp_can_be_awarded_to_the_player_only() {
    let mut pack = policy_test_pack();
    pack.settings.combat.xp_distribution = XpDistributionDefinition {
        recipients: XpRecipientMode::PlayerOnly,
        mode: XpDistributionMode::FullEach,
    };
    pack.levels.default = vec![LevelDefinition {
        exp_required: 100,
        ..Default::default()
    }];
    add_goblin(&mut pack, 1, 10);

    let mut state = WorldState::new(&pack);
    add_follower(&mut state, ACTOR_B_ID);
    attack_goblin(&mut state, &pack);

    assert_eq!(state.actor_xp(ACTOR_A_ID), 10);
    assert_eq!(state.actor_xp(ACTOR_B_ID), 0);
}

#[test]
fn ally_attack_policy_can_filter_scale_and_cap_contributions() {
    let mut pack = policy_test_pack();
    pack.settings.combat.ally_attack = AllyAttackDefinition {
        participants: AllyAttackParticipants::FollowersOnly,
        mode: AllyAttackMode::AttackStat,
        contribution_percent: 50,
        maximum_per_ally: Some(1),
    };
    add_goblin(&mut pack, 10, 0);

    let mut state = WorldState::new(&pack);
    for (actor_id, attack) in [(ACTOR_A_ID, 2), (ACTOR_B_ID, 6), (ACTOR_C_ID, 20)] {
        state
            .actor_stats
            .entry(actor_id.to_string())
            .or_default()
            .insert("confidence".to_string(), attack);
    }
    state
        .actor_room_overrides
        .insert(ACTOR_C_ID.to_string(), LOUNGE_ID.to_string());
    add_follower(&mut state, ACTOR_B_ID);
    state.set_relationship(
        ACTOR_C_ID,
        ActorRelationship {
            stance: ActorStance::Allied,
            follows_player: false,
        },
    );

    attack_goblin(&mut state, &pack);

    assert_eq!(state.actor_stat("goblin", "stamina"), 7);
}
