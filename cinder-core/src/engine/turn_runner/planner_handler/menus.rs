use crate::engine::turn_runner::types::PlannedTurn;
use crate::content::types::ContentPack;
use crate::engine::events::WorldEvent;
use crate::engine::menus::{
    build_menu_choice_events, resolve_menu_choice, resolve_menu_choice_in_options,
};
use crate::engine::state::{WorldState, render_dynamic_story_text};

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
    planned.events.push(WorldEvent::UnknownInput {
        raw_input: raw_input.to_string(),
    });
    false
}