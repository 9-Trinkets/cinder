use cinder_core::content::types::ContentPack;
use cinder_core::engine::events::{TimestampedWorldEvent, WorldEvent};
use cinder_core::engine::reducer::{ReducerOutput, apply_events};
use cinder_core::engine::state::WorldState;

#[derive(Default)]
pub struct ActorCommandInput<'a> {
    pub actor_id: &'a str,
    pub actor_name: &'a str,
    pub room_id: &'a str,
    pub target_room_id: Option<&'a str>,
    pub target_actor_id: Option<&'a str>,
    pub target_actor_name: Option<&'a str>,
    pub context_label: Option<&'a str>,
    pub feature_id: Option<&'a str>,
    pub consumable_id: Option<&'a str>,
    pub freeform_text: Option<&'a str>,
}

pub fn drive_actor_command(
    state: &mut WorldState,
    pack: &ContentPack,
    command_id: &str,
    input: ActorCommandInput<'_>,
) -> ReducerOutput {
    apply_events(
        state,
        pack,
        &[TimestampedWorldEvent::now(WorldEvent::ActorCommandUsed {
            actor_id: input.actor_id.to_string(),
            actor_name: input.actor_name.to_string(),
            room_id: input.room_id.to_string(),
            command_id: command_id.to_string(),
            target_room_id: input.target_room_id.map(str::to_string),
            target_actor_id: input.target_actor_id.map(str::to_string),
            target_actor_name: input.target_actor_name.map(str::to_string),
            context_label: input.context_label.map(str::to_string),
            feature_id: input.feature_id.map(str::to_string),
            consumable_id: input.consumable_id.map(str::to_string),
            freeform_text: input.freeform_text.map(str::to_string),
        })],
    )
}
