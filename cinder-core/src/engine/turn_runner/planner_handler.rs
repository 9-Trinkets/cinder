use super::planning::{PlanningContext, plan_authored_command};
use super::types::{AggregatedTurn, PlannedTurn, RouteEnvelope};
use crate::content::types::ContentPack;
use crate::engine::commands::PlayerCommand;
use crate::engine::events::WorldEvent;
use crate::engine::menus::{build_menu_choice_events, resolve_menu_choice};
use crate::engine::state::WorldState;

pub(super) fn build_planned_turn(
    content: &ContentPack,
    aggregated: AggregatedTurn,
    planner_state: &WorldState,
    turn_number: u32,
    channel_surfing_only: bool,
) -> (PlannedTurn, bool) {
    let mut planned = PlannedTurn {
        events: vec![],
        pending_dialogue: None,
        grounded_dialogue: None,
    };
    let advances_time = match aggregated.command.command {
        PlayerCommand::Authored { command_id, input } => plan_authored_command(
            content,
            &command_id,
            input.as_deref(),
            PlanningContext {
                raw_input: &aggregated.command.raw_input,
                current_room_id: &aggregated.world.current_room_id,
                planner_state,
                channel_surfing_only,
                turn_number,
            },
            &mut planned,
        ),
        PlayerCommand::Unknown if channel_surfing_only => {
            planned.events.push(WorldEvent::UnknownInput {
                raw_input: aggregated.command.raw_input.clone(),
            });
            false
        }
        PlayerCommand::Help => {
            planned.events.push(WorldEvent::HelpShown);
            false
        }
        PlayerCommand::Quit => {
            planned.events.push(WorldEvent::SessionEnded);
            false
        }
        PlayerCommand::Unknown => {
            if let Some(menu_id) = planner_state.active_menu_id.as_deref() {
                if let Some(menu) = content.menu(menu_id) {
                    if let Some(option) = resolve_menu_choice(menu, &aggregated.command.raw_input) {
                        planned
                            .events
                            .extend(build_menu_choice_events(content, menu, option));
                        true
                    } else {
                        planned.events.push(WorldEvent::ActionRejected {
                            message: menu.invalid_choice_text.clone(),
                        });
                        false
                    }
                } else {
                    planned.events.push(WorldEvent::ActionRejected {
                        message: content.ui_text.menu_unavailable.clone(),
                    });
                    false
                }
            } else if let Some(stage_menu) = planner_state
                .active_objective_stage_ids
                .iter()
                .find_map(|stage_id| {
                    content
                        .menus
                        .iter()
                        .find(|m| m.stage_id == *stage_id && !m.options.is_empty())
                })
            {
                if let Some(option) =
                    resolve_menu_choice(stage_menu, &aggregated.command.raw_input)
                {
                    planned
                        .events
                        .extend(build_menu_choice_events(content, stage_menu, option));
                    true
                } else {
                    planned.events.push(WorldEvent::ActionRejected {
                        message: stage_menu.invalid_choice_text.clone(),
                    });
                    false
                }
            } else {
                planned.events.push(WorldEvent::UnknownInput {
                    raw_input: aggregated.command.raw_input.clone(),
                });
                false
            }
        }
    };
    planned.events.insert(
        0,
        WorldEvent::TurnStarted {
            turn_number,
            raw_input: aggregated.command.raw_input.clone(),
            advances_time,
        },
    );
    (planned, advances_time)
}

pub(super) fn resolve_next_role(
    planned: &PlannedTurn,
    next_menu_intent: impl FnOnce() -> Result<String, String>,
    next_reducer: impl FnOnce() -> Result<String, String>,
) -> Result<RouteEnvelope, String> {
    let next = if planned.pending_dialogue.is_some() {
        next_menu_intent()?
    } else {
        next_reducer()?
    };
    Ok(RouteEnvelope {
        next,
        message: serde_json::to_string(planned).map_err(|error| error.to_string())?,
    })
}
