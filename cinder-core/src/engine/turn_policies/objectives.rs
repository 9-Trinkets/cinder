use crate::content::types::{
    ActionDefinition, BeatObjectiveAffordanceTarget, BeatObjectiveCompletionTrigger,
    BeatObjectiveConditionalGuidanceDefinition, BeatObjectiveDefinition, BeatObjectiveProgressRef,
    ContentPack,
};
use crate::engine::dialogue::{ActorTurnActionRequest, ActorTurnCommandInvocation};
use crate::engine::state::WorldState;

const OBJECTIVE_ACTOR_COMPLETE_STORY_VAR_PREFIX: &str = "beat_objective:actor_complete";
const OBJECTIVE_PROGRESS_STORY_VAR_PREFIX: &str = "beat_objective:progress";

pub(crate) fn apply_actor_turn_policies(
    content: &ContentPack,
    state: &WorldState,
    request: &mut ActorTurnActionRequest,
) {
    let actor_id = request.actor_id.clone();
    for objective in active_objectives(content, state)
        .filter(|objective| objective_applies_to_actor(content, state, objective, &actor_id))
    {
        let notes = objective_guidance_notes_for_actor(content, state, objective, &actor_id);
        request.current_beat_notes.extend(notes);
        apply_objective_affordance_priorities(objective, content, state, request);
    }
}

pub(crate) fn actor_objective_guidance_notes(
    content: &ContentPack,
    state: &WorldState,
    actor_id: &str,
) -> Vec<String> {
    active_objectives(content, state)
        .filter(|objective| objective_applies_to_actor(content, state, objective, actor_id))
        .flat_map(|objective| objective_guidance_notes_for_actor(content, state, objective, actor_id))
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ObjectiveSpeechEvent {
    ToActor,
    ToRoom,
}

pub(crate) fn mark_actor_objective_progress_for_speech_event(
    content: &ContentPack,
    state: &mut WorldState,
    actor_id: &str,
    event: ObjectiveSpeechEvent,
) {
    let keys = active_objectives(content, state)
        .filter(|objective| objective_applies_to_actor(content, state, objective, actor_id))
        .filter(|objective| {
            objective
                .completion
                .mark_actor_complete_on
                .iter()
                .any(|trigger| speech_trigger_matches(*trigger, event))
        })
        .map(|objective| objective_actor_complete_key(&objective.id, actor_id))
        .collect::<Vec<_>>();
    for key in keys {
        state.story_vars.set_unchecked(&key, "true");
    }
}

pub(crate) fn apply_command_objective_progress_effects(
    state: &mut WorldState,
    action: &ActionDefinition,
) {
    for progress in &action.sets_objective_progress {
        state.story_vars.set_unchecked(
            &objective_progress_story_var_key(&progress.objective_id, &progress.key),
            "true",
        );
    }
    for progress in &action.clears_objective_progress {
        state
            .story_vars
            .values_mut()
            .remove(&objective_progress_story_var_key(
                &progress.objective_id,
                &progress.key,
            ));
    }
}

fn objective_guidance_notes_for_actor(
    content: &ContentPack,
    state: &WorldState,
    objective: &BeatObjectiveDefinition,
    actor_id: &str,
) -> Vec<String> {
    let mut notes = Vec::new();
    let completed_actor_count = content
        .actors
        .iter()
        .map(|actor| actor.id.as_str())
        .filter(|actor_id| actor_is_complete(state, objective, actor_id))
        .count();
    if completed_actor_count < content.actors.len() {
        let actor_is_complete = actor_is_complete(state, objective, actor_id);
        if actor_is_complete {
            if !objective
                .guidance
                .prompt_note_if_others_incomplete
                .trim()
                .is_empty()
            {
                notes.push(objective.guidance.prompt_note_if_others_incomplete.clone());
            }
        } else if !objective
            .guidance
            .prompt_note_if_actor_incomplete
            .trim()
            .is_empty()
        {
            notes.push(objective.guidance.prompt_note_if_actor_incomplete.clone());
        }
    }
    for conditional in matching_conditional_guidance(content, state, objective) {
        if !conditional.prompt_note.trim().is_empty() {
            notes.push(conditional.prompt_note.clone());
        }
    }
    notes
}

fn apply_objective_affordance_priorities(
    objective: &BeatObjectiveDefinition,
    content: &ContentPack,
    state: &WorldState,
    request: &mut ActorTurnActionRequest,
) {
    let priorities = objective
        .guidance
        .conditional
        .iter()
        .filter(|conditional| conditional_guidance_matches(content, state, conditional))
        .flat_map(|conditional| conditional.prioritize.iter())
        .chain(objective.guidance.prioritize.iter())
        .collect::<Vec<_>>();
    if priorities.is_empty() {
        return;
    }
    request.affordances.sort_by_key(|affordance| {
        for (index, priority) in priorities.iter().enumerate() {
            let ActorTurnCommandInvocation::Command {
                command_id,
                target_actor_id,
                ..
            } = &affordance.invocation;
            let target_matches = match priority.target {
                BeatObjectiveAffordanceTarget::Any => true,
                BeatObjectiveAffordanceTarget::Actor => target_actor_id.is_some(),
                BeatObjectiveAffordanceTarget::Room => target_actor_id.is_none(),
            };
            if *command_id == priority.command_id && target_matches {
                return index;
            }
        }
        usize::MAX
    });
}

fn active_objectives<'a>(
    content: &'a ContentPack,
    state: &WorldState,
) -> impl Iterator<Item = &'a BeatObjectiveDefinition> {
    content.beat_objectives.objectives.iter().filter(|objective| {
        objective_stage_ids(objective).into_iter().any(|stage_id| {
            state
                .active_objective_stage_ids
                .iter()
                .any(|active| active == stage_id)
        })
    })
}

fn objective_applies_to_actor(
    content: &ContentPack,
    state: &WorldState,
    objective: &BeatObjectiveDefinition,
    actor_id: &str,
) -> bool {
    let active_stages = objective_stage_ids(objective)
        .into_iter()
        .filter(|stage_id| {
            state
                .active_objective_stage_ids
                .iter()
                .any(|active| active == *stage_id)
        })
        .filter_map(|stage_id| {
            content
                .beats
                .stages
                .iter()
                .find(|stage| stage.id == *stage_id)
        })
        .collect::<Vec<_>>();
    if active_stages.is_empty() {
        return true;
    }
    if active_stages
        .iter()
        .all(|stage| stage.target_actor_story_var.is_empty())
    {
        return true;
    }
    active_stages.iter().any(|stage| {
        !stage.target_actor_story_var.is_empty()
            && state
                .story_vars
                .get(&stage.target_actor_story_var)
                .map(|ids| ids.split(',').any(|id| id.trim() == actor_id))
                .unwrap_or(false)
    })
}

pub(crate) fn clear_inactive_objective_state(content: &ContentPack, state: &mut WorldState) {
    for objective in content.beat_objectives.objectives.iter().filter(|objective| {
        !objective_stage_ids(objective).into_iter().any(|stage_id| {
            state
                .active_objective_stage_ids
                .iter()
                .any(|active| active == stage_id)
        })
    }) {
        for actor in &content.actors {
            state
                .story_vars
                .values_mut()
                .remove(&objective_actor_complete_key(&objective.id, &actor.id));
        }
        for progress in &objective.progress.keys {
            state
                .story_vars
                .values_mut()
                .remove(&objective_progress_story_var_key(&objective.id, &progress.key));
        }
    }
}

fn objective_stage_ids(objective: &BeatObjectiveDefinition) -> Vec<&str> {
    objective.stage_ids.iter().map(String::as_str).collect()
}

fn actor_is_complete(state: &WorldState, objective: &BeatObjectiveDefinition, actor_id: &str) -> bool {
    state
        .story_vars
        .get(&objective_actor_complete_key(&objective.id, actor_id))
        .is_some_and(|value| value == "true")
}

fn objective_actor_complete_key(objective_id: &str, actor_id: &str) -> String {
    format!("{OBJECTIVE_ACTOR_COMPLETE_STORY_VAR_PREFIX}:{objective_id}:{actor_id}")
}

fn objective_progress_story_var_key(objective_id: &str, key: &str) -> String {
    format!("{OBJECTIVE_PROGRESS_STORY_VAR_PREFIX}:{objective_id}:{key}")
}

pub(crate) fn objective_progress_is_met(
    content: &ContentPack,
    state: &WorldState,
    progress: &BeatObjectiveProgressRef,
) -> bool {
    content
        .beat_objectives
        .objectives
        .iter()
        .any(|objective| objective.id == progress.objective_id)
        && state
            .story_vars
            .get(&objective_progress_story_var_key(
                &progress.objective_id,
                &progress.key,
            ))
            .is_some_and(|value| value == "true")
}

pub(crate) fn objective_progress_label(
    content: &ContentPack,
    progress: &BeatObjectiveProgressRef,
) -> String {
    content
        .beat_objectives
        .objectives
        .iter()
        .find(|objective| objective.id == progress.objective_id)
        .and_then(|objective| {
            objective
                .progress
                .keys
                .iter()
                .find(|entry| entry.key == progress.key)
        })
        .map(|entry| {
            if entry.label.trim().is_empty() {
                entry.key.clone()
            } else {
                entry.label.clone()
            }
        })
        .unwrap_or_else(|| progress.key.clone())
}

fn matching_conditional_guidance<'a>(
    content: &'a ContentPack,
    state: &'a WorldState,
    objective: &'a BeatObjectiveDefinition,
) -> impl Iterator<Item = &'a BeatObjectiveConditionalGuidanceDefinition> {
    objective
        .guidance
        .conditional
        .iter()
        .filter(|conditional| conditional_guidance_matches(content, state, conditional))
}

fn conditional_guidance_matches(
    content: &ContentPack,
    state: &WorldState,
    conditional: &BeatObjectiveConditionalGuidanceDefinition,
) -> bool {
    conditional
        .required_objective_progress
        .iter()
        .all(|progress| objective_progress_is_met(content, state, progress))
        && conditional
            .blocked_by_objective_progress
            .iter()
            .all(|progress| !objective_progress_is_met(content, state, progress))
}

fn speech_trigger_matches(trigger: BeatObjectiveCompletionTrigger, event: ObjectiveSpeechEvent) -> bool {
    matches!(
        (trigger, event),
        (
            BeatObjectiveCompletionTrigger::SpeechToActor,
            ObjectiveSpeechEvent::ToActor
        ) | (
            BeatObjectiveCompletionTrigger::SpeechToRoom,
            ObjectiveSpeechEvent::ToRoom
        )
    )
}
