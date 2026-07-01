use crate::content::types::{AdvanceCondition, ContentPack};
use crate::engine::state::WorldState;

pub(super) fn advance_objective_for_signal(
    state: &mut WorldState,
    content: &ContentPack,
    signal: &str,
) -> Vec<String> {
    let mut messages = Vec::new();
    let mut next_active_stage_ids = Vec::with_capacity(state.active_objective_stage_ids.len());
    for current_stage_id in state.active_objective_stage_ids.clone() {
        let Some(current_stage) = content
            .beats
            .stages
            .iter()
            .find(|stage| stage.id == current_stage_id)
        else {
            next_active_stage_ids.push(current_stage_id);
            continue;
        };
        if !current_stage.advance_signals.iter().any(|candidate| {
            candidate.signal() == signal && advance_conditions_met(state, candidate.conditions())
        }) {
            next_active_stage_ids.push(current_stage_id);
            continue;
        }
        let next_stage_ids = current_stage.resolved_next_stage_ids();
        if next_stage_ids.is_empty() {
            continue;
        }
        for next_stage_id in next_stage_ids {
            next_active_stage_ids.push(next_stage_id.clone());
            let Some(next_stage) = content
                .beats
                .stages
                .iter()
                .find(|stage| stage.id == next_stage_id)
            else {
                continue;
            };
            if !next_stage.update_message.is_empty() {
                messages.push(super::observation::render_story_text(
                    &next_stage.update_message,
                    state,
                ));
            }
            for relocation in &next_stage.actor_relocations {
                state
                    .actor_room_overrides
                    .insert(relocation.actor_id.clone(), relocation.to_room_id.clone());
            }
            if !next_stage.projector_sequence_var_key.is_empty() {
                let selected_value = state.story_vars.get(&next_stage.projector_sequence_var_key);
                eprintln!("[debug] projector check: key={:?}, selected_value={:?}, movies_count={}", 
                    next_stage.projector_sequence_var_key, selected_value, content.movies.len());
                if let Some(selected_value) = selected_value
                    && let Some(movie) = content
                        .movies
                        .iter()
                        .find(|movie| movie.match_value == *selected_value)
                {
                    eprintln!("[debug] projector matched: movie_id={}", movie.id);
                    state.pending_projector_sequence_id = Some(movie.id.clone());
                    state.current_time_minutes += next_stage.elapsed_minutes;
                    state.pending_projector_narrative_lines = next_stage
                        .narrative_lines
                        .iter()
                        .map(|line| super::observation::render_story_text(line, state))
                        .collect();
                    continue;
                }
                eprintln!("[debug] projector no match for key={:?}", next_stage.projector_sequence_var_key);
            }
            state.current_time_minutes += next_stage.elapsed_minutes;
            for line in &next_stage.narrative_lines {
                messages.push(super::observation::render_story_text(line, state));
            }
            if next_stage.end_session {
                state.game_over = true;
                messages.push(content.presentation.presentation_text.session_ended.clone());
            }
        }
    }
    next_active_stage_ids.sort();
    next_active_stage_ids.dedup();
    state.active_objective_stage_ids = next_active_stage_ids;
    if state.active_objective_stage_ids.is_empty()
        && !state.game_over
        && state.story_vars.contains_key("movie_title")
        && state.story_vars.contains_key("snack_title")
        && let Some(stage) = content
            .beats
            .stages
            .iter()
            .find(|stage| stage.id == "go-to-bed")
    {
        state.active_objective_stage_ids.push(stage.id.clone());
        if !stage.update_message.is_empty() {
            messages.push(super::observation::render_story_text(
                &stage.update_message,
                state,
            ));
        }
    }
    messages
}

pub(super) fn advance_conditions_met(state: &WorldState, conditions: &[AdvanceCondition]) -> bool {
    if conditions.is_empty() {
        return true;
    }
    let input = world_state_condition_input(state);
    conditions
        .iter()
        .all(|cond| evaluate_advance_condition(&input, cond))
}

pub(super) fn world_state_condition_input(state: &WorldState) -> serde_json::Value {
    serde_json::json!({
        "current_time_minutes": state.current_time_minutes,
        "active_stage_ids": state.active_objective_stage_ids,
        "actor_stats": state.actor_stats,
        "pair_stats": state.pair_stats,
    })
}

pub(super) fn evaluate_advance_condition(
    input: &serde_json::Value,
    cond: &AdvanceCondition,
) -> bool {
    let actual = resolve_path(input, &cond.path);
    match cond.operator.as_str() {
        "equal" => actual == Some(&cond.value),
        "greater_than" => compare_numbers(&actual, &cond.value, |a, b| a > b),
        "less_than" => compare_numbers(&actual, &cond.value, |a, b| a < b),
        "gte" => compare_numbers(&actual, &cond.value, |a, b| a >= b),
        "lte" => compare_numbers(&actual, &cond.value, |a, b| a <= b),
        "not_equal" => actual != Some(&cond.value),
        "array_contains" => actual
            .and_then(|v| v.as_array())
            .map(|arr| arr.contains(&cond.value))
            .unwrap_or(false),
        _ => false,
    }
}

pub(super) fn resolve_path<'a>(
    value: &'a serde_json::Value,
    path: &str,
) -> Option<&'a serde_json::Value> {
    path.split('.')
        .try_fold(value, |acc, key| acc.as_object()?.get(key))
}

pub(super) fn compare_numbers(
    actual: &Option<&serde_json::Value>,
    expected: &serde_json::Value,
    cmp: impl Fn(f64, f64) -> bool,
) -> bool {
    let Some(a) = actual.and_then(|v| v.as_f64()) else {
        return false;
    };
    let Some(b) = expected.as_f64() else {
        return false;
    };
    cmp(a, b)
}

pub(super) fn time_reached_signal(current_time_minutes: u32) -> String {
    let hour24 = (current_time_minutes / 60) % 24;
    let minute = current_time_minutes % 60;
    format!("time_reached:{hour24:02}:{minute:02}")
}

pub(super) fn time_reached_signals(
    previous_time_minutes: u32,
    current_time_minutes: u32,
) -> Vec<String> {
    ((previous_time_minutes + 1)..=current_time_minutes)
        .map(time_reached_signal)
        .collect()
}
