use super::super::types::PlannedTurn;
use super::PlanningContext;
use crate::content::types::ContentPack;
use crate::engine::events::{ObservationMode, WorldEvent};
use crate::engine::state::display_actor_name;
use crate::engine::turn_policies::story_var_is_truthy;

pub(super) fn plan_observe_room(context: &PlanningContext<'_>, planned: &mut PlannedTurn) -> bool {
    planned.events.push(WorldEvent::CurrentRoomObserved {
        room_id: context.current_room_id.to_string(),
        mode: ObservationMode::Detailed,
    });
    false
}

pub(super) fn plan_observe_target(
    content: &ContentPack,
    target: &str,
    context: &PlanningContext<'_>,
    planned: &mut PlannedTurn,
) -> bool {
    if let Some(actor) = content.resolve_actor(target).or_else(|| {
        content.actors.iter().find(|actor| {
            display_actor_name(context.planner_state, actor).eq_ignore_ascii_case(target)
        })
    }) {
        let actor_name = display_actor_name(context.planner_state, actor);
        if context
            .planner_state
            .actor_room_id(&actor.id, &actor.room_id)
            == context.current_room_id
        {
            planned.events.push(WorldEvent::ActorObserved {
                actor_id: actor.id.clone(),
            });
        } else {
            planned.events.push(WorldEvent::ActionRejected {
                message: content.render_template(
                    &content.presentation.error_text.actor_not_here,
                    &[("actor_name", actor_name.as_str())],
                ),
            });
        }
    } else if let Some(feature) = content.resolve_feature_in_room(context.current_room_id, target) {
        planned.events.push(WorldEvent::FeatureObserved {
            room_id: context.current_room_id.to_string(),
            feature_id: feature.id.clone(),
        });
    } else if let Some(item) =
        content.resolve_item_in_scope(context.planner_state, context.current_room_id, target)
    {
        planned.events.push(WorldEvent::ItemObserved {
            item_id: item.id.clone(),
        });
    } else {
        planned.events.push(WorldEvent::ActionRejected {
            message: content.render_template(
                &content.presentation.error_text.feature_unknown,
                &[("target", target)],
            ),
        });
    }

    false
}

pub(super) fn plan_move_to_room_target(
    content: &ContentPack,
    target: &str,
    advances_time: bool,
    context: &PlanningContext<'_>,
    planned: &mut PlannedTurn,
) -> bool {
    if let Some(exit) = content.resolve_exit_for(context.current_room_id, target, |key| {
        story_var_is_truthy(context.planner_state, key)
    }) {
        planned.events.push(WorldEvent::PlayerMoved {
            from_room_id: context.current_room_id.to_string(),
            to_room_id: exit.room_id.clone(),
        });
        planned.events.push(WorldEvent::CurrentRoomObserved {
            room_id: exit.room_id.clone(),
            mode: ObservationMode::Summary,
        });
        advances_time
    } else {
        planned.events.push(WorldEvent::ActionRejected {
            message: content.render_template(
                &content.presentation.error_text.cannot_go,
                &[("target", target)],
            ),
        });
        false
    }
}
