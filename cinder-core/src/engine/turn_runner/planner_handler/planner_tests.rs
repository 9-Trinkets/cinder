use super::build_planned_turn;
use crate::content::types::{ContentPack, OpeningMenuDefinition, PartyOrderKind};
use crate::engine::commands::PlayerCommand;
use crate::engine::events::WorldEvent;
use crate::engine::state::{ActorStance, WorldSnapshot, WorldState};
use crate::engine::test_fixtures::{minimal_test_pack, rebuild_test_pack_indexes};
use crate::engine::turn_runner::types::{AggregatedTurn, CommandSignal, PlannedTurn};

fn plan_unknown(content: &ContentPack, state: &WorldState, raw_input: &str) -> (PlannedTurn, bool) {
    build_planned_turn(
        content,
        AggregatedTurn {
            command: CommandSignal {
                raw_input: raw_input.to_string(),
                command: PlayerCommand::Unknown,
            },
            world: WorldSnapshot {
                turn_number: state.turn_number,
                current_room_id: state.current_room_id.clone(),
            },
        },
        state,
        state.turn_number + 1,
        false,
    )
}

fn plan_order(
    content: &ContentPack,
    state: &WorldState,
    actor_reference: &str,
    order: PartyOrderKind,
) -> (PlannedTurn, bool) {
    build_planned_turn(
        content,
        AggregatedTurn {
            command: CommandSignal {
                raw_input: format!("order {actor_reference} {order:?}"),
                command: PlayerCommand::PartyOrder {
                    actor_reference: actor_reference.to_string(),
                    order,
                },
            },
            world: WorldSnapshot {
                turn_number: state.turn_number,
                current_room_id: state.current_room_id.clone(),
            },
        },
        state,
        state.turn_number + 1,
        false,
    )
}

#[test]
fn unknown_input_without_a_menu_emits_unknown_input() {
    let content = minimal_test_pack();
    let state = WorldState::new(&content);

    let (planned, advances_time) = plan_unknown(&content, &state, "eqrgmquz");

    assert!(!advances_time);
    assert!(
        planned
            .events
            .iter()
            .any(|event| matches!(event, WorldEvent::UnknownInput { .. }))
    );
}

#[test]
fn unknown_input_while_a_menu_is_open_is_rejected_as_an_invalid_choice() {
    let mut content = minimal_test_pack();
    content.menus.push(OpeningMenuDefinition {
        id: "test-menu".to_string(),
        invalid_choice_text: "That is not a choice.".to_string(),
        options: vec![crate::content::types::OpeningMenuOptionDefinition {
            id: "leave".to_string(),
            title: "Leave".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    });
    rebuild_test_pack_indexes(&mut content);
    let mut state = WorldState::new(&content);
    state.active_menu_id = Some("test-menu".to_string());

    let (planned, advances_time) = plan_unknown(&content, &state, "bleh");

    assert!(!advances_time);
    assert!(
        planned.events.iter().any(|event| matches!(
            event,
            WorldEvent::ActionRejected { message } if message == "That is not a choice."
        )),
        "expected an invalid-choice rejection, got: {:?}",
        planned.events
    );
    assert!(
        !planned
            .events
            .iter()
            .any(|event| matches!(event, WorldEvent::UnknownInput { .. }))
    );
}

#[test]
fn party_order_resolves_an_allied_member_by_stable_id_without_advancing_time() {
    let content = minimal_test_pack();
    let mut state = WorldState::new(&content);
    state.set_stance("blair", ActorStance::Allied);

    let (planned, advances_time) = plan_order(&content, &state, "blair", "assist".to_string());

    assert!(!advances_time);
    assert!(planned.events.iter().any(|event| matches!(
        event,
        WorldEvent::PartyOrderAssigned { actor_id, order }
            if actor_id == "blair" && order == "assist"
    )));
}

#[test]
fn party_order_rejects_members_outside_the_current_party() {
    let content = minimal_test_pack();
    let state = WorldState::new(&content);

    let (planned, advances_time) = plan_order(&content, &state, "blair", "guard".to_string());

    assert!(!advances_time);
    assert!(
        planned
            .events
            .iter()
            .any(|event| matches!(event, WorldEvent::ActionRejected { .. }))
    );
}

#[test]
fn party_order_rejects_ambiguous_member_names() {
    let mut content = minimal_test_pack();
    let mut duplicate = content.actor("blair").unwrap().clone();
    duplicate.id = "other-blair".to_string();
    content.actors.push(duplicate);
    rebuild_test_pack_indexes(&mut content);
    let mut state = WorldState::new(&content);
    state.set_stance("blair", ActorStance::Allied);
    state.set_stance("other-blair", ActorStance::Allied);

    let (planned, advances_time) = plan_order(&content, &state, "Blair", "guard".to_string());

    assert!(!advances_time);
    assert!(
        planned
            .events
            .iter()
            .any(|event| matches!(event, WorldEvent::ActionRejected { .. }))
    );
}