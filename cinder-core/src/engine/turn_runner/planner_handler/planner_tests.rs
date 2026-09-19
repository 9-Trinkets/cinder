use super::build_planned_turn;
use crate::content::types::{
    ActionDefinition, ContentPack, ItemDefinition, ItemKind, OpeningMenuDefinition, PackMessage,
    PartyOrderKind,
};
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
    )
}

fn plan_command(
    content: &ContentPack,
    state: &WorldState,
    raw_input: &str,
    command: PlayerCommand,
) -> (PlannedTurn, bool) {
    build_planned_turn(
        content,
        AggregatedTurn {
            command: CommandSignal {
                raw_input: raw_input.to_string(),
                command,
            },
            world: WorldSnapshot {
                turn_number: state.turn_number,
                current_room_id: state.current_room_id.clone(),
            },
        },
        state,
        state.turn_number + 1,
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

    let (planned, advances_time) = plan_order(&content, &state, "blair", "follow".to_string());

    assert!(!advances_time);
    assert!(planned.events.iter().any(|event| matches!(
        event,
        WorldEvent::PartyOrderAssigned { actor_id, order }
            if actor_id == "blair" && order == "follow"
    )));
}

#[test]
fn party_order_resolves_an_allied_member_in_another_room() {
    let content = minimal_test_pack();
    let mut state = WorldState::new(&content);
    state.set_stance("blair", ActorStance::Allied);
    state
        .actor_room_overrides
        .insert("blair".to_string(), "other-room".to_string());

    let (planned, advances_time) = plan_order(&content, &state, "blair", "guard".to_string());

    assert!(!advances_time);
    assert!(planned.events.iter().any(|event| matches!(
        event,
        WorldEvent::PartyOrderAssigned { actor_id, order }
            if actor_id == "blair" && order == "guard"
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

#[test]
fn equipment_commands_resolve_stable_item_ids_and_advance_time() {
    let mut content = minimal_test_pack();
    content
        .settings
        .equipment_slots
        .insert("weapon".to_string());
    content.items.push(ItemDefinition {
        id: "iron-chisel".to_string(),
        label: "iron chisel".to_string(),
        kind: ItemKind::Weapon,
        equip_slots: vec!["weapon".to_string()],
        ..ItemDefinition::default()
    });
    rebuild_test_pack_indexes(&mut content);
    let mut state = WorldState::new(&content);
    state.add_item("iron-chisel");

    let (planned, advances_time) = plan_command(
        &content,
        &state,
        "equip chisel",
        PlayerCommand::Equip {
            target: "chisel".to_string(),
        },
    );

    assert!(advances_time);
    assert!(planned.events.iter().any(|event| matches!(
        event,
        WorldEvent::PlayerEquippedItem { item_id } if item_id == "iron-chisel"
    )));
}

#[test]
fn equipment_commands_reject_items_outside_the_valid_source() {
    let mut content = minimal_test_pack();
    content
        .settings
        .equipment_slots
        .insert("weapon".to_string());
    content.messages.insert(
        "equipment.item_not_held".to_string(),
        PackMessage::Narration("Not held: {item}.".to_string()),
    );
    content.items.push(ItemDefinition {
        id: "iron-chisel".to_string(),
        label: "iron chisel".to_string(),
        kind: ItemKind::Weapon,
        equip_slots: vec!["weapon".to_string()],
        ..ItemDefinition::default()
    });
    rebuild_test_pack_indexes(&mut content);
    let state = WorldState::new(&content);

    let (planned, advances_time) = plan_command(
        &content,
        &state,
        "equip iron chisel",
        PlayerCommand::Equip {
            target: "iron-chisel".to_string(),
        },
    );

    assert!(!advances_time);
    assert!(planned.events.iter().any(|event| matches!(
        event,
        WorldEvent::ActionRejected { message } if message == "Not held: iron chisel."
    )));
}

#[test]
fn item_transfers_rejected_when_not_declared_in_pack() {
    let content = minimal_test_pack();
    let state = WorldState::new(&content);

    let (take_planned, take_advances) = plan_command(
        &content,
        &state,
        "take stone",
        PlayerCommand::Take {
            target: "stone".to_string(),
        },
    );
    assert!(!take_advances);
    assert!(take_planned.events.iter().any(|event| matches!(
        event,
        WorldEvent::UnknownInput { raw_input } if raw_input == "take stone"
    )));

    let (drop_planned, drop_advances) = plan_command(
        &content,
        &state,
        "drop stone",
        PlayerCommand::Drop {
            target: "stone".to_string(),
        },
    );
    assert!(!drop_advances);
    assert!(drop_planned.events.iter().any(|event| matches!(
        event,
        WorldEvent::UnknownInput { raw_input } if raw_input == "drop stone"
    )));
}

#[test]
fn item_transfers_accepted_when_declared_in_pack() {
    let mut content = minimal_test_pack();
    content.actions.push(ActionDefinition {
        id: "take".to_string(),
        player_enabled: true,
        ..ActionDefinition::default()
    });
    content.actions.push(ActionDefinition {
        id: "drop".to_string(),
        player_enabled: true,
        ..ActionDefinition::default()
    });
    rebuild_test_pack_indexes(&mut content);
    let state = WorldState::new(&content);

    let (take_planned, take_advances) = plan_command(
        &content,
        &state,
        "take stone",
        PlayerCommand::Take {
            target: "stone".to_string(),
        },
    );
    assert!(!take_advances);
    // Not UnknownInput; item not found in room emits ActionRejected
    assert!(take_planned.events.iter().any(|event| matches!(
        event,
        WorldEvent::ActionRejected { .. }
    )));

    let (drop_planned, drop_advances) = plan_command(
        &content,
        &state,
        "drop stone",
        PlayerCommand::Drop {
            target: "stone".to_string(),
        },
    );
    assert!(!drop_advances);
    // Not UnknownInput; item not held in inventory emits ActionRejected
    assert!(drop_planned.events.iter().any(|event| matches!(
        event,
        WorldEvent::ActionRejected { .. }
    )));
}

#[test]
fn give_to_party_member_plans_event_when_valid() {
    let mut content = minimal_test_pack();
    content.items.push(ItemDefinition {
        id: "iron-chisel".to_string(),
        label: "iron chisel".to_string(),
        ..ItemDefinition::default()
    });
    rebuild_test_pack_indexes(&mut content);

    let mut state = WorldState::new(&content);
    state.set_stance("blair", ActorStance::Allied);
    state.add_item("iron-chisel");

    let (planned, advances_time) = plan_command(
        &content,
        &state,
        "give iron chisel to blair",
        PlayerCommand::GiveToPartyMember {
            item_target: "iron chisel".to_string(),
            actor_reference: "blair".to_string(),
        },
    );

    assert!(advances_time);
    assert!(planned.events.iter().any(|event| matches!(
        event,
        WorldEvent::PlayerGaveItemToPartyMember { actor_id, item_id }
            if actor_id == "blair" && item_id == "iron-chisel"
    )));
}

#[test]
fn take_from_party_member_plans_event_when_held() {
    let mut content = minimal_test_pack();
    content.items.push(ItemDefinition {
        id: "iron-chisel".to_string(),
        label: "iron chisel".to_string(),
        ..ItemDefinition::default()
    });
    rebuild_test_pack_indexes(&mut content);

    let mut state = WorldState::new(&content);
    state.set_stance("blair", ActorStance::Allied);
    state.actor_add_item("blair", "iron-chisel");

    let (planned, advances_time) = plan_command(
        &content,
        &state,
        "take iron chisel from blair",
        PlayerCommand::TakeFromPartyMember {
            item_target: "iron chisel".to_string(),
            actor_reference: "blair".to_string(),
        },
    );

    assert!(advances_time);
    assert!(planned.events.iter().any(|event| matches!(
        event,
        WorldEvent::PlayerTookItemFromPartyMember { actor_id, item_id }
            if actor_id == "blair" && item_id == "iron-chisel"
    )));
}
