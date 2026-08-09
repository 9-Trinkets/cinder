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
                            let option_id = "done".to_string();
                            let title = "Done".to_string();
                            planned.events.push(WorldEvent::MenuChoiceMade {
                                menu_id: menu_id.to_string(),
                                option_id,
                                title,
                            });
                            true
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

#[cfg(test)]
mod tests {
    use super::{AggregatedTurn, build_planned_turn};
    use crate::content::loader::load_named_pack;
    use crate::engine::commands::parse_command;
    use crate::engine::events::TimestampedWorldEvent;
    use crate::engine::events::WorldEvent;
    use crate::engine::reducer::apply_events;
    use crate::engine::state::WorldSnapshot;
    use crate::engine::state::WorldState;
    use crate::engine::turn_runner::types::CommandSignal;

    #[test]
    fn stage_menu_choice_wins_over_colliding_authored_command() {
        let content = load_named_pack("isla", Some("en")).expect("load isla");
        let mut state = WorldState::new(&content);
        state.current_room_id = "kitchen".to_string();
        state.active_objective_stage_ids = vec!["request-quarter".to_string()];

        let aggregated = AggregatedTurn {
            command: CommandSignal {
                raw_input: "serve coffee".to_string(),
                command: parse_command(&content, "serve coffee"),
            },
            world: WorldSnapshot {
                turn_number: 0,
                current_room_id: "kitchen".to_string(),
            },
        };

        let (planned, advances_time) = build_planned_turn(&content, aggregated, &state, 1, false);

        assert!(advances_time);
        assert!(planned.events.iter().any(|event| matches!(
            event,
            WorldEvent::MenuChoiceMade {
                menu_id,
                option_id,
                ..
            } if menu_id == "request-quarter" && option_id == "quarter-coffee"
        )));
        assert!(
            !planned
                .events
                .iter()
                .any(|event| matches!(event, WorldEvent::ActionRejected { .. }))
        );
    }

    #[test]
    fn isla_brewing_coffee_enables_serving_during_intake() {
        let content = load_named_pack("isla", Some("en")).expect("load isla");
        let mut state = WorldState::new(&content);
        state.current_room_id = "kitchen".to_string();
        state.active_objective_stage_ids = vec!["intake".to_string()];

        let brew = AggregatedTurn {
            command: CommandSignal {
                raw_input: "brew coffee".to_string(),
                command: parse_command(&content, "brew coffee"),
            },
            world: WorldSnapshot {
                turn_number: 0,
                current_room_id: "kitchen".to_string(),
            },
        };

        let (planned_brew, brew_advances_time) =
            build_planned_turn(&content, brew, &state, 1, false);

        assert!(brew_advances_time);
        assert!(planned_brew.events.iter().any(|event| matches!(
            event,
            WorldEvent::CommandBundleProgressApplied { command_id } if command_id == "brew_coffee"
        )));

        let brew_events = planned_brew
            .events
            .into_iter()
            .map(TimestampedWorldEvent::now)
            .collect::<Vec<_>>();
        apply_events(&mut state, &content, &brew_events);

        assert!(state.has_item("coffee"));
        assert_eq!(
            state
                .story_vars
                .get("rule_bundle:progress:reading-service-ritual:coffee_ready"),
            Some("true")
        );

        state.current_room_id = "cafe".to_string();
        let serve = AggregatedTurn {
            command: CommandSignal {
                raw_input: "serve coffee".to_string(),
                command: parse_command(&content, "serve coffee"),
            },
            world: WorldSnapshot {
                turn_number: 1,
                current_room_id: "cafe".to_string(),
            },
        };

        let (planned_serve, serve_advances_time) =
            build_planned_turn(&content, serve, &state, 2, false);

        assert!(!serve_advances_time);
        assert!(planned_serve.events.iter().any(|event| matches!(
            event,
            WorldEvent::ContentEvent { event_id, .. } if event_id == "isla.coffee_served"
        )));
        assert!(planned_serve.events.iter().any(|event| matches!(
            event,
            WorldEvent::ItemConsumed { item_id, .. } if item_id == "coffee"
        )));
        assert!(
            !planned_serve
                .events
                .iter()
                .any(|event| matches!(event, WorldEvent::ActionRejected { .. }))
        );
    }
}
