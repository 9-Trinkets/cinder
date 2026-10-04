use crate::content::types::ContentPack;
use crate::engine::events::WorldEvent;
use crate::engine::menus::{
    build_menu_choice_events, resolve_menu_choice, resolve_menu_choice_in_options,
};
use crate::engine::state::{WorldState, render_dynamic_story_text};
use crate::engine::turn_runner::types::PlannedTurn;

/// Resolves a raw input line against an open menu (or an objective stage's
/// menu). Handles multi-select "done"/"toggle:" words and option resolution,
/// returning the events to queue and whether the turn advances time.
pub(super) fn try_resolve_menu_choice(
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
                        message: render_dynamic_story_text(
                            &menu.invalid_choice_text,
                            planner_state,
                        ),
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

/// Fallback for input no parsed command and no menu option matched. Any input
/// that *does* resolve an open menu option is handled by
/// [`try_resolve_menu_choice`], so reaching here means the input only fails —
/// in a menu context it is rejected as an invalid choice, otherwise it is an
/// unknown input.
pub(super) fn plan_unknown_command(
    content: &ContentPack,
    planner_state: &WorldState,
    raw_input: &str,
    planned: &mut PlannedTurn,
) -> bool {
    if let Some(menu_id) = planner_state.active_menu_id.as_deref() {
        if let Some(menu) = content.menu(menu_id) {
            planned.events.push(WorldEvent::ActionRejected {
                message: render_dynamic_story_text(&menu.invalid_choice_text, planner_state),
            });
        } else {
            planned.events.push(WorldEvent::ActionRejected {
                message: content.ui_text.menu_unavailable.clone(),
            });
        }
        return false;
    }
    if let Some(stage_menu) = planner_state
        .active_objective_stage_ids
        .iter()
        .find_map(|stage_id| {
            content
                .menus
                .iter()
                .find(|menu| menu.stage_id == *stage_id && !menu.options.is_empty())
        })
    {
        planned.events.push(WorldEvent::ActionRejected {
            message: render_dynamic_story_text(&stage_menu.invalid_choice_text, planner_state),
        });
        return false;
    }
    let raw = raw_input.trim();
    if let Some(exit) = content.resolve_exit_for(&planner_state.current_room_id, raw, |key| {
        crate::engine::turn_policies::story_var_is_truthy(planner_state, key)
    }) {
        planned.events.push(WorldEvent::PlayerMoved {
            from_room_id: planner_state.current_room_id.clone(),
            to_room_id: exit.room_id.clone(),
        });
        planned.events.push(WorldEvent::CurrentRoomObserved {
            room_id: exit.room_id.clone(),
            mode: crate::engine::events::ObservationMode::Summary,
        });
        return true;
    }
    if try_reject_unreachable_exit(content, planner_state, raw, planned) {
        return false;
    }
    planned.events.push(WorldEvent::UnknownInput {
        raw_input: raw_input.to_string(),
    });
    false
}

fn try_reject_unreachable_exit(
    content: &ContentPack,
    planner_state: &WorldState,
    raw: &str,
    planned: &mut PlannedTurn,
) -> bool {
    if let Some(dir) = canonical_direction(raw) {
        planned.events.push(WorldEvent::ActionRejected {
            message: content.render_template(
                &content.presentation.error_text.cannot_go,
                &[("target", dir)],
            ),
        });
        return true;
    }
    let room_has_gated_exit = content
        .room(&planner_state.current_room_id)
        .is_some_and(|room| {
            room.exits.iter().any(|exit| {
                exit.label.eq_ignore_ascii_case(raw)
                    || exit.aliases.iter().any(|a| a.eq_ignore_ascii_case(raw))
                    || exit.room_id.eq_ignore_ascii_case(raw)
            })
        });
    if room_has_gated_exit {
        let formatted = format_cannot_go_target(raw);
        planned.events.push(WorldEvent::ActionRejected {
            message: content.render_template(
                &content.presentation.error_text.cannot_go,
                &[("target", &formatted)],
            ),
        });
        return true;
    }
    false
}

pub(crate) fn canonical_direction(raw: &str) -> Option<&'static str> {
    let lower = raw.trim().to_ascii_lowercase();
    let stripped = strip_direction_prefix(&lower);
    match stripped {
        "north" | "n" => Some("north"),
        "south" | "s" => Some("south"),
        "east" | "e" => Some("east"),
        "west" | "w" => Some("west"),
        "northeast" | "ne" => Some("northeast"),
        "northwest" | "nw" => Some("northwest"),
        "southeast" | "se" => Some("southeast"),
        "southwest" | "sw" => Some("southwest"),
        "up" | "u" => Some("up"),
        "down" | "d" => Some("down"),
        "in" => Some("in"),
        "out" => Some("out"),
        _ => None,
    }
}

pub(crate) fn strip_direction_prefix(s: &str) -> &str {
    let s = s
        .strip_prefix("go to ")
        .or_else(|| s.strip_prefix("go "))
        .or_else(|| s.strip_prefix("enter "))
        .or_else(|| s.strip_prefix("move to "))
        .unwrap_or(s)
        .trim();
    let s = s
        .strip_prefix("the ")
        .or_else(|| s.strip_prefix("to "))
        .unwrap_or(s)
        .trim();
    s.strip_prefix("the ").unwrap_or(s).trim()
}

pub(crate) fn format_cannot_go_target(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some(dir) = canonical_direction(trimmed) {
        return dir.to_string();
    }

    // Strip generic command words if typed ("go to ", "go ", "move to ", "move ", "enter ")
    let mut cleaned = trimmed;
    let lower = trimmed.to_ascii_lowercase();
    for prefix in &["go to ", "go ", "move to ", "move ", "enter "] {
        if lower.starts_with(prefix) {
            cleaned = trimmed[prefix.len()..].trim();
            break;
        }
    }

    if let Some(dir) = canonical_direction(cleaned) {
        return dir.to_string();
    }

    let cleaned_lower = cleaned.to_ascii_lowercase();

    if cleaned_lower.starts_with("through ")
        || cleaned_lower.starts_with("into ")
        || cleaned_lower.starts_with("towards ")
        || cleaned_lower.starts_with("toward ")
        || cleaned_lower.starts_with("to the ")
    {
        return cleaned.to_string();
    }

    if cleaned_lower.starts_with("to ") {
        let after_to = cleaned[3..].trim();
        let after_lower = after_to.to_ascii_lowercase();
        if after_lower.starts_with("the ")
            || after_lower.starts_with("floor ")
            || after_lower.starts_with("level ")
            || after_lower.starts_with("room ")
        {
            return cleaned.to_string();
        }
        return format!("to the {after_to}");
    }

    if cleaned_lower.starts_with("the ")
        || cleaned_lower.starts_with("floor ")
        || cleaned_lower.starts_with("level ")
        || cleaned_lower.starts_with("room ")
    {
        return format!("to {cleaned}");
    }

    format!("to the {cleaned}")
}
