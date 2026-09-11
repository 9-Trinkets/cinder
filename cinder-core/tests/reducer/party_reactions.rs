use super::common::*;
use cinder_core::content::types::{
    PackMessage, PartyCombatDecisionRule, PartyDecisionCondition, PartyDecisionTier,
    PartyOrderKind, PartyPolicyDefinition, PartyReactionAction, PartyReactionCooldown,
    PartyReactionWindow, PartySupportEffect, PartyTargetSelection,
};
use cinder_core::engine::events::{TimestampedWorldEvent, WorldEvent};
use cinder_core::engine::reducer::apply_events;
use cinder_core::engine::state::{ActorStance, WorldState};
use std::collections::BTreeMap;

const SECOND_ALLY_ID: &str = "drew";

fn rule(
    id: &str,
    tier: PartyDecisionTier,
    action: PartyReactionAction,
    conditions: Vec<PartyDecisionCondition>,
    target: PartyTargetSelection,
) -> PartyCombatDecisionRule {
    PartyCombatDecisionRule {
        id: id.to_string(),
        tier,
        window: PartyReactionWindow::AfterHostileDamage,
        action,
        conditions,
        target,
        candidate_priority: vec![],
        support_effect: None,
        cooldown: PartyReactionCooldown::FixedMinutes { minutes: 5 },
        message: match action {
            PartyReactionAction::Counterattack => "combat.party_counterattack",
            PartyReactionAction::Support => "combat.party_support",
            PartyReactionAction::Hold => "combat.party_holds",
            PartyReactionAction::Intercept => "",
        }
        .to_string(),
    }
}

fn reaction_pack(
    initial_orders: BTreeMap<String, PartyOrderKind>,
    rules: Vec<PartyCombatDecisionRule>,
) -> cinder_core::content::types::ContentPack {
    let mut pack = reducer_test_pack();
    pack.settings.combat.player_actor_id = ACTOR_A_ID.to_string();
    pack.settings.combat.health_stat_id = "stamina".to_string();
    pack.settings.combat.attack_stat_id = "confidence".to_string();
    pack.settings.combat.defense_stat_id = "hunger".to_string();
    pack.settings.combat.minimum_damage = 1;
    pack.settings.party = PartyPolicyDefinition {
        initial_orders,
        combat_rules: rules,
    };
    pack.messages.insert(
        "combat.party_counterattack".to_string(),
        PackMessage::Narration(
            "{actor} hits {target} for {damage} {kind}. ({remaining} remaining)".to_string(),
        ),
    );
    pack.messages.insert(
        "combat.party_support".to_string(),
        PackMessage::Narration(
            "{actor} supports {target} for {amount}. ({remaining} remaining)".to_string(),
        ),
    );
    pack.messages.insert(
        "combat.party_holds".to_string(),
        PackMessage::Narration("{actor} holds.".to_string()),
    );
    for actor in &mut pack.actors {
        let (confidence, stamina, hunger) = match actor.id.as_str() {
            ACTOR_A_ID => (0, 20, 0),
            ACTOR_B_ID => (8, 10, 1),
            ACTOR_C_ID => (4, 20, 2),
            _ => (0, 5, 0),
        };
        actor.initial_stats = BTreeMap::from([
            ("confidence".to_string(), confidence),
            ("stamina".to_string(), stamina),
            ("hunger".to_string(), hunger),
        ]);
    }
    let ally = pack
        .actors
        .iter_mut()
        .find(|actor| actor.id == ACTOR_B_ID)
        .unwrap();
    ally.attack_kind = "fire".to_string();
    let hostile = pack
        .actors
        .iter_mut()
        .find(|actor| actor.id == ACTOR_C_ID)
        .unwrap();
    hostile.resistances = BTreeMap::from([("fire".to_string(), 3)]);
    rebuild_test_pack_indexes(&mut pack);
    pack
}

fn combat_state(pack: &cinder_core::content::types::ContentPack) -> WorldState {
    let mut state = WorldState::new(pack);
    state
        .actor_room_overrides
        .insert(ACTOR_C_ID.to_string(), LOUNGE_ID.to_string());
    state.set_stance(ACTOR_B_ID, ActorStance::Allied);
    state.set_stance(ACTOR_C_ID, ActorStance::Hostile);
    state
}

fn hostile_strike(state: &mut WorldState, pack: &cinder_core::content::types::ContentPack) {
    apply_events(
        state,
        pack,
        &[TimestampedWorldEvent::now(WorldEvent::HostileStrike {
            actor_id: ACTOR_C_ID.to_string(),
        })],
    );
}

#[test]
fn default_striker_counterattacks_with_its_own_kind_and_target_resistance() {
    let pack = reaction_pack(
        BTreeMap::from([(ACTOR_B_ID.to_string(), "assist".to_string())]),
        vec![rule(
            "striker-default",
            PartyDecisionTier::Order,
            PartyReactionAction::Counterattack,
            vec![PartyDecisionCondition::OrderIs {
                orders: vec!["assist".to_string()],
            }],
            PartyTargetSelection::Attacker,
        )],
    );
    let mut state = combat_state(&pack);

    hostile_strike(&mut state, &pack);

    assert_eq!(state.actor_stat(ACTOR_C_ID, "stamina"), 17);
    assert_eq!(
        state.party_reaction_ready_at.get(ACTOR_B_ID),
        Some(&(state.current_time_minutes + 5))
    );
}

#[test]
fn explicit_assist_reacts_without_a_default_combat_role() {
    let pack = reaction_pack(
        BTreeMap::new(),
        vec![rule(
            "ordered-assist",
            PartyDecisionTier::Order,
            PartyReactionAction::Counterattack,
            vec![PartyDecisionCondition::OrderIs {
                orders: vec!["assist".to_string()],
            }],
            PartyTargetSelection::Attacker,
        )],
    );
    let mut state = combat_state(&pack);
    state
        .assign_party_order(&pack, ACTOR_B_ID, "assist".to_string())
        .unwrap();

    hostile_strike(&mut state, &pack);

    assert_eq!(state.actor_stat(ACTOR_C_ID, "stamina"), 17);
}

#[test]
fn survival_hold_interrupts_an_assist_order_and_consumes_readiness() {
    let hold = rule(
        "survival-hold",
        PartyDecisionTier::Survival,
        PartyReactionAction::Hold,
        vec![PartyDecisionCondition::ActorHealthAtMostPercent { percent: 25 }],
        PartyTargetSelection::SelfActor,
    );
    let assist = rule(
        "ordered-assist",
        PartyDecisionTier::Order,
        PartyReactionAction::Counterattack,
        vec![PartyDecisionCondition::OrderIs {
            orders: vec!["assist".to_string()],
        }],
        PartyTargetSelection::Attacker,
    );
    let pack = reaction_pack(BTreeMap::new(), vec![hold, assist]);
    let mut state = combat_state(&pack);
    state.adjust_actor_stat(ACTOR_B_ID, "stamina", -8).unwrap();
    state
        .assign_party_order(&pack, ACTOR_B_ID, "assist".to_string())
        .unwrap();

    let output = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::HostileStrike {
            actor_id: ACTOR_C_ID.to_string(),
        })],
    );

    assert_eq!(state.actor_stat(ACTOR_C_ID, "stamina"), 20);
    assert!(output.lines.iter().any(|line| line.text == "Blair holds."));
    assert!(state.party_reaction_ready_at.contains_key(ACTOR_B_ID));
}

#[test]
fn support_applies_its_authored_effect_to_the_selected_target() {
    let mut support = rule(
        "medic-default",
        PartyDecisionTier::Order,
        PartyReactionAction::Support,
        vec![PartyDecisionCondition::OrderIs {
            orders: vec!["assist".to_string()],
        }],
        PartyTargetSelection::Player,
    );
    support.support_effect = Some(PartySupportEffect::AdjustActorStat {
        stat: "stamina".to_string(),
        delta: 3,
    });
    let pack = reaction_pack(
        BTreeMap::from([(ACTOR_B_ID.to_string(), "assist".to_string())]),
        vec![support],
    );
    let mut state = combat_state(&pack);
    state.current_room_id = KITCHEN_ID.to_string();
    state
        .actor_room_overrides
        .insert(ACTOR_B_ID.to_string(), KITCHEN_ID.to_string());
    state
        .actor_room_overrides
        .insert(ACTOR_C_ID.to_string(), KITCHEN_ID.to_string());

    hostile_strike(&mut state, &pack);

    assert_eq!(state.actor_stat(ACTOR_A_ID, "stamina"), 19);
    assert!(state.party_reaction_ready_at.contains_key(ACTOR_B_ID));
}

#[test]
fn an_interceptor_cannot_counterattack_during_the_same_interval() {
    let intercept = PartyCombatDecisionRule {
        id: "defender-intercepts".to_string(),
        tier: PartyDecisionTier::Order,
        window: PartyReactionWindow::BeforeHostileDamage,
        action: PartyReactionAction::Intercept,
        conditions: vec![PartyDecisionCondition::OrderIs {
            orders: vec!["guard".to_string()],
        }],
        target: PartyTargetSelection::Player,
        candidate_priority: vec![],
        support_effect: None,
        cooldown: PartyReactionCooldown::FixedMinutes { minutes: 5 },
        message: String::new(),
    };
    let counter = rule(
        "striker-default",
        PartyDecisionTier::Order,
        PartyReactionAction::Counterattack,
        vec![PartyDecisionCondition::OrderIs {
            orders: vec!["assist".to_string()],
        }],
        PartyTargetSelection::Attacker,
    );
    let pack = reaction_pack(
        BTreeMap::from([(ACTOR_B_ID.to_string(), "guard".to_string())]),
        vec![intercept, counter],
    );
    let mut state = combat_state(&pack);

    hostile_strike(&mut state, &pack);

    assert_eq!(state.actor_stat(ACTOR_C_ID, "stamina"), 20);
    assert!(state.actor_stat(ACTOR_B_ID, "stamina") < 10);
}

#[test]
fn player_defeat_ends_the_strike_before_party_reactions() {
    let pack = reaction_pack(
        BTreeMap::from([(ACTOR_B_ID.to_string(), "assist".to_string())]),
        vec![rule(
            "striker-default",
            PartyDecisionTier::Order,
            PartyReactionAction::Counterattack,
            vec![PartyDecisionCondition::OrderIs {
                orders: vec!["assist".to_string()],
            }],
            PartyTargetSelection::Attacker,
        )],
    );
    let mut state = combat_state(&pack);
    state.adjust_actor_stat(ACTOR_A_ID, "stamina", -19).unwrap();

    hostile_strike(&mut state, &pack);

    assert_eq!(state.actor_stat(ACTOR_C_ID, "stamina"), 20);
    assert!(!state.party_reaction_ready_at.contains_key(ACTOR_B_ID));
}

#[test]
fn multiple_ready_members_counterattack_separately_until_the_attacker_falls() {
    let mut pack = reaction_pack(
        BTreeMap::from([
            (ACTOR_B_ID.to_string(), "assist".to_string()),
            (SECOND_ALLY_ID.to_string(), "assist".to_string()),
        ]),
        vec![rule(
            "striker-default",
            PartyDecisionTier::Order,
            PartyReactionAction::Counterattack,
            vec![PartyDecisionCondition::OrderIs {
                orders: vec!["assist".to_string()],
            }],
            PartyTargetSelection::Attacker,
        )],
    );
    let mut second = pack.actor(ACTOR_B_ID).unwrap().clone();
    second.id = SECOND_ALLY_ID.to_string();
    second.name = "Drew".to_string();
    second.attack_kind = "cold".to_string();
    second.initial_stats.insert("confidence".to_string(), 7);
    pack.actors.push(second);
    pack.actors
        .iter_mut()
        .find(|actor| actor.id == ACTOR_C_ID)
        .unwrap()
        .initial_stats
        .insert("stamina".to_string(), 7);
    rebuild_test_pack_indexes(&mut pack);
    let mut state = combat_state(&pack);
    state.set_stance(SECOND_ALLY_ID, ActorStance::Allied);

    hostile_strike(&mut state, &pack);

    assert!(state.actor_is_defeated(ACTOR_C_ID, "stamina"));
    assert!(state.party_reaction_ready_at.contains_key(ACTOR_B_ID));
    assert!(state.party_reaction_ready_at.contains_key(SECOND_ALLY_ID));
    assert_eq!(state.stance(ACTOR_C_ID), ActorStance::Neutral);
}
