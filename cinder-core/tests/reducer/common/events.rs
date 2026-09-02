use cinder_core::content::types::ContentPack;
use cinder_core::engine::events::{TimestampedWorldEvent, WorldEvent};
use cinder_core::engine::reducer::{ReducerOutput, apply_events};
use cinder_core::engine::state::WorldState;

#[allow(clippy::too_many_arguments)]
pub fn drive_actor_command(
    state: &mut WorldState,
    pack: &ContentPack,
    actor_id: &str,
    actor_name: &str,
    room_id: &str,
    command_id: &str,
    target_room_id: Option<&str>,
    target_actor_id: Option<&str>,
    target_actor_name: Option<&str>,
    context_label: Option<&str>,
    feature_id: Option<&str>,
    consumable_id: Option<&str>,
    freeform_text: Option<&str>,
) -> ReducerOutput {
    apply_events(
        state,
        pack,
        &[TimestampedWorldEvent::now(WorldEvent::ActorCommandUsed {
            actor_id: actor_id.to_string(),
            actor_name: actor_name.to_string(),
            room_id: room_id.to_string(),
            command_id: command_id.to_string(),
            target_room_id: target_room_id.map(str::to_string),
            target_actor_id: target_actor_id.map(str::to_string),
            target_actor_name: target_actor_name.map(str::to_string),
            context_label: context_label.map(str::to_string),
            feature_id: feature_id.map(str::to_string),
            consumable_id: consumable_id.map(str::to_string),
            freeform_text: freeform_text.map(str::to_string),
        })],
    )
}
