use crate::content::types::{ContentPack, ProtectionRuleDefinition};
use crate::engine::narrative::NarrativeLines;
use crate::engine::state::{ActorRelationship, WorldState};

use super::beat_advance::advance_objective_for_signal;
use super::handlers::rendered_message_line;

pub(super) fn apply_active_protection_rules(
    state: &mut WorldState,
    content: &ContentPack,
    lines: &mut NarrativeLines,
) {
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

    for (stage_id, rule) in active_rules {
        apply_protection_rule(state, content, lines, &stage_id, &rule);
    }
}

fn apply_protection_rule(
    state: &mut WorldState,
    content: &ContentPack,
    lines: &mut NarrativeLines,
    stage_id: &str,
    rule: &ProtectionRuleDefinition,
) {
    let hostile_present = state.onstage_actors(content).any(|actor| {
        actor.tags.iter().any(|tag| tag == &rule.hostile_tag)
            && state.actor_is_in_room(content, &actor.id, &rule.room_id)
            && !state.actor_is_defeated(&actor.id, &content.settings.combat.health_stat_id)
    });

    if !hostile_present {
        let protection_state = state
            .protection_rule_states
            .entry(stage_id.to_string())
            .or_default();
        let was_at_risk = protection_state.at_risk_actor_id.take().is_some();
        protection_state.breach_started_minutes = None;
        if was_at_risk {
            push_message(content, lines, &rule.cleared_message, &[]);
        }
        return;
    }

    let needs_target = state
        .protection_rule_states
        .get(stage_id)
        .is_none_or(|protection_state| protection_state.at_risk_actor_id.is_none());
    if needs_target {
        let next_index = state
            .protection_rule_states
            .get(stage_id)
            .map(|protection_state| protection_state.next_actor_index)
            .unwrap_or_default();
        let Some((actor_id, selected_index)) =
            next_living_protected_actor(state, content, rule, next_index)
        else {
            return;
        };
        let actor_name = actor_name(state, content, &actor_id);
        let breach_minutes = rule.breach_minutes.to_string();
        let protection_state = state
            .protection_rule_states
            .entry(stage_id.to_string())
            .or_default();
        protection_state.next_actor_index =
            (selected_index + 1) % rule.protected_actor_ids.len().max(1);
        protection_state.at_risk_actor_id = Some(actor_id);
        protection_state.breach_started_minutes = Some(state.current_time_minutes);
        push_message(
            content,
            lines,
            &rule.warning_message,
            &[
                ("actor", actor_name.as_str()),
                ("minutes", breach_minutes.as_str()),
            ],
        );
        return;
    }

    let Some(protection_state) = state.protection_rule_states.get(stage_id) else {
        return;
    };
    let Some(started_at) = protection_state.breach_started_minutes else {
        return;
    };
    if state.current_time_minutes.saturating_sub(started_at) < rule.breach_minutes {
        return;
    }

    let actor_id = protection_state
        .at_risk_actor_id
        .clone()
        .unwrap_or_default();
    let actor_name = actor_name(state, content, &actor_id);
    let health_stat_id = content.settings.combat.health_stat_id.clone();
    let _ = state.adjust_actor_stat(content, &actor_id, &health_stat_id, i32::MIN / 2);
    state.set_relationship(&actor_id, ActorRelationship::default());

    let deaths = rule
        .protected_actor_ids
        .iter()
        .filter(|actor_id| state.actor_is_defeated(actor_id, &health_stat_id))
        .count();
    if !rule.death_count_story_var.is_empty() {
        state
            .story_vars
            .set_unchecked(&rule.death_count_story_var, &deaths.to_string());
    }
    if let Some(protection_state) = state.protection_rule_states.get_mut(stage_id) {
        protection_state.at_risk_actor_id = None;
        protection_state.breach_started_minutes = None;
    }
    let deaths_text = deaths.to_string();
    push_message(
        content,
        lines,
        &rule.death_message,
        &[
            ("actor", actor_name.as_str()),
            ("deaths", deaths_text.as_str()),
        ],
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

fn next_living_protected_actor(
    state: &WorldState,
    content: &ContentPack,
    rule: &ProtectionRuleDefinition,
    start_index: usize,
) -> Option<(String, usize)> {
    if rule.protected_actor_ids.is_empty() {
        return None;
    }
    let health_stat_id = &content.settings.combat.health_stat_id;
    (0..rule.protected_actor_ids.len()).find_map(|offset| {
        let index = (start_index + offset) % rule.protected_actor_ids.len();
        let actor_id = &rule.protected_actor_ids[index];
        (!state.actor_is_defeated(actor_id, health_stat_id)).then(|| (actor_id.clone(), index))
    })
}

fn actor_name(state: &WorldState, content: &ContentPack, actor_id: &str) -> String {
    state
        .actor(content, actor_id)
        .map(|actor| crate::engine::state::display_actor_name(state, actor))
        .unwrap_or_else(|| actor_id.to_string())
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
