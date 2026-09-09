use super::common::*;
use cinder_core::content::types::{
    ChannelAvailability, ChannelKind, ChannelPrivacy, ContentPack, MessagingChannel, PackMessage,
};
use cinder_core::engine::events::{TimestampedWorldEvent, WorldEvent};
use cinder_core::engine::narrative::NarrativeLineKind;
use cinder_core::engine::reducer::apply_events;
use cinder_core::engine::state::WorldState;
use std::collections::BTreeMap;

/// A pack with an explicit handler commentary channel and messages covering
/// handler, system, and narration delivery.
pub fn handler_feedback_pack() -> ContentPack {
    let mut pack = reducer_test_pack();
    pack.settings.channels.push(MessagingChannel {
        id: "handler-comms".to_string(),
        kind: ChannelKind::Direct,
        privacy: ChannelPrivacy::Private,
        availability: ChannelAvailability::Always,
        participants: vec!["player".to_string(), "handler".to_string()],
        label: None,
    });
    pack.settings.feedback_channel_id = "handler-comms".to_string();
    pack.actors
        .push(test_actor("handler", "Handler", LOUNGE_ID));
    pack.messages = BTreeMap::from([
        (
            "item.acquired_inventory".to_string(),
            PackMessage::Voiced {
                voice: cinder_core::content::types::PackMessageVoice::Handler,
                text: "{label} logged as ready.".to_string(),
            },
        ),
        (
            "item.acquired_room".to_string(),
            PackMessage::Voiced {
                voice: cinder_core::content::types::PackMessageVoice::System,
                text: "OBJECT REGISTERED: {label}.".to_string(),
            },
        ),
        (
            "item.consumed_use".to_string(),
            PackMessage::Narration("The {label} registered as used.".to_string()),
        ),
    ]);
    rebuild_test_pack_indexes(&mut pack);
    pack
}

#[test]
fn action_rejected_stays_automated_when_handler_channel_is_declared() {
    let pack = handler_feedback_pack();
    let mut state = WorldState::new(&pack);

    let output = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::ActionRejected {
            message: "ACTION UNAVAILABLE.".to_string(),
        })],
    );

    let line = output
        .lines
        .iter()
        .find(|line| line.text == "ACTION UNAVAILABLE.")
        .expect("the automated error should remain visible");
    assert_eq!(line.kind, NarrativeLineKind::Error);
}

#[test]
fn action_rejected_stays_error_without_a_feedback_channel() {
    let pack = reducer_test_pack();
    let mut state = WorldState::new(&pack);

    let output = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::ActionRejected {
            message: "Not right now.".to_string(),
        })],
    );

    let line = output
        .lines
        .iter()
        .find(|line| line.text == "Not right now.")
        .expect("the error should still narrate");
    assert_eq!(line.kind, NarrativeLineKind::Error);
}

#[test]
fn handler_voiced_messages_render_as_handler_channel_lines() {
    let mut pack = handler_feedback_pack();
    pack.items = vec![cinder_core::content::types::ItemDefinition {
        id: "coffee".to_string(),
        label: "coffee".to_string(),
        description: "Fresh coffee.".to_string(),
        ..cinder_core::content::types::ItemDefinition::default()
    }];
    rebuild_test_pack_indexes(&mut pack);
    let mut state = WorldState::new(&pack);

    let output = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::ItemAcquired {
            item_id: "coffee".to_string(),
            storage: cinder_core::content::types::ItemStorageTarget::PlayerInventory,
        })],
    );

    assert!(
        output
            .lines
            .iter()
            .any(|line| line.kind == NarrativeLineKind::Channel
                && line.text == "Handler: coffee logged as ready."),
        "handler-voiced messages should be attributed to the handler: {:?}",
        output.lines
    );
}

#[test]
fn system_voiced_messages_render_as_system_lines() {
    let mut pack = handler_feedback_pack();
    pack.items = vec![cinder_core::content::types::ItemDefinition {
        id: "coffee".to_string(),
        label: "coffee".to_string(),
        description: "Fresh coffee.".to_string(),
        ..cinder_core::content::types::ItemDefinition::default()
    }];
    rebuild_test_pack_indexes(&mut pack);
    let mut state = WorldState::new(&pack);

    let output = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::ItemAcquired {
            item_id: "coffee".to_string(),
            storage: cinder_core::content::types::ItemStorageTarget::CurrentRoom,
        })],
    );

    let line = output
        .lines
        .iter()
        .find(|line| line.text == "OBJECT REGISTERED: coffee.")
        .expect("system feedback should be emitted");
    assert_eq!(line.kind, NarrativeLineKind::System);
}

#[test]
fn world_narration_messages_ignore_handler_attribution() {
    let mut pack = handler_feedback_pack();
    pack.items = vec![cinder_core::content::types::ItemDefinition {
        id: "coffee".to_string(),
        label: "coffee".to_string(),
        description: "Fresh coffee.".to_string(),
        ..cinder_core::content::types::ItemDefinition::default()
    }];
    rebuild_test_pack_indexes(&mut pack);
    let mut state = WorldState::new(&pack);
    state.current_room_id = KITCHEN_ID.to_string();

    apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::ItemAcquired {
            item_id: "coffee".to_string(),
            storage: cinder_core::content::types::ItemStorageTarget::CurrentRoom,
        })],
    );
    let output = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::ItemConsumed {
            item_id: "coffee".to_string(),
            storage: cinder_core::content::types::ItemStorageTarget::CurrentRoom,
            consumer_id: None,
            consumer_name: None,
        })],
    );

    let line = output
        .lines
        .iter()
        .find(|line| line.text == "The coffee registered as used.")
        .expect("consumption should narrate its message");
    assert_eq!(line.kind, NarrativeLineKind::Narration);
}
