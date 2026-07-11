use crate::content::types::{AdvanceCondition, AdvanceEffect, ContentPack};
use crate::engine::state::WorldState;
use std::collections::BTreeMap;

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
        state.stages_completed += 1;
        if current_stage.next_stage_ids.is_empty() {
            continue;
        }
        for next_stage_id in &current_stage.next_stage_ids {
            next_active_stage_ids.push(next_stage_id.clone());
            let Some(next_stage) = content
                .beats
                .stages
                .iter()
                .find(|stage| stage.id == *next_stage_id)
            else {
                continue;
            };
            for relocation in &next_stage.actor_relocations {
                state
                    .actor_room_overrides
                    .insert(relocation.actor_id.clone(), relocation.to_room_id.clone());
            }
            if !next_stage.projector_sequence_var_key.is_empty() {
                let selected_value = state.story_vars.get(&next_stage.projector_sequence_var_key);
                if let Some(selected_value) = selected_value
                    && let Some(movie) = content
                        .movies
                        .iter()
                        .find(|movie| movie.match_value == *selected_value)
                {
                    state.pending_projector_sequence_id = Some(movie.id.clone());
                    state.current_time_minutes += next_stage.elapsed_minutes;
                    state.pending_projector_narrative_lines = next_stage
                        .narrative_lines
                        .iter()
                        .map(|line| super::observation::render_story_text(line, state))
                        .collect();
                    continue;
                }
            }
            state.current_time_minutes += next_stage.elapsed_minutes;
            for line in &next_stage.narrative_lines {
                messages.push(super::observation::render_story_text(line, state));
            }
            for effect in &next_stage.on_advance_effects {
                match effect {
                    AdvanceEffect::AdjustActorStat {
                        actor_id,
                        stat,
                        delta,
                    } => {
                        if let Err(e) = state.adjust_actor_stat(actor_id, stat, *delta) {
                            eprintln!("[cinder] on_advance_effect error: {e}");
                        }
                    }
                    AdvanceEffect::AdjustPairStat {
                        participant_a_id,
                        participant_b_id,
                        stat,
                        delta,
                    } => {
                        if let Err(e) =
                            state.adjust_pair_stat(participant_a_id, participant_b_id, stat, *delta)
                        {
                            eprintln!("[cinder] on_advance_effect error: {e}");
                        }
                    }
                }
            }
            if next_stage.end_session {
                state.stages_completed += 1;
                state.game_over = true;
                messages.push(content.presentation.presentation_text.session_ended.clone());
            }
        }
    }
    next_active_stage_ids.sort();
    next_active_stage_ids.dedup();
    state.active_objective_stage_ids = next_active_stage_ids;
    if let Some(stage_id) = fallback_stage_to_activate(
        &content.settings.fallback_stage_id,
        &content.settings.fallback_required_story_vars,
        &content.beats.stages,
        &state.active_objective_stage_ids,
        state.game_over,
        &state.story_vars,
    ) {
        state.active_objective_stage_ids.push(stage_id);
    }
    messages
}

fn fallback_stage_to_activate(
    fallback_stage_id: &str,
    fallback_required_story_vars: &[String],
    stages: &[crate::content::types::BeatDefinition],
    active_stage_ids: &[String],
    game_over: bool,
    story_vars: &BTreeMap<String, String>,
) -> Option<String> {
    if game_over || !active_stage_ids.is_empty() || fallback_stage_id.is_empty() {
        return None;
    }
    if !fallback_required_story_vars
        .iter()
        .all(|required_key| story_vars.contains_key(required_key))
    {
        return None;
    }
    stages
        .iter()
        .find(|stage| stage.id == fallback_stage_id)
        .map(|stage| stage.id.clone())
}

#[cfg(test)]
mod tests {
    use super::fallback_stage_to_activate;
    use crate::content::types::BeatDefinition;
    use std::collections::BTreeMap;

    #[test]
    fn fallback_stage_activates_only_when_requirements_are_met() {
        let stages = vec![BeatDefinition {
            id: "wind-down".to_string(),
            ..BeatDefinition::default()
        }];
        let story_vars = BTreeMap::from([
            ("movie_title".to_string(), "A Film".to_string()),
            ("snack_title".to_string(), "Toast".to_string()),
        ]);

        let stage_id = fallback_stage_to_activate(
            "wind-down",
            &["movie_title".to_string(), "snack_title".to_string()],
            &stages,
            &[],
            false,
            &story_vars,
        );

        assert_eq!(stage_id.as_deref(), Some("wind-down"));
    }

    #[test]
    fn fallback_stage_does_not_activate_when_any_requirement_is_missing() {
        let stages = vec![BeatDefinition {
            id: "wind-down".to_string(),
            ..BeatDefinition::default()
        }];
        let story_vars = BTreeMap::from([("movie_title".to_string(), "A Film".to_string())]);

        let stage_id = fallback_stage_to_activate(
            "wind-down",
            &["movie_title".to_string(), "snack_title".to_string()],
            &stages,
            &[],
            false,
            &story_vars,
        );

        assert!(stage_id.is_none());
    }
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
        "stages_completed": state.stages_completed,
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
