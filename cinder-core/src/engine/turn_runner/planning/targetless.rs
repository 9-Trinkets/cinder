use super::super::types::PlannedTurn;
use super::PlanningContext;
use crate::content::types::{ActionDefinition, ContentPack};
use crate::engine::events::WorldEvent;
use crate::engine::turn_policies::{command_availability_issue, command_unavailable_message};

pub(super) fn plan_targetless_command(
    content: &ContentPack,
    action: &ActionDefinition,
    context: &PlanningContext<'_>,
    planned: &mut PlannedTurn,
) -> bool {
    if let Some(issue) = command_availability_issue(content, context.planner_state, action) {
        planned.events.push(WorldEvent::ActionRejected {
            message: command_unavailable_message(content, action, &issue),
        });
        return false;
    }
    let metadata = action
        .player_command
        .as_ref()
        .unwrap_or_else(|| panic!("action '{}' should define player_command", action.id));
    let room_id = context.current_room_id.to_string();
    let actor_name = content.opening.title.as_str();
    planned.events.push(WorldEvent::ActorCommandUsed {
        actor_id: content.settings.combat.player_actor_id.clone(),
        actor_name: actor_name.to_string(),
        room_id,
        command_id: action.id.clone(),
        target_room_id: None,
        target_actor_id: None,
        target_actor_name: None,
        context_label: None,
        feature_id: None,
        consumable_id: None,
        freeform_text: None,
    });
    if metadata.advances_time {
        planned.events.push(WorldEvent::TurnStarted {
            turn_number: context.turn_number,
            raw_input: action.id.clone(),
            advances_time: true,
        });
    }
    true
}
