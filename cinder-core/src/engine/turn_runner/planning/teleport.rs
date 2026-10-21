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
            message: content
                .render_message("teleport.not_learned", &[])
                .unwrap_or_default(),
        });
        return false;
    }

    if !content.has_any_teleport_destination(state) {
        planned.events.push(WorldEvent::ActionRejected {
            message: content
                .render_message("teleport.no_anchors", &[])
                .unwrap_or_default(),
        });
        return false;
    }

    let Some(raw_target) = input.map(str::trim).filter(|s| !s.is_empty()) else {
        planned.events.push(WorldEvent::ActionRejected {
            message: content
                .render_message("teleport.specify_anchor", &[])
                .unwrap_or_default(),
        });
        return false;
    };

    let target_lower = raw_target.to_ascii_lowercase();

    let Some(anchor) = content.resolve_teleport_target(state, &target_lower) else {
        planned.events.push(WorldEvent::ActionRejected {
            message: content
                .render_message("teleport.unknown_anchor", &[("target", raw_target)])
                .unwrap_or_default(),
        });
        return false;
    };
    let (dest_room_id, is_permanent) = anchor;

    if dest_room_id == context.current_room_id {
        planned.events.push(WorldEvent::ActionRejected {
            message: content
                .render_message("teleport.already_here", &[])
                .unwrap_or_default(),
        });
        return false;
    }

    let player_id = &content.settings.combat.player_actor_id;
    let trace_lvl = state.actor_skill_level(player_id, "trace");
    let mp_cost = crate::engine::skills::effective_trace_mp_cost_for_pack(
        content,
        crate::engine::skills::TELEPORT_BASE_MP_COST,
        trace_lvl,
    );
    if content.stats.actor.contains_key("mp") {
        let current_mp = state.actor_stat_u32(player_id, "mp");
        if current_mp < mp_cost {
            planned.events.push(WorldEvent::ActionRejected {
                message: content
                    .render_message(
                        "teleport.insufficient_mp",
                        &[
                            ("mp_cost", &mp_cost.to_string()),
                            ("current_mp", &current_mp.to_string()),
                        ],
                    )
                    .unwrap_or_default(),
            });
            return false;
        }
        planned.events.push(WorldEvent::ActorStatAdjusted {
            actor_id: player_id.to_string(),
            stat: "mp".to_string(),
            delta: -(mp_cost as i32),
        });
    }

    planned.events.push(WorldEvent::SkillSpAwarded {
        actor_id: player_id.to_string(),
        skill_id: "teleport".to_string(),
        sp: 5,
    });

    if is_permanent {
        planned.events.push(WorldEvent::NarrativeLine {
            text: content
                .render_message("teleport.platform_arrival", &[])
                .unwrap_or_default(),
        });
    } else {
        planned.events.push(WorldEvent::NarrativeLine {
            text: content
                .render_message("teleport.chalk_arrival", &[])
                .unwrap_or_default(),
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
