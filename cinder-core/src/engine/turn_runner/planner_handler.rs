use super::planning::{PlanningContext, plan_authored_command};
use super::types::{AggregatedTurn, PlannedTurn, RouteEnvelope};
use crate::content::types::ContentPack;
use crate::engine::commands::PlayerCommand;
use crate::engine::events::WorldEvent;
use crate::engine::menus::{
    build_menu_choice_events, resolve_menu_choice, resolve_menu_choice_in_options,
};
use crate::engine::state::{WorldState, render_dynamic_story_text};

fn try_resolve_menu_choice(
    content: &ContentPack,
    planner_state: &WorldState,
    raw_input: &str,
) -> Option<(Vec<WorldEvent>, bool)> {
    if let Some(menu_id) = planner_state.active_menu_id.as_deref()
        && let Some(menu) = content.menu(menu_id)
    {
        let raw = raw_input.trim();
        let is_multi_select = menu.max_selections > 0;
        if is_multi_select && raw == "done" {
            if menu.min_selections > 0
                && planner_state.pending_menu_selections.len() < menu.min_selections
            {
                return Some((
                    vec![WorldEvent::ActionRejected {
                        message: render_dynamic_story_text(&menu.invalid_choice_text, planner_state),
                    }],
                    false,
                ));
            }
            return Some((
                vec![WorldEvent::MenuChoiceMade {
                    menu_id: menu_id.to_string(),
                    option_id: "done".to_string(),
                    title: "Done".to_string(),
                }],
                true,
            ));
        }
        if is_multi_select && raw.starts_with("toggle:") {
            let option_id = raw["toggle:".len()..].to_string();
            let selected = !planner_state.pending_menu_selections.contains(&option_id);
            return Some((
                vec![WorldEvent::MenuSelectionToggled {
                    menu_id: menu_id.to_string(),
                    option_id,
                    selected,
                }],
                false,
            ));
        }
        let option = planner_state
            .generated_menu_options
            .get(menu_id)
            .and_then(|options| resolve_menu_choice_in_options(options, raw_input))
            .or_else(|| resolve_menu_choice(menu, raw_input));
        if let Some(option) = option {
            return Some((
                build_menu_choice_events(content, planner_state, menu, option),
                true,
            ));
        }
    }

    planner_state
        .active_objective_stage_ids
        .iter()
        .find_map(|stage_id| {
            let menu = content
                .menus
                .iter()
                .find(|menu| menu.stage_id == *stage_id && !menu.options.is_empty())?;
            let option = resolve_menu_choice(menu, raw_input)?;
            Some((
                build_menu_choice_events(content, planner_state, menu, option),
                true,
            ))
        })
}

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
            PlayerCommand::Help => {
                planned.events.push(WorldEvent::HelpShown);
                false
            }
            PlayerCommand::Quit => {
                planned.events.push(WorldEvent::ActEnded);
                false
            }
            PlayerCommand::Unknown => {
                if let Some(menu_id) = planner_state.active_menu_id.as_deref() {
                    if let Some(menu) = content.menu(menu_id) {
                        let raw = aggregated.command.raw_input.trim();
                        let is_multi_select = menu.max_selections > 0;
                        if is_multi_select && raw == "done" {
                            if menu.min_selections > 0
                                && planner_state.pending_menu_selections.len()
                                    < menu.min_selections
                            {
                                planned.events.push(WorldEvent::ActionRejected {
                                    message: render_dynamic_story_text(
                                        &menu.invalid_choice_text,
                                        planner_state,
                                    ),
                                });
                                false
                            } else {
                                let option_id = "done".to_string();
                                let title = "Done".to_string();
                                planned.events.push(WorldEvent::MenuChoiceMade {
                                    menu_id: menu_id.to_string(),
                                    option_id,
                                    title,
                                });
                                true
                            }
                        } else if is_multi_select && raw.starts_with("toggle:") {
                            let option_id = raw["toggle:".len()..].to_string();
                            let selected =
                                !planner_state.pending_menu_selections.contains(&option_id);
                            planned.events.push(WorldEvent::MenuSelectionToggled {
                                menu_id: menu_id.to_string(),
                                option_id,
                                selected,
                            });
                            false
                        } else {
                            let option = planner_state
                                .generated_menu_options
                                .get(menu_id)
                                .and_then(|options| {
                                    resolve_menu_choice_in_options(
                                        options,
                                        &aggregated.command.raw_input,
                                    )
                                })
                                .or_else(|| {
                                    resolve_menu_choice(menu, &aggregated.command.raw_input)
                                });
                            if let Some(option) = option {
                                planned.events.extend(build_menu_choice_events(
                                    content,
                                    planner_state,
                                    menu,
                                    option,
                                ));
                                true
                            } else {
                                planned.events.push(WorldEvent::ActionRejected {
                                    message: render_dynamic_story_text(
                                        &menu.invalid_choice_text,
                                        planner_state,
                                    ),
                                });
                                false
                            }
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
                        planned.events.extend(build_menu_choice_events(
                            content,
                            planner_state,
                            stage_menu,
                            option,
                        ));
                        true
                    } else {
                        planned.events.push(WorldEvent::ActionRejected {
                            message: render_dynamic_story_text(
                                &stage_menu.invalid_choice_text,
                                planner_state,
                            ),
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
