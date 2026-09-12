//! Builds a [`PlannedTurn`] from parsed input. The heavy lifting is split by
//! concern: authored commands live in `super::planning`, and the fallback
//! planners (items, party orders, unknown-menu fallbacks) live beside this
//! file.

mod items;
mod menus;
mod party;
#[cfg(test)]
mod planner_tests;

use self::items::{plan_drop_command, plan_take_command};
use self::menus::{plan_unknown_command, try_resolve_menu_choice};
use self::party::plan_party_order;
use super::planning::{PlanningContext, plan_authored_command};
use super::types::{AggregatedTurn, PlannedTurn, RouteEnvelope};
use crate::content::types::ContentPack;
use crate::engine::commands::PlayerCommand;
use crate::engine::events::WorldEvent;
use crate::engine::scripted::advance_scripted_sequences;
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
    let advances_time = if let Some((events, advances_time)) =
        try_resolve_menu_choice(content, planner_state, &aggregated.command.raw_input)
    {
        planned.events.extend(events);
        advances_time
    } else {
        match aggregated.command.command {
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
            PlayerCommand::Take { target } => plan_take_command(
                content,
                planner_state,
                &aggregated.world.current_room_id,
                &target,
                &mut planned,
            ),
            PlayerCommand::Drop { target } => {
                plan_drop_command(content, planner_state, &target, &mut planned)
            }
            PlayerCommand::PartyOrder {
                actor_reference,
                order,
            } => plan_party_order(
                content,
                planner_state,
                &aggregated.world.current_room_id,
                &actor_reference,
                order,
                &mut planned,
            ),
            PlayerCommand::Help => {
                planned.events.push(WorldEvent::HelpShown);
                false
            }
            PlayerCommand::Quit => {
                planned.events.push(WorldEvent::ActEnded);
                false
            }
            PlayerCommand::Unknown => plan_unknown_command(
                content,
                planner_state,
                &aggregated.command.raw_input,
                &mut planned,
            ),
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
    // Scripted sequences play one line per advancing turn, after the command's
    // own events resolve. The planner emits the step's content event plus the
    // playhead commit; the reducer advances the sequence.
    if advances_time {
        planned
            .events
            .extend(advance_scripted_sequences(content, planner_state));
    }
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