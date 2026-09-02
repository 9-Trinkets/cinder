use super::CinderRuntime;
use crate::content::types::StageAssignmentDefinition;
use crate::engine::dialogue::{
    StageAssignment, StageAssignmentCandidate, StageAssignmentRequest, StageAssignmentScore,
};
use crate::engine::dialogue_grounding::render_story_text;
use crate::engine::state::{TurnOutcome, display_actor_name};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;

impl CinderRuntime {
    pub(super) fn apply_stage_assignments(
        &self,
        outcome: TurnOutcome,
    ) -> Result<TurnOutcome, Box<dyn Error>> {
        let Some((request, config)) = self.stage_assignment_request()? else {
            return Ok(outcome);
        };
        let assignment = self
            .dialogue
            .assign_stage_participants(&request)
            .unwrap_or_else(|error| {
                eprintln!(
                    "[cinder] stage assignment failed for '{}': {error}, using deterministic fallback",
                    request.stage_id
                );
                self.fallback_stage_assignment(&request)
            });
        let summary = self.commit_stage_assignment(&request, &assignment, &config)?;
        if summary.is_empty() {
            return Ok(outcome);
        }
        let text = if outcome.text.is_empty() {
            summary
        } else {
            format!("{}\n\n{}", outcome.text, summary)
        };
        Ok(TurnOutcome {
            text,
            phase: outcome.phase,
            lines: outcome.lines,
        })
    }

    fn stage_assignment_request(
        &self,
    ) -> Result<Option<(StageAssignmentRequest, StageAssignmentDefinition)>, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for stage assignment")?;
        for stage_id in &state.active_objective_stage_ids {
            let Some(stage) = self
                .content
                .beats
                .stages
                .iter()
                .find(|stage| &stage.id == stage_id)
            else {
                continue;
            };
            let Some(config) = stage.stage_assignment.as_ref() else {
                continue;
            };
            if config.initiator_actor_id.trim().is_empty()
                && config.selected_host_story_var.trim().is_empty()
                && config.remaining_host_story_var.trim().is_empty()
            {
                continue;
            }
            let selected_room_id = resolve_assignment_room(
                &state,
                &config.selected_room_story_var,
                &config.selected_room_id,
            );
            let remaining_room_id = resolve_assignment_room(
                &state,
                &config.remaining_room_story_var,
                &config.remaining_room_id,
            );
            if selected_room_id.trim().is_empty() || remaining_room_id.trim().is_empty() {
                continue;
            }
            let applied_flag = format!("stage_assignment_applied:{}", stage.id);
            if state
                .story_vars
                .get(&applied_flag)
                .is_some_and(|value| value == "true")
            {
                continue;
            }
            let mut anchored_room_assignments = BTreeMap::new();
            insert_anchored_room(
                &mut anchored_room_assignments,
                &state,
                &config.selected_host_story_var,
                &selected_room_id,
            );
            insert_anchored_room(
                &mut anchored_room_assignments,
                &state,
                &config.remaining_host_story_var,
                &remaining_room_id,
            );
            let initiator = if config.initiator_actor_id.trim().is_empty() {
                None
            } else {
                self.content.actor(&config.initiator_actor_id).cloned()
            };
            if initiator.is_none() && anchored_room_assignments.is_empty() {
                continue;
            }
            let candidates = self
                .content
                .actors
                .iter()
                .filter(|actor| {
                    let initiator_id = initiator.as_ref().map(|a| a.id.as_str()).unwrap_or("");
                    actor.id != initiator_id && !anchored_room_assignments.contains_key(&actor.id)
                })
                .map(|actor| {
                    let current_room_id = state.actor_room_id(&actor.id, &actor.room_id);
                    StageAssignmentCandidate {
                        actor_id: actor.id.clone(),
                        actor_name: display_actor_name(&state, actor),
                        current_room_id: current_room_id.to_string(),
                        current_room_title: assignment_room_title(self, current_room_id),
                        actor_stats: state.actor_stats_snapshot(&actor.id),
                        pair_stats_with_initiator: initiator
                            .as_ref()
                            .map(|initiator| state.pair_stats_snapshot(&actor.id, &initiator.id))
                            .unwrap_or_default(),
                    }
                })
                .collect::<Vec<_>>();
            if candidates.is_empty() && initiator.is_none() {
                continue;
            }
            let request = StageAssignmentRequest {
                locale: self.content.locale.clone(),
                system_text: self.content.system_text.clone(),
                stage_id: stage.id.clone(),
                selection_label: if config.selection_label.trim().is_empty() {
                    stage.summary.clone()
                } else {
                    config.selection_label.clone()
                },
                prompt_instructions: config.prompt_instructions.clone(),
                initiator_actor_id: initiator
                    .as_ref()
                    .map(|actor| actor.id.clone())
                    .unwrap_or_default(),
                initiator_actor_name: initiator
                    .as_ref()
                    .map(|actor| display_actor_name(&state, actor))
                    .unwrap_or_default(),
                selected_room_id: selected_room_id.clone(),
                selected_room_title: assignment_room_title(self, &selected_room_id),
                remaining_room_id: remaining_room_id.clone(),
                remaining_room_title: assignment_room_title(self, &remaining_room_id),
                beat_note: render_story_text(&stage.beat_note, &state),
                anchored_room_assignments,
                candidates,
            };
            return Ok(Some((request, config.clone())));
        }
        Ok(None)
    }

    fn fallback_stage_assignment(&self, request: &StageAssignmentRequest) -> StageAssignment {
        let assignments = request
            .candidates
            .iter()
            .enumerate()
            .map(|(index, candidate)| StageAssignmentScore {
                actor_id: candidate.actor_id.clone(),
                selection_score: if candidate.current_room_id == request.selected_room_id {
                    100
                } else {
                    100 - index as i32
                },
                rationale: "deterministic fallback".to_string(),
            })
            .collect();
        StageAssignment { assignments }
    }

    fn commit_stage_assignment(
        &self,
        request: &StageAssignmentRequest,
        assignment: &StageAssignment,
        config: &StageAssignmentDefinition,
    ) -> Result<String, Box<dyn Error>> {
        let mut scored = assignment.assignments.clone();
        scored.sort_by(|left, right| {
            right
                .selection_score
                .cmp(&left.selection_score)
                .then_with(|| left.actor_id.cmp(&right.actor_id))
        });
        let selected_count = scored
            .iter()
            .filter(|entry| entry.selection_score >= config.score_threshold)
            .count()
            .max(config.min_selected_actors)
            .min(config.max_selected_actors)
            .min(scored.len());
        let chosen = scored
            .into_iter()
            .take(selected_count)
            .map(|entry| entry.actor_id)
            .collect::<BTreeSet<_>>();

        let mut state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state to commit stage assignment")?;
        let applied_flag = format!("stage_assignment_applied:{}", request.stage_id);
        if state
            .story_vars
            .get(&applied_flag)
            .is_some_and(|value| value == "true")
        {
            return Ok(String::new());
        }
        let mut preplaced_rooms = request.anchored_room_assignments.clone();
        if !request.initiator_actor_id.is_empty() {
            preplaced_rooms.insert(
                request.initiator_actor_id.clone(),
                request.selected_room_id.clone(),
            );
        }
        state.story_vars.set_unchecked(&applied_flag, "true");
        if !config.group_story_var_key.trim().is_empty() {
            let mut group_ids = preplaced_rooms
                .iter()
                .filter(|(_, room_id)| *room_id == &request.selected_room_id)
                .map(|(actor_id, _)| actor_id.clone())
                .collect::<Vec<_>>();
            group_ids.extend(chosen.iter().cloned());
            group_ids.sort();
            state
                .story_vars
                .set_unchecked(&config.group_story_var_key, &group_ids.join(","));
        }
        if !config.remaining_group_story_var_key.trim().is_empty() {
            let mut remaining_ids = preplaced_rooms
                .iter()
                .filter(|(_, room_id)| *room_id == &request.remaining_room_id)
                .map(|(actor_id, _)| actor_id.clone())
                .collect::<Vec<_>>();
            remaining_ids.extend(
                request
                    .candidates
                    .iter()
                    .filter(|candidate| !chosen.contains(&candidate.actor_id))
                    .map(|candidate| candidate.actor_id.clone()),
            );
            remaining_ids.sort();
            state.story_vars.set_unchecked(
                &config.remaining_group_story_var_key,
                &remaining_ids.join(","),
            );
        }
        for (actor_id, room_id) in &preplaced_rooms {
            state
                .story_vars
                .set_unchecked(&format!("{}_assigned_room", actor_id), room_id);
        }
        for candidate in &request.candidates {
            if preplaced_rooms.contains_key(&candidate.actor_id) {
                continue;
            }
            let room_id = if chosen.contains(&candidate.actor_id) {
                &request.selected_room_id
            } else {
                &request.remaining_room_id
            };
            state
                .story_vars
                .set_unchecked(&format!("{}_assigned_room", candidate.actor_id), room_id);
        }
        let mut selected_names = preplaced_rooms
            .iter()
            .filter(|(_, room_id)| *room_id == &request.selected_room_id)
            .filter_map(|(actor_id, _)| {
                self.content
                    .actor(actor_id)
                    .map(|actor| display_actor_name(&state, actor))
            })
            .collect::<Vec<_>>();
        selected_names.extend(
            request
                .candidates
                .iter()
                .filter(|candidate| chosen.contains(&candidate.actor_id))
                .map(|candidate| candidate.actor_name.clone()),
        );
        let mut remaining_names = preplaced_rooms
            .iter()
            .filter(|(_, room_id)| *room_id == &request.remaining_room_id)
            .filter_map(|(actor_id, _)| {
                self.content
                    .actor(actor_id)
                    .map(|actor| display_actor_name(&state, actor))
            })
            .collect::<Vec<_>>();
        remaining_names.extend(
            request
                .candidates
                .iter()
                .filter(|candidate| !chosen.contains(&candidate.actor_id))
                .map(|candidate| candidate.actor_name.clone()),
        );
        let mut lines = Vec::new();
        if !config.initiator_line_template.trim().is_empty() {
            lines.push(self.content.render_template(
                &config.initiator_line_template,
                &[
                    ("initiator_name", &request.initiator_actor_name),
                    ("selection_label", &request.selection_label),
                    ("selected_room_title", &request.selected_room_title),
                    ("remaining_room_title", &request.remaining_room_title),
                ],
            ));
        }
        if !selected_names.is_empty() && !config.selected_line_template.trim().is_empty() {
            let selected_names = join_with_and(&selected_names);
            lines.push(self.content.render_template(
                &config.selected_line_template,
                &[
                    ("selected_names", &selected_names),
                    ("selection_label", &request.selection_label),
                    ("selected_room_title", &request.selected_room_title),
                    ("remaining_room_title", &request.remaining_room_title),
                ],
            ));
        }
        if !remaining_names.is_empty() && !config.remaining_line_template.trim().is_empty() {
            let remaining_names = join_with_and(&remaining_names);
            lines.push(self.content.render_template(
                &config.remaining_line_template,
                &[
                    ("remaining_names", &remaining_names),
                    ("selection_label", &request.selection_label),
                    ("selected_room_title", &request.selected_room_title),
                    ("remaining_room_title", &request.remaining_room_title),
                ],
            ));
        }
        Ok(lines.join(" "))
    }
}

fn resolve_assignment_room(
    state: &crate::engine::state::WorldState,
    story_var: &str,
    default_room_id: &str,
) -> String {
    if story_var.is_empty() {
        default_room_id.to_string()
    } else {
        state
            .story_vars
            .get(story_var)
            .map(str::to_string)
            .unwrap_or_else(|| default_room_id.to_string())
    }
}

fn insert_anchored_room(
    assignments: &mut BTreeMap<String, String>,
    state: &crate::engine::state::WorldState,
    story_var: &str,
    room_id: &str,
) {
    if !story_var.trim().is_empty()
        && let Some(host_id) = state.story_vars.get(story_var)
        && !host_id.trim().is_empty()
    {
        assignments.insert(host_id.to_string(), room_id.to_string());
    }
}

fn assignment_room_title(runtime: &CinderRuntime, room_id: &str) -> String {
    runtime
        .content
        .room(room_id)
        .map(|room| room.title.clone())
        .unwrap_or_else(|| room_id.to_string())
}

fn join_with_and(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [only] => only.clone(),
        [first, second] => format!("{first} and {second}"),
        _ => {
            let mut result = items[..items.len() - 1].join(", ");
            result.push_str(", and ");
            result.push_str(&items[items.len() - 1]);
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::engine::runtime::stage_assignment_fixtures::{
        activity_split_assignment, activity_split_state, assert_activity_split_assignment,
        assert_dinner_prep_assignment, dinner_prep_assignment, stage_assignment_runtime,
        stage_assignment_test_pack,
    };
    use crate::engine::state::{GamePhase, TurnOutcome, WorldState};

    #[test]
    fn stage_assignment_caps_selected_actors_and_marks_stage_complete() {
        let content = stage_assignment_test_pack();
        let state = WorldState::new(&content);
        let (runtime, _trace_dir) =
            stage_assignment_runtime(content, state, "dinner-prep", dinner_prep_assignment());

        let first = runtime
            .apply_stage_assignments(TurnOutcome {
                text: String::new(),
                phase: GamePhase::Active,
                lines: Vec::new(),
            })
            .expect("apply assignment");
        let exported = runtime.export_state().expect("export state");

        assert_dinner_prep_assignment(&first, &exported);

        let second = runtime
            .apply_stage_assignments(TurnOutcome {
                text: String::new(),
                phase: GamePhase::Active,
                lines: Vec::new(),
            })
            .expect("reapply assignment");
        assert!(second.text.is_empty());
    }

    #[test]
    fn stage_assignment_assigns_activity_hosts_via_story_vars_only() {
        let content = stage_assignment_test_pack();
        let state = activity_split_state(&content);
        let (runtime, _trace_dir) = stage_assignment_runtime(
            content,
            state,
            "activity-split",
            activity_split_assignment(),
        );

        runtime
            .apply_stage_assignments(TurnOutcome {
                text: String::new(),
                phase: GamePhase::Active,
                lines: Vec::new(),
            })
            .expect("apply assignment");
        let exported = runtime.export_state().expect("export state");
        assert_activity_split_assignment(&exported);
    }
}
