use super::common::*;
use cinder_core::content::types::PackMessage;
use cinder_core::engine::events::{TimestampedWorldEvent, WorldEvent};
use cinder_core::engine::reducer::apply_events;
use cinder_core::engine::state::WorldState;

#[test]
fn player_movement_condenses_multiple_followers_into_one_line() {
    let mut pack = reducer_test_pack();
    pack.settings.combat.player_actor_id = ACTOR_A_ID.to_string();
    pack.settings.combat.health_stat_id = "stamina".to_string();
    pack.messages.insert(
        "follow.party_follows".to_string(),
        PackMessage::Narration("{actors} cross the room with you.".to_string()),
    );
    pack.messages.insert(
        "follow.actor_follows".to_string(),
        PackMessage::Narration("{actor} crosses the room with you.".to_string()),
    );
    let mut state = WorldState::new(&pack);
    state.set_follows_player(ACTOR_B_ID, true);
    state.set_follows_player(ACTOR_C_ID, true);
    state
        .actor_room_overrides
        .insert(ACTOR_C_ID.to_string(), LOUNGE_ID.to_string());

    let output = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::PlayerMoved {
            from_room_id: LOUNGE_ID.to_string(),
            to_room_id: KITCHEN_ID.to_string(),
        })],
    );

    assert_eq!(
        output
            .lines
            .iter()
            .filter(|line| line.text.contains("cross the room with you"))
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>(),
        vec!["Blair and Casey cross the room with you."]
    );
    assert_eq!(state.actor_room_id(ACTOR_B_ID, LOUNGE_ID), KITCHEN_ID);
    assert_eq!(state.actor_room_id(ACTOR_C_ID, KITCHEN_ID), KITCHEN_ID);
}
