//! Item reducer tests grouped by driver: inventory/room transfers and
//! action-driven item creation (cook, capture, trace).

mod creation;
mod transfer;

use super::common::*;
use cinder_core::content::types::ItemDefinition;

/// Binds a raw command input to any actor/room with a (nullable) freeform text.
fn command(
    actor_id: &'static str,
    actor_name: &'static str,
    room_id: &'static str,
    freeform_text: Option<&'static str>,
) -> ActorCommandInput<'static> {
    ActorCommandInput {
        actor_id,
        actor_name,
        room_id,
        target_room_id: None,
        target_actor_id: None,
        target_actor_name: None,
        context_label: None,
        feature_id: None,
        consumable_id: None,
        freeform_text,
    }
}

/// Binds a raw command input to the default player actor.
fn plain_command(
    room_id: &'static str,
    freeform_text: Option<&'static str>,
) -> ActorCommandInput<'static> {
    command(ACTOR_A_ID, ACTOR_A_NAME, room_id, freeform_text)
}

/// Builds a minimal item definition with the given id/label/description.
fn item(id: &str, label: &str, description: &str) -> ItemDefinition {
    ItemDefinition {
        id: id.to_string(),
        label: label.to_string(),
        description: description.to_string(),
        ..ItemDefinition::default()
    }
}

/// Builds a minimal trace-mark item definition (anchored in its room).
fn trace_item(id: &str, label: &str, description: &str) -> ItemDefinition {
    ItemDefinition {
        trace_mark: true,
        ..item(id, label, description)
    }
}