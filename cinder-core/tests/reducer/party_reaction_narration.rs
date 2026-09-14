use super::common::party_reactions::*;
use super::common::*;
use cinder_core::content::types::{
    PartyDecisionCondition, PartyDecisionTier, PartyReactionAction, PartySupportEffect,
    PartyTargetSelection,
};
use cinder_core::engine::state::ActorStance;
use std::collections::BTreeMap;

#[test]
fn multiple_holds_are_narrated_once() {
    let hold = rule(
        "survival-hold",
        PartyDecisionTier::Survival,
        PartyReactionAction::Hold,
        vec![PartyDecisionCondition::ActorHealthAtMostPercent { percent: 25 }],
        PartyTargetSelection::SelfActor,
    );
    let mut pack = reaction_pack(
        BTreeMap::from([
            (ACTOR_B_ID.to_string(), "assist".to_string()),
            (SECOND_ALLY_ID.to_string(), "assist".to_string()),
        ]),
        vec![hold],
    );
    add_second_ally(&mut pack);
    let mut state = combat_state(&pack);
    state.set_stance(SECOND_ALLY_ID, ActorStance::Allied);
    state.adjust_actor_stat(&pack, ACTOR_B_ID, "stamina", -8).unwrap();
    state
        .adjust_actor_stat(&pack, SECOND_ALLY_ID, "stamina", -8)
        .unwrap();

    let output = hostile_strike(&mut state, &pack);

    assert_eq!(
        output
            .lines
            .iter()
            .filter(|line| line.text.contains("hold"))
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>(),
        vec!["Blair and Drew hold."]
    );
}

#[test]
fn matching_support_reactions_are_narrated_once() {
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
    let mut pack = reaction_pack(
        BTreeMap::from([
            (ACTOR_B_ID.to_string(), "assist".to_string()),
            (SECOND_ALLY_ID.to_string(), "assist".to_string()),
        ]),
        vec![support],
    );
    add_second_ally(&mut pack);
    let mut state = combat_state(&pack);
    state.set_stance(SECOND_ALLY_ID, ActorStance::Allied);

    let output = hostile_strike(&mut state, &pack);

    assert_eq!(
        state.actor_stat(ACTOR_A_ID, "stamina"),
        state.actor_stat_maximum(&pack, ACTOR_A_ID, "stamina")
    );
    assert_eq!(
        output
            .lines
            .iter()
            .filter(|line| line.text.contains("support Alex"))
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>(),
        vec!["Blair and Drew support Alex for 4. (20 remaining)"]
    );
}

#[test]
fn mixed_reactions_are_narrated_in_execution_order_without_nonadjacent_grouping() {
    let counter = rule(
        "ordered-counter",
        PartyDecisionTier::Order,
        PartyReactionAction::Counterattack,
        vec![PartyDecisionCondition::OrderIs {
            orders: vec!["assist".to_string()],
        }],
        PartyTargetSelection::Attacker,
    );
    let hold = rule(
        "ordered-hold",
        PartyDecisionTier::Order,
        PartyReactionAction::Hold,
        vec![PartyDecisionCondition::OrderIs {
            orders: vec!["guard".to_string()],
        }],
        PartyTargetSelection::SelfActor,
    );
    let mut pack = reaction_pack(
        BTreeMap::from([
            (ACTOR_B_ID.to_string(), "assist".to_string()),
            (SECOND_ALLY_ID.to_string(), "guard".to_string()),
            (THIRD_ALLY_ID.to_string(), "assist".to_string()),
        ]),
        vec![counter, hold],
    );
    add_second_ally(&mut pack);
    add_ally(&mut pack, THIRD_ALLY_ID, "Evan", 7);
    let mut state = combat_state(&pack);
    state.set_stance(SECOND_ALLY_ID, ActorStance::Allied);
    state.set_stance(THIRD_ALLY_ID, ActorStance::Allied);

    let output = hostile_strike(&mut state, &pack);

    assert_eq!(
        output
            .lines
            .iter()
            .filter(|line| {
                line.text.contains("Blair")
                    || line.text.contains("Drew")
                    || line.text.contains("Evan")
            })
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>(),
        vec![
            "Blair hits Casey for 3 fire. (17 remaining)",
            "Drew holds.",
            "Evan hits Casey for 5 cold. (12 remaining)",
        ]
    );
}

#[test]
fn empty_messages_suppress_normal_reaction_narration_without_suppressing_mechanics() {
    let mut counter = rule(
        "silent-counter",
        PartyDecisionTier::Order,
        PartyReactionAction::Counterattack,
        vec![PartyDecisionCondition::OrderIs {
            orders: vec!["assist".to_string()],
        }],
        PartyTargetSelection::Attacker,
    );
    counter.message.clear();
    let mut support = rule(
        "silent-support",
        PartyDecisionTier::Order,
        PartyReactionAction::Support,
        vec![PartyDecisionCondition::OrderIs {
            orders: vec!["support".to_string()],
        }],
        PartyTargetSelection::Player,
    );
    support.message.clear();
    support.support_effect = Some(PartySupportEffect::AdjustActorStat {
        stat: "stamina".to_string(),
        delta: 3,
    });
    let mut hold = rule(
        "silent-hold",
        PartyDecisionTier::Order,
        PartyReactionAction::Hold,
        vec![PartyDecisionCondition::OrderIs {
            orders: vec!["guard".to_string()],
        }],
        PartyTargetSelection::SelfActor,
    );
    hold.message.clear();
    let mut pack = reaction_pack(
        BTreeMap::from([
            (ACTOR_B_ID.to_string(), "assist".to_string()),
            (SECOND_ALLY_ID.to_string(), "assist".to_string()),
            (THIRD_ALLY_ID.to_string(), "support".to_string()),
            (FOURTH_ALLY_ID.to_string(), "support".to_string()),
            (FIFTH_ALLY_ID.to_string(), "guard".to_string()),
            (SIXTH_ALLY_ID.to_string(), "guard".to_string()),
        ]),
        vec![counter, support, hold],
    );
    add_second_ally(&mut pack);
    add_ally(&mut pack, THIRD_ALLY_ID, "Evan", 7);
    add_ally(&mut pack, FOURTH_ALLY_ID, "Faye", 7);
    add_ally(&mut pack, FIFTH_ALLY_ID, "Gia", 7);
    add_ally(&mut pack, SIXTH_ALLY_ID, "Hank", 7);
    let mut state = combat_state(&pack);
    for actor_id in [
        SECOND_ALLY_ID,
        THIRD_ALLY_ID,
        FOURTH_ALLY_ID,
        FIFTH_ALLY_ID,
        SIXTH_ALLY_ID,
    ] {
        state.set_stance(actor_id, ActorStance::Allied);
    }

    let output = hostile_strike(&mut state, &pack);

    assert_eq!(state.actor_stat(ACTOR_C_ID, "stamina"), 12);
    assert_eq!(
        state.actor_stat(ACTOR_A_ID, "stamina"),
        state.actor_stat_maximum(&pack, ACTOR_A_ID, "stamina")
    );
    for actor_id in [
        ACTOR_B_ID,
        SECOND_ALLY_ID,
        THIRD_ALLY_ID,
        FOURTH_ALLY_ID,
        FIFTH_ALLY_ID,
        SIXTH_ALLY_ID,
    ] {
        assert!(state.party_reaction_ready_at.contains_key(actor_id));
    }
    assert!(output.lines.iter().all(|line| {
        !line.text.contains("Blair")
            && !line.text.contains("Drew")
            && !line.text.contains("Evan")
            && !line.text.contains("Faye")
            && !line.text.contains("Gia")
            && !line.text.contains("Hank")
    }));
}

#[test]
fn empty_counterattack_message_still_narrates_no_effect() {
    let mut counter = rule(
        "silent-counter",
        PartyDecisionTier::Order,
        PartyReactionAction::Counterattack,
        vec![PartyDecisionCondition::OrderIs {
            orders: vec!["assist".to_string()],
        }],
        PartyTargetSelection::Attacker,
    );
    counter.message.clear();
    let mut pack = reaction_pack(
        BTreeMap::from([(ACTOR_B_ID.to_string(), "assist".to_string())]),
        vec![counter],
    );
    pack.actors
        .iter_mut()
        .find(|actor| actor.id == ACTOR_C_ID)
        .unwrap()
        .resistances
        .insert("fire".to_string(), 99);
    rebuild_test_pack_indexes(&mut pack);
    let mut state = combat_state(&pack);

    let output = hostile_strike(&mut state, &pack);

    assert_eq!(state.actor_stat(ACTOR_C_ID, "stamina"), 20);
    assert!(state.party_reaction_ready_at.contains_key(ACTOR_B_ID));
    assert!(
        output
            .lines
            .iter()
            .any(|line| line.text == "No effect on Casey from fire.")
    );
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
    add_second_ally(&mut pack);
    pack.actors
        .iter_mut()
        .find(|actor| actor.id == ACTOR_C_ID)
        .unwrap()
        .initial_stats
        .insert("stamina".to_string(), 7);
    rebuild_test_pack_indexes(&mut pack);
    let mut state = combat_state(&pack);
    state.set_stance(SECOND_ALLY_ID, ActorStance::Allied);

    let output = hostile_strike(&mut state, &pack);

    assert!(state.actor_is_defeated(ACTOR_C_ID, "stamina"));
    assert!(state.party_reaction_ready_at.contains_key(ACTOR_B_ID));
    assert!(state.party_reaction_ready_at.contains_key(SECOND_ALLY_ID));
    assert_eq!(state.stance(ACTOR_C_ID), ActorStance::Neutral);
    assert_eq!(
        output
            .lines
            .iter()
            .filter(|line| line.text.contains("hit Casey"))
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>(),
        vec!["Blair and Drew hit Casey for 8. (-1 remaining)"]
    );
}
