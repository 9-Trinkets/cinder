use crate::content::types::{AdvanceEffect, Condition, ContentPack, SurvivorRewardDefinition};
use crate::engine::narrative::{NarrativeLine, NarrativeLines};
use crate::engine::state::{GamePhase, SpawnActorConfig, SpawnActorOutcome, WorldState};
use crate::engine::turn_policies::clear_inactive_objective_state;
use crate::engine::world_mutation::{WorldMutation, WorldMutationApplier};

use super::handlers::{handle_item_transferred, rendered_message_line};
use super::observation::render_actor_speech_line;

pub(crate) fn advance_objective_for_signal(
    state: &mut WorldState,
    content: &ContentPack,
    signal: &str,
) -> Vec<NarrativeLine> {
    dispatch_due_actor_spawns(state, content);
    let mut messages = Vec::new();
    let mut next_active_stage_ids = Vec::with_capacity(state.active_objective_stage_ids.len());
    let mut next_stage_started_minutes = std::collections::BTreeMap::new();
    for current_stage_id in state.active_objective_stage_ids.clone() {
        let current_stage_started_minutes = state
            .stage_started_minutes
            .get(&current_stage_id)
            .copied()
            .unwrap_or(state.clock.current_time_minutes);
        let Some(current_stage) = content
            .beats
            .stages
            .iter()
            .find(|stage| stage.id == current_stage_id)
        else {
            next_stage_started_minutes
                .insert(current_stage_id.clone(), current_stage_started_minutes);
            next_active_stage_ids.push(current_stage_id);
            continue;
        };
        let Some(matched_signal) = current_stage.advance_signals.iter().find(|candidate| {
            advance_signal_matches(
                content,
                state,
                &current_stage_id,
                signal,
                candidate.signal(),
                candidate.conditions(),
            )
        }) else {
            next_stage_started_minutes
                .insert(current_stage_id.clone(), current_stage_started_minutes);
            next_active_stage_ids.push(current_stage_id);
            continue;
        };
        state.stages_completed += 1;
        state.completed_stage_ids.insert(current_stage_id.clone());
        // Completion commentary belongs to the stage that was just finished, so
        // it is emitted here — ahead of the terminal-stage early return below.
        if let Some(key) = current_stage.completion_message.as_deref()
            && let Some(line) = rendered_message_line(content, key, &[])
        {
            messages.push(line);
        }
        apply_stage_effects(
            state,
            content,
            &current_stage.completion_effects,
            &mut messages,
        );
        let next_stage_ids = matched_signal
            .next_stage_ids()
            .unwrap_or(&current_stage.next_stage_ids);
        if next_stage_ids.is_empty() {
            continue;
        }
        for next_stage_id in next_stage_ids {
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
                    .actors
                    .actor_room_overrides
                    .insert(relocation.actor_id.clone(), relocation.to_room_id.clone());
            }
            dispatch_actor_spawns_for_stage(state, content, next_stage, 0);
            // A stage can open with a movie, picked by id or by a story variable. Its
            // own narrative lines wait until the movie is closed.
            let movie_id = if !next_stage.movie_id.is_empty() {
                Some(next_stage.movie_id.clone())
            } else if next_stage.projector_sequence_var_key.is_empty() {
                None
            } else {
                state
                    .story_vars
                    .get(&next_stage.projector_sequence_var_key)
                    .and_then(|selected_value| {
                        content
                            .movies
                            .iter()
                            .find(|movie| movie.match_value == selected_value)
                    })
                    .map(|movie| movie.id.clone())
            };
            state.clock.current_time_minutes += next_stage.elapsed_minutes;
            next_stage_started_minutes
                .entry(next_stage_id.clone())
                .or_insert(state.clock.current_time_minutes);
            let rendered_lines = next_stage
                .narrative_lines
                .iter()
                .map(|line| super::observation::render_story_text(line, state))
                .collect::<Vec<_>>();
            if let Some(movie_id) = movie_id {
                state.pending_projector_sequence_id = Some(movie_id);
                if next_stage.lines_before_movie {
                    for line in rendered_lines {
                        messages.push(NarrativeLine::narration(line));
                    }
                } else {
                    state.pending_projector_narrative_lines = rendered_lines;
                }
            } else {
                for line in rendered_lines {
                    messages.push(NarrativeLine::narration(line));
                }
            }
            if !next_stage.open_menu.is_empty() {
                state.active_menu_id = Some(next_stage.open_menu.clone());
            }
            apply_stage_effects(
                state,
                content,
                &next_stage.on_advance_effects,
                &mut messages,
            );
            if next_stage.end_act && !content.act_cast.is_empty() {
                state.stages_completed += 1;
                state.phase = GamePhase::ActEnded;
            }
            if next_stage.end_game {
                state.stages_completed += 1;
                state.phase = GamePhase::GameEnded;
            }
        }
    }
    next_active_stage_ids.sort();
    next_active_stage_ids.dedup();
    state.active_objective_stage_ids = next_active_stage_ids;
    state.stage_started_minutes = next_stage_started_minutes;
    state
        .story_vars
        .clear_scoped(crate::engine::state::VariableScope::Stage);
    clear_inactive_objective_state(content, state);
    if let Some(stage_id) = fallback_stage_to_activate(
        &content.settings.fallback_stage_id,
        &content.settings.fallback_when,
        &content.beats.stages,
        &state.active_objective_stage_ids,
        state.phase != GamePhase::Active,
        content,
        state,
    ) {
        state.active_objective_stage_ids.push(stage_id.clone());
        state
            .stage_started_minutes
            .insert(stage_id, state.clock.current_time_minutes);
    }
    messages
}

pub(super) fn apply_stage_effects(
    state: &mut WorldState,
    content: &ContentPack,
    effects: &[AdvanceEffect],
    messages: &mut Vec<NarrativeLine>,
) {
    for effect in effects {
        if let Err(error) = apply_stage_effect(state, content, effect, messages) {
            eprintln!("[cinder] stage effect error: {error}");
        }
    }
}

fn apply_stage_effect(
    state: &mut WorldState,
    content: &ContentPack,
    effect: &AdvanceEffect,
    messages: &mut Vec<NarrativeLine>,
) -> Result<(), String> {
    let mutation = match effect {
        AdvanceEffect::AdjustActorStat {
            actor_id,
            stat,
            delta,
        } => WorldMutation::AdjustActorStat {
            actor_id: actor_id.clone(),
            stat: stat.clone(),
            delta: *delta,
        },
        AdvanceEffect::AdjustPairStat {
            participant_a_id,
            participant_b_id,
            stat,
            delta,
        } => WorldMutation::AdjustPairStat {
            participant_a_id: participant_a_id.clone(),
            participant_b_id: participant_b_id.clone(),
            stat: stat.clone(),
            delta: *delta,
        },
        AdvanceEffect::SetStoryVar { key, value } => WorldMutation::SetStoryVar {
            key: key.clone(),
            value: value.clone(),
        },
        AdvanceEffect::GrantItem {
            item_id,
            count,
            message,
        } => {
            if !state.has_item(item_id) {
                for _ in 0..*count {
                    state.add_item(item_id);
                }
                push_effect_message(content, message, messages);
            }
            return Ok(());
        }
        AdvanceEffect::RemoveItem {
            item_id,
            count,
            message,
        } => {
            let mut removed = false;
            for _ in 0..*count {
                if state.remove_item(item_id) {
                    removed = true;
                }
            }
            if removed {
                push_effect_message(content, message, messages);
            }
            return Ok(());
        }
        AdvanceEffect::QueueSequence { sequence_id } => {
            return state.queue_scripted_sequence(sequence_id);
        }
        AdvanceEffect::SetFollowsPlayer { actor_id, follows } => {
            state.set_follows_player(actor_id, *follows);
            return Ok(());
        }
        AdvanceEffect::ResolveSurvivorRewards {
            rewards,
            survivor_count_story_var,
            all_survived_story_var,
            all_survived_message,
        } => {
            resolve_survivor_rewards(
                state,
                content,
                rewards,
                survivor_count_story_var,
                all_survived_story_var,
                all_survived_message,
                messages,
            );
            return Ok(());
        }
    };
    WorldMutationApplier::new(state, content).apply(&mutation)
}

fn resolve_survivor_rewards(
    state: &mut WorldState,
    content: &ContentPack,
    rewards: &[SurvivorRewardDefinition],
    survivor_count_story_var: &str,
    all_survived_story_var: &str,
    all_survived_message: &str,
    messages: &mut Vec<NarrativeLine>,
) {
    let health_stat_id = &content.settings.combat.health_stat_id;
    let mut survivor_count = 0;
    for reward in rewards {
        if state.actor_is_defeated(&reward.actor_id, health_stat_id) {
            continue;
        }
        survivor_count += 1;
        let newly_unlocked = state.story_vars.get(&reward.unlock_story_var) != Some("true");
        state
            .story_vars
            .set_unchecked(&reward.unlock_story_var, "true");
        if newly_unlocked {
            let actor_name = state
                .actor_display_name(content, &reward.actor_id)
                .unwrap_or(&reward.actor_id);
            if let Some(speech) = content.render_message(&reward.message, &[])
                && !speech.trim().is_empty()
            {
                messages.push(NarrativeLine::narration(render_actor_speech_line(
                    content,
                    &actor_name,
                    None,
                    &speech,
                )));
            }
        }
        if !state.has_item(&reward.item_id) {
            let mut transfer_lines = NarrativeLines::default();
            state.actor_add_item(&reward.actor_id, &reward.item_id);
            let player_id = content.settings.combat.player_actor_id.clone();
            handle_item_transferred(
                state,
                content,
                &reward.item_id,
                &reward.actor_id,
                &player_id,
                Some(&reward.actor_id),
                &mut transfer_lines,
            );
            messages.extend(transfer_lines.0);
        }
    }
    state
        .story_vars
        .set_unchecked(survivor_count_story_var, &survivor_count.to_string());
    if !all_survived_story_var.is_empty() && survivor_count == rewards.len() {
        let newly_unlocked = state.story_vars.get(all_survived_story_var) != Some("true");
        state
            .story_vars
            .set_unchecked(all_survived_story_var, "true");
        if newly_unlocked {
            push_effect_message(content, all_survived_message, messages);
        }
    }
}

fn push_effect_message(content: &ContentPack, message: &str, messages: &mut Vec<NarrativeLine>) {
    if !message.is_empty()
        && let Some(line) = rendered_message_line(content, message, &[])
    {
        messages.push(line);
    }
}

fn dispatch_due_actor_spawns(state: &mut WorldState, content: &ContentPack) {
    for stage_id in state.active_objective_stage_ids.clone() {
        let Some(stage) = content
            .beats
            .stages
            .iter()
            .find(|stage| stage.id == stage_id)
        else {
            continue;
        };
        let started_at = state
            .stage_started_minutes
            .get(&stage_id)
            .copied()
            .unwrap_or(state.clock.current_time_minutes);
        let elapsed_minutes = state.clock.current_time_minutes.saturating_sub(started_at);
        dispatch_actor_spawns_for_stage(state, content, stage, elapsed_minutes);
    }
}

pub(super) fn route_stopped_spawn_schedules(
    state: &mut WorldState,
    content: &ContentPack,
) -> Vec<NarrativeLine> {
    let mut messages = Vec::new();
    for stage in &content.beats.stages {
        let Some(schedule) = stage.actor_spawn_schedule.as_ref() else {
            continue;
        };
        let schedule_started = state
            .active_objective_stage_ids
            .iter()
            .any(|stage_id| stage_id == &stage.id)
            || state.completed_stage_ids.contains(&stage.id);
        if !schedule_started
            || !schedule.rout_on_stop
            || schedule.stop_story_var.is_empty()
            || !crate::engine::turn_policies::story_var_is_truthy(state, &schedule.stop_story_var)
        {
            continue;
        }
        let routed_key = format!("spawn_schedule_routed:{}", stage.id);
        if crate::engine::turn_policies::story_var_is_truthy(state, &routed_key) {
            continue;
        }
        let count = state.despawn_template_instances(&schedule.template_id);
        state.story_vars.set_unchecked(&routed_key, "true");
        if count > 0
            && !schedule.rout_message.is_empty()
            && let Some(line) = rendered_message_line(
                content,
                &schedule.rout_message,
                &[("count", count.to_string().as_str())],
            )
        {
            messages.push(line);
        }
    }
    messages
}

fn dispatch_actor_spawns_for_stage(
    state: &mut WorldState,
    content: &ContentPack,
    stage: &crate::content::types::BeatDefinition,
    elapsed_minutes: u32,
) {
    let Some(schedule) = stage.actor_spawn_schedule.as_ref() else {
        return;
    };
    if !schedule.stop_story_var.is_empty()
        && crate::engine::turn_policies::story_var_is_truthy(state, &schedule.stop_story_var)
    {
        return;
    }
    let interval = schedule.interval_minutes.max(1);
    let due_batches = elapsed_minutes / interval + 1;
    let due_count = schedule
        .total_count
        .min(schedule.batch_size.saturating_mul(due_batches as usize));
    let instance_prefix = format!("{}-", schedule.template_id);
    let dispatched_count = state
        .actors
        .spawned_actors
        .keys()
        .filter(|actor_id| actor_id.starts_with(&instance_prefix))
        .count();

    let mut current_batch_leader: Option<String> = None;
    for i in dispatched_count..due_count {
        match state.spawn_actor(
            content,
            SpawnActorConfig {
                template_id: &schedule.template_id,
                room_id: Some(&schedule.room_id),
                stance: None,
                follows_player: false,
                scale_with_actor_id: None,
                scale_stat: None,
                max_active_instances: None,
            },
        ) {
            SpawnActorOutcome::Success(info) => {
                if let Some(party_order) = &schedule.party_order {
                    let batch_size = schedule.batch_size.max(1);
                    if i % batch_size == 0 {
                        current_batch_leader = Some(info.instance_id.clone());
                        state
                            .relations
                            .party_orders
                            .insert(info.instance_id, party_order.clone());
                    } else {
                        let leader_id = current_batch_leader.clone().or_else(|| {
                            let batch_start = (i / batch_size) * batch_size + 1;
                            let candidate = format!("{}-{batch_start}", schedule.template_id);
                            state
                                .actors
                                .spawned_actors
                                .contains_key(&candidate)
                                .then_some(candidate)
                        });
                        if let Some(leader) = leader_id {
                            state
                                .relations
                                .party_orders
                                .insert(info.instance_id.clone(), "follow".to_string());
                            state.set_followed_actor(&info.instance_id, Some(&leader));
                        }
                    }
                }
            }
            SpawnActorOutcome::CapacityExceeded { .. } | SpawnActorOutcome::TemplateNotFound => {
                break;
            }
        }
    }
}

fn advance_signal_matches(
    content: &ContentPack,
    state: &WorldState,
    current_stage_id: &str,
    current_signal: &str,
    candidate_signal: &str,
    conditions: &[Condition],
) -> bool {
    if let Some(minutes) = candidate_signal.strip_prefix("stage_elapsed:") {
        let Ok(required_minutes) = minutes.parse::<u32>() else {
            return false;
        };
        let Some(started_minutes) = state.stage_started_minutes.get(current_stage_id).copied()
        else {
            return false;
        };
        return current_signal.starts_with("time_reached:")
            && state
                .clock
                .current_time_minutes
                .saturating_sub(started_minutes)
                >= required_minutes
            && conditions_hold(content, state, conditions);
    }

    candidate_signal == current_signal && conditions_hold(content, state, conditions)
}

fn conditions_hold(content: &ContentPack, state: &WorldState, conditions: &[Condition]) -> bool {
    crate::engine::conditions::all_hold(
        conditions,
        &crate::engine::conditions::ConditionContext::new(content, state, &state.current_room_id),
    )
}

fn fallback_stage_to_activate(
    fallback_stage_id: &str,
    fallback_when: &[Condition],
    stages: &[crate::content::types::BeatDefinition],
    active_stage_ids: &[String],
    game_over: bool,
    content: &ContentPack,
    state: &WorldState,
) -> Option<String> {
    if game_over || !active_stage_ids.is_empty() || fallback_stage_id.is_empty() {
        return None;
    }
    if !conditions_hold(content, state, fallback_when) {
        return None;
    }
    stages
        .iter()
        .find(|stage| stage.id == fallback_stage_id)
        .map(|stage| stage.id.clone())
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

#[cfg(test)]
mod tests {
    use super::{advance_objective_for_signal, fallback_stage_to_activate};
    use crate::content::types::{AdvanceSignal, BeatDefinition, BeatsDefinition};
    use crate::engine::state::WorldState;

    fn fallback_state(vars: &[(&str, &str)]) -> (crate::content::types::ContentPack, WorldState) {
        let pack = crate::engine::test_fixtures::minimal_test_pack();
        let mut state = WorldState::new(&pack);
        for (key, value) in vars {
            state.story_vars.set_unchecked(key, value);
        }
        (pack, state)
    }

    fn fallback_requirements() -> Vec<crate::content::types::Condition> {
        vec![
            crate::content::types::Condition::StoryVar("movie_title".to_string()),
            crate::content::types::Condition::StoryVar("snack_title".to_string()),
        ]
    }

    #[test]
    fn fallback_stage_activates_only_when_requirements_are_met() {
        let stages = vec![BeatDefinition {
            id: "wind-down".to_string(),
            ..BeatDefinition::default()
        }];
        let (pack, state) = fallback_state(&[("movie_title", "A Film"), ("snack_title", "Toast")]);

        let stage_id = fallback_stage_to_activate(
            "wind-down",
            &fallback_requirements(),
            &stages,
            &[],
            false,
            &pack,
            &state,
        );

        assert_eq!(stage_id.as_deref(), Some("wind-down"));
    }

    #[test]
    fn fallback_stage_does_not_activate_when_any_requirement_is_missing() {
        let stages = vec![BeatDefinition {
            id: "wind-down".to_string(),
            ..BeatDefinition::default()
        }];
        let (pack, state) = fallback_state(&[("movie_title", "A Film")]);

        let stage_id = fallback_stage_to_activate(
            "wind-down",
            &fallback_requirements(),
            &stages,
            &[],
            false,
            &pack,
            &state,
        );

        assert!(stage_id.is_none());
    }

    #[test]
    fn stage_elapsed_signal_advances_relative_to_stage_start() {
        let mut pack = crate::engine::test_fixtures::minimal_test_pack();
        pack.beats = BeatsDefinition {
            initial_stage_ids: vec!["prep".to_string()],
            stages: vec![
                BeatDefinition {
                    id: "prep".to_string(),
                    advance_signals: vec![AdvanceSignal::Simple("stage_elapsed:10".to_string())],
                    next_stage_ids: vec!["done".to_string()],
                    ..BeatDefinition::default()
                },
                BeatDefinition {
                    id: "done".to_string(),
                    ..BeatDefinition::default()
                },
            ],
            ..BeatsDefinition::default()
        };
        let mut state = WorldState::new(&pack);
        state.clock.current_time_minutes = 9 * 60 + 12;
        state
            .stage_started_minutes
            .insert("prep".to_string(), 9 * 60);

        advance_objective_for_signal(&mut state, &pack, "time_reached:09:12");

        assert_eq!(state.active_objective_stage_ids, vec!["done".to_string()]);
        assert_eq!(
            state.stage_started_minutes.get("done"),
            Some(&(9 * 60 + 12))
        );
    }
}
