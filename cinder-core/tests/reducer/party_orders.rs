use super::common::*;
use cinder_core::content::types::{PackMessage, PackMessageVoice};
use cinder_core::engine::events::{TimestampedWorldEvent, WorldEvent};
use cinder_core::engine::narrative::NarrativeLineKind;
use cinder_core::engine::reducer::apply_events;
use cinder_core::engine::state::{ActorStance, WorldState};
use std::collections::BTreeMap;

fn party_order_pack() -> cinder_core::content::types::ContentPack {
    let mut pack = reducer_test_pack();
    pack.settings.party.initial_orders = BTreeMap::from([
        (ACTOR_A_ID.to_string(), "guard".to_string()),
        (ACTOR_B_ID.to_string(), "assist".to_string()),
    ]);
    pack.messages.insert(
        "party.order_guard_assigned".to_string(),
        PackMessage::Voiced {
            voice: PackMessageVoice::System,
            text: "{actor} will guard you.".to_string(),
        },
    );
    pack.messages.insert(
        "party.order_assist_assigned".to_string(),
        PackMessage::Voiced {
            voice: PackMessageVoice::System,
            text: "{actor} will assist you.".to_string(),
        },
    );
    pack
}

fn allied_party_state(pack: &cinder_core::content::types::ContentPack) -> WorldState {
    let mut state = WorldState::new(pack);
    state.set_stance(ACTOR_A_ID, ActorStance::Allied);
    state.set_stance(ACTOR_B_ID, ActorStance::Allied);
    state
}

#[test]
fn assigning_assist_overrides_guard_and_keeps_the_member_following() {
    let pack = party_order_pack();
    let mut state = allied_party_state(&pack);
    state.set_follows_player(ACTOR_A_ID, false);

    let output = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::PartyOrderAssigned {
            actor_id: ACTOR_A_ID.to_string(),
            order: "assist".to_string(),
        })],
    );

    assert_eq!(
        state.party_order(&pack, ACTOR_A_ID),
        Some("assist".to_string())
    );
    assert!(state.follows_player(ACTOR_A_ID));
    assert!(output.lines.iter().any(|line| {
        line.kind == NarrativeLineKind::System && line.text == "Alex will assist you."
    }));
}

#[test]
fn assigning_guard_overrides_assist_with_system_feedback() {
    let pack = party_order_pack();
    let mut state = allied_party_state(&pack);

    let output = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::PartyOrderAssigned {
            actor_id: ACTOR_B_ID.to_string(),
            order: "guard".to_string(),
        })],
    );

    assert_eq!(
        state.party_order(&pack, ACTOR_B_ID),
        Some("guard".to_string())
    );
    assert!(state.follows_player(ACTOR_B_ID));
    assert!(output.lines.iter().any(|line| {
        line.kind == NarrativeLineKind::System && line.text == "Blair will guard you."
    }));
}
