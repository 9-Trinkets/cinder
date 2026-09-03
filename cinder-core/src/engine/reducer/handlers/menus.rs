use crate::engine::reducer::beat_advance::advance_objective_for_signal;
use crate::engine::reducer::observation::render_story_text;
use crate::content::types::ContentPack;
use crate::engine::hooks::apply_world_hook_effects;
use crate::engine::narrative::NarrativeLines;
use crate::engine::state::WorldState;
use serde_json::json;
pub(crate) fn handle_menu_opened(
    state: &mut WorldState,
    content: &ContentPack,
    menu_id: &str,
    lines: &mut NarrativeLines,
) {
    state.active_menu_id = Some(menu_id.to_string());
    state.pending_menu_selections.clear();
    if let Some(menu) = content.menu(menu_id) {
        lines.extend_narration(
            menu.opening_narrative_lines
                .iter()
                .map(|line| render_story_text(line, state)),
        );
    }
    lines.extend_narration(advance_objective_for_signal(
        state,
        content,
        &format!("menu_opened:{menu_id}"),
    ));
}

pub(crate) fn handle_menu_choice_made(
    state: &mut WorldState,
    content: &ContentPack,
    menu_id: &str,
    option_id: &str,
    title: &str,
    lines: &mut NarrativeLines,
) {
    state.active_menu_id = None;
    if let Some(menu) = content.menu(menu_id) {
        let is_multi_select = menu.max_selections > 0;
        if is_multi_select && option_id == "done" {
            let selected_ids: Vec<String> = state.pending_menu_selections.clone();
            let selected_titles: Vec<String> = selected_ids
                .iter()
                .filter_map(|id| {
                    menu.options
                        .iter()
                        .find(|opt| opt.id == *id)
                        .map(|opt| opt.title.clone())
                })
                .collect();
            let joined_titles = selected_titles.join(", ");
            let joined_ids = selected_ids.join(", ");
            state
                .story_vars
                .set_unchecked("selection_title", &joined_titles);
            if !menu.multi_selection_var_keys.is_empty() {
                for (i, var_key) in menu.multi_selection_var_keys.iter().enumerate() {
                    let value = selected_titles.get(i).cloned().unwrap_or_default();
                    state.story_vars.set_unchecked(var_key, &value);
                }
            }
            if !menu.multi_selection_room_var_keys.is_empty() {
                let selected_rooms: Vec<String> = selected_ids
                    .iter()
                    .filter_map(|id| {
                        menu.options
                            .iter()
                            .find(|opt| opt.id == *id)
                            .filter(|opt| !opt.room_id.is_empty())
                            .map(|opt| opt.room_id.clone())
                    })
                    .collect();
                for (i, var_key) in menu.multi_selection_room_var_keys.iter().enumerate() {
                    let value = selected_rooms.get(i).cloned().unwrap_or_default();
                    state.story_vars.set_unchecked(var_key, &value);
                }
            }
            if !menu.multi_selection_host_var_keys.is_empty() {
                let selected_hosts: Vec<String> = selected_ids
                    .iter()
                    .filter_map(|id| {
                        menu.options
                            .iter()
                            .find(|opt| opt.id == *id)
                            .filter(|opt| !opt.host_actor_id.is_empty())
                            .map(|opt| opt.host_actor_id.clone())
                    })
                    .collect();
                for (i, var_key) in menu.multi_selection_host_var_keys.iter().enumerate() {
                    let value = selected_hosts.get(i).cloned().unwrap_or_default();
                    state.story_vars.set_unchecked(var_key, &value);
                }
            }
            if menu.multi_selection_var_keys.is_empty() && !menu.selection_var_key.is_empty() {
                state
                    .story_vars
                    .set_unchecked(&menu.selection_var_key, &joined_titles);
            }
            if !menu.selection_id_var_key.is_empty() {
                state
                    .story_vars
                    .set_unchecked(&menu.selection_id_var_key, &joined_ids);
            }
            lines.narration(render_story_text(
                &menu.selection_confirmation,
                state,
            ));
        } else {
            state.story_vars.set_unchecked("selection_title", title);
            if !menu.selection_var_key.is_empty() {
                state
                    .story_vars
                    .set_unchecked(&menu.selection_var_key, title);
            }
            if !menu.selection_id_var_key.is_empty() {
                state
                    .story_vars
                    .set_unchecked(&menu.selection_id_var_key, option_id);
            }
            lines.narration(render_story_text(
                &menu.selection_confirmation,
                state,
            ));
        }
        state.pending_menu_selections.clear();
    }
    apply_world_hook_effects(
        state,
        content,
        &format!("menu.{menu_id}.selected"),
        json!({
            "menu_id": menu_id,
            "option_id": option_id,
            "title": title,
        }),
    )
    .unwrap_or_else(|error| eprintln!("[cinder] hook warning (menu.selected): {error}"));
    lines.extend_narration(advance_objective_for_signal(
        state,
        content,
        &format!("menu_selected:{menu_id}"),
    ));
}

pub(crate) fn handle_menu_selection_toggled(
    state: &mut WorldState,
    content: &ContentPack,
    menu_id: &str,
    option_id: &str,
    selected: bool,
) {
    if let Some(menu) = content.menu(menu_id) {
        let max = menu.max_selections;
        if selected {
            if max > 0 && state.pending_menu_selections.len() >= max {
                return;
            }
            if !state
                .pending_menu_selections
                .contains(&option_id.to_string())
            {
                state.pending_menu_selections.push(option_id.to_string());
            }
        } else {
            state.pending_menu_selections.retain(|id| id != option_id);
        }
    }
}