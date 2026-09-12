use super::common::party_reactions::*;
use super::common::*;
use cinder_core::content::types::{
    PartyCombatDecisionRule, PartyDecisionCondition, PartyDecisionTier, PartyReactionAction,
    PartyReactionCooldown, PartyReactionWindow, PartySupportEffect, PartyTargetSelection,
};
use cinder_core::engine::events::{TimestampedWorldEvent, WorldEvent};
use cinder_core::engine::reducer::apply_events;
use std::collections::BTreeMap;

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

    let _ = hostile_strike(&mut state, &pack);

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

    let _ = hostile_strike(&mut state, &pack);

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

    let _ = hostile_strike(&mut state, &pack);

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

    let _ = hostile_strike(&mut state, &pack);

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

    let _ = hostile_strike(&mut state, &pack);

    assert_eq!(state.actor_stat(ACTOR_C_ID, "stamina"), 20);
    assert!(!state.party_reaction_ready_at.contains_key(ACTOR_B_ID));
}
