use super::PlanningContext;
use crate::content::types::{ActionDefinition, ContentPack};
use crate::engine::events::{ObservationMode, WorldEvent};
use crate::engine::state::WorldState;
use crate::engine::turn_runner::PlannedTurn;

fn resolve_permanent_anchor(state: &WorldState, target: &str) -> Option<&'static str> {
    let perm_floor4 = state.story_vars.get("anchor_floor4_platform") == Some(&"true".to_string());
    if perm_floor4
        && matches!(
            target,
            "floor4_platform" | "teleport_platform" | "floor 4 platform" | "floor 4"
        )
    {
        return Some("teleport_platform");
    }

    let perm_floor5 = state.story_vars.get("anchor_floor5_gate") == Some(&"true".to_string());
    if perm_floor5
        && matches!(
            target,
            "floor5_start"
                | "floor5_gate"
                | "floor 5 descent platform"
                | "floor 5 platform"
                | "floor 5"
        )
    {
        return Some("floor5_start");
    }

    None
}

fn resolve_chalk_anchor<'a>(
    content: &'a ContentPack,
    state: &'a WorldState,
    target: &str,
) -> Option<&'a str> {
    state.chalk_anchors.iter().find_map(|room_id| {
        if room_id.eq_ignore_ascii_case(target) {
            return Some(room_id.as_str());
        }
        let title_matches = content
            .room(room_id)
            .is_some_and(|r| r.title.eq_ignore_ascii_case(target));
        if title_matches {
            Some(room_id.as_str())
        } else {
            None
        }
    })
}

pub(super) fn plan_teleport_command(
    content: &ContentPack,
    action: &ActionDefinition,
    input: Option<&str>,
    context: &PlanningContext<'_>,
    planned: &mut PlannedTurn,
) -> bool {
    let state = context.planner_state;
    if state.story_vars.get("knows_teleport") != Some(&"true".to_string()) {
        planned.events.push(WorldEvent::ActionRejected {
            message: "You have not learned how to teleport yet.".to_string(),
        });
        return false;
    }

    if !state.has_any_teleport_anchor() {
        planned.events.push(WorldEvent::ActionRejected {
            message: "You do not have any active teleport anchors yet. Trace a teleport sigil in a room to create a chalk anchor, or visit a teleport platform.".to_string(),
        });
        return false;
    }

    let Some(raw_target) = input.map(str::trim).filter(|s| !s.is_empty()) else {
        planned.events.push(WorldEvent::ActionRejected {
            message: "Specify an anchor to teleport to. (e.g. teleport teleport_platform)".to_string(),
        });
        return false;
    };

    let target_lower = raw_target.to_ascii_lowercase();

    let (dest_room_id, is_permanent) =
        if let Some(room_id) = resolve_permanent_anchor(state, &target_lower) {
            (room_id, true)
        } else if let Some(room_id) = resolve_chalk_anchor(content, state, &target_lower) {
            (room_id, false)
        } else {
            planned.events.push(WorldEvent::ActionRejected {
                message: format!(
                    "Unknown anchor '{raw_target}'. Open your Teleport panel to view available anchors."
                ),
            });
            return false;
        };

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
