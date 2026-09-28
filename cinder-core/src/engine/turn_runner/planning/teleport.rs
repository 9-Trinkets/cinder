use super::PlanningContext;
use crate::content::types::{ActionDefinition, ContentPack};
use crate::engine::events::{ObservationMode, WorldEvent};
use crate::engine::turn_runner::PlannedTurn;

pub(super) fn plan_teleport_command(
    content: &ContentPack,
    action: &ActionDefinition,
    input: Option<&str>,
    context: &PlanningContext<'_>,
    planned: &mut PlannedTurn,
) -> bool {
    let state = context.planner_state;
    if state.story_vars.get("knows_teleport") != Some("true") {
        planned.events.push(WorldEvent::ActionRejected {
            message: "You have not learned how to teleport yet.".to_string(),
        });
        return false;
    }

    if !content.has_any_teleport_destination(state) {
        planned.events.push(WorldEvent::ActionRejected {
            message: "You do not have any active teleport anchors yet. Trace a teleport sigil in a room to create a chalk anchor, or visit a teleport platform.".to_string(),
        });
        return false;
    }

    let Some(raw_target) = input.map(str::trim).filter(|s| !s.is_empty()) else {
        planned.events.push(WorldEvent::ActionRejected {
            message: "Specify an anchor to teleport to. (e.g. teleport teleport_platform)"
                .to_string(),
        });
        return false;
    };

    let target_lower = raw_target.to_ascii_lowercase();

    let Some(anchor) = content.resolve_teleport_target(state, &target_lower) else {
        planned.events.push(WorldEvent::ActionRejected {
            message: format!(
                "Unknown anchor '{raw_target}'. Open your Teleport panel to view available anchors."
            ),
        });
        return false;
    };
    let (dest_room_id, is_permanent) = anchor;

    if dest_room_id == context.current_room_id {
        planned.events.push(WorldEvent::ActionRejected {
            message: "You are already standing at this anchor.".to_string(),
        });
        return false;
    }

    if is_permanent {
        planned.events.push(WorldEvent::NarrativeLine {
            text: "The brass platform glows with blue light. The air blurs around you, and you step out at your destination."
                .to_string(),
        });
    } else {
        planned.events.push(WorldEvent::NarrativeLine {
            text: "Bright light flares from the chalk. You appear at the anchor, and the chalk mark fades away."
                .to_string(),
        });
        planned.events.push(WorldEvent::ChalkAnchorConsumed {
            room_id: dest_room_id.to_string(),
        });
    }

    planned.events.push(WorldEvent::PlayerMoved {
        from_room_id: context.current_room_id.to_string(),
        to_room_id: dest_room_id.to_string(),
    });
    planned.events.push(WorldEvent::CurrentRoomObserved {
        room_id: dest_room_id.to_string(),
        mode: ObservationMode::Summary,
    });

    let metadata = action.player_command.as_ref();
    metadata.is_none_or(|m| m.advances_time)
}
