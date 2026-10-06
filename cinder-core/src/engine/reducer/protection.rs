use crate::content::types::ContentPack;
use crate::engine::narrative::NarrativeLines;
use crate::engine::state::WorldState;

use super::beat_advance::advance_objective_for_signal;
use super::handlers::rendered_message_line;

pub(super) fn apply_active_protection_rules(
    _state: &mut WorldState,
    _content: &ContentPack,
    _lines: &mut NarrativeLines,
) {
    // Spatial live combat handles civilian protection directly.
}

pub(in crate::engine::reducer) fn handle_protected_actor_defeat(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    actor_name: &str,
    lines: &mut NarrativeLines,
) {
    let health_stat_id = &content.settings.combat.health_stat_id;
    let active_rules = state
        .active_objective_stage_ids
        .iter()
        .filter_map(|stage_id| {
            content
                .beats
                .stages
                .iter()
                .find(|stage| stage.id == *stage_id)
                .and_then(|stage| {
                    stage
                        .protection_rule
                        .clone()
                        .map(|rule| (stage_id.clone(), rule))
                })
        })
        .collect::<Vec<_>>();

    for (_stage_id, rule) in active_rules {
        if !rule.protected_actor_ids.iter().any(|id| id == actor_id) {
            continue;
        }

        let deaths = rule
            .protected_actor_ids
            .iter()
            .filter(|id| state.actor_is_defeated(id, health_stat_id))
            .count();

        if !rule.death_count_story_var.is_empty() {
            state
                .story_vars
                .set_unchecked(&rule.death_count_story_var, &deaths.to_string());
        }

        let deaths_text = deaths.to_string();
        push_message(
            content,
            lines,
            &rule.death_message,
            &[("actor", actor_name), ("deaths", deaths_text.as_str())],
        );

        let failed = rule.failure_death_count > 0 && deaths >= rule.failure_death_count;
        let already_failed = !rule.failure_story_var.is_empty()
            && state
                .story_vars
                .get(&rule.failure_story_var)
                .is_some_and(|value| value == "true");
        if failed && !already_failed {
            if !rule.failure_story_var.is_empty() {
                state
                    .story_vars
                    .set_unchecked(&rule.failure_story_var, "true");
            }
            if !rule.failure_signal.is_empty() {
                lines.extend_lines(advance_objective_for_signal(
                    state,
                    content,
                    &rule.failure_signal,
                ));
            }
        }
    }
}

fn push_message(
    content: &ContentPack,
    lines: &mut NarrativeLines,
    key: &str,
    vars: &[(&str, &str)],
) {
    if key.is_empty() {
        return;
    }
    if let Some(rendered) = rendered_message_line(content, key, vars) {
        lines.0.push(rendered);
    }
}
