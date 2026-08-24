use crate::content::types::{
    ActionDefinition, ContentPack, RuleBundleAffordanceTarget, RuleBundleCompletionTrigger,
    RuleBundleConditionalGuidanceDefinition, RuleBundleDefinition, RuleBundleProgressRef,
};
use crate::engine::dialogue::{ActorTurnActionRequest, ActorTurnCommandInvocation};
use crate::engine::state::WorldState;

const BUNDLE_ACTOR_COMPLETE_STORY_VAR_PREFIX: &str = "rule_bundle:actor_complete";
const BUNDLE_PROGRESS_STORY_VAR_PREFIX: &str = "rule_bundle:progress";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CommandAvailabilityIssue {
    StageInactive,
    MissingBundleProgress(Vec<String>),
    BlockedByBundleProgress(Vec<String>),
}

pub(crate) fn apply_actor_turn_policies(
    content: &ContentPack,
    state: &WorldState,
    request: &mut ActorTurnActionRequest,
) {
    let actor_id = request.actor_id.clone();
    for bundle in active_bundles(content, state)
        .filter(|bundle| bundle_applies_to_actor(content, state, bundle, &actor_id))
    {
        let notes = bundle_guidance_notes_for_actor(content, state, bundle, &actor_id);
        request.current_beat_notes.extend(notes);
        apply_bundle_affordance_priorities(bundle, content, state, request);
    }
}

pub(crate) fn actor_bundle_guidance_notes(
    content: &ContentPack,
    state: &WorldState,
    actor_id: &str,
) -> Vec<String> {
    active_bundles(content, state)
        .filter(|bundle| bundle_applies_to_actor(content, state, bundle, actor_id))
        .flat_map(|bundle| bundle_guidance_notes_for_actor(content, state, bundle, actor_id))
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BundleSpeechEvent {
    ToActor,
    ToRoom,
}

pub(crate) fn mark_actor_bundle_progress_for_speech_event(
    content: &ContentPack,
    state: &mut WorldState,
    actor_id: &str,
    event: BundleSpeechEvent,
) {
    let keys = active_bundles(content, state)
        .filter(|bundle| bundle_applies_to_actor(content, state, bundle, actor_id))
        .filter(|bundle| {
            bundle
                .completion
                .mark_actor_complete_on
                .iter()
                .any(|trigger| speech_trigger_matches(*trigger, event))
        })
        .map(|bundle| bundle_actor_complete_key(&bundle.id, actor_id))
        .collect::<Vec<_>>();
    for key in keys {
        state.story_vars.set_unchecked(&key, "true");
    }
}

pub(crate) fn command_availability_issue(
    content: &ContentPack,
    state: &WorldState,
    action: &ActionDefinition,
) -> Option<CommandAvailabilityIssue> {
    let a = &action.available;
    if !a.available_during.is_empty()
        && !a
            .available_during
            .iter()
            .any(|stage_id| state.active_objective_stage_ids.contains(stage_id))
    {
        return Some(CommandAvailabilityIssue::StageInactive);
    }

    let missing = a
        .required_bundle_progress
        .iter()
        .filter(|progress| !bundle_progress_is_met(content, state, progress))
        .map(|progress| bundle_progress_label(content, progress))
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Some(CommandAvailabilityIssue::MissingBundleProgress(missing));
    }

    let blocked = a
        .blocked_by_bundle_progress
        .iter()
        .filter(|progress| bundle_progress_is_met(content, state, progress))
        .map(|progress| bundle_progress_label(content, progress))
        .collect::<Vec<_>>();
    if !blocked.is_empty() {
        return Some(CommandAvailabilityIssue::BlockedByBundleProgress(blocked));
    }

    None
}

pub(crate) fn command_unavailable_message(
    _content: &ContentPack,
    action: &ActionDefinition,
    issue: &CommandAvailabilityIssue,
) -> String {
    let verb = action.command.to_ascii_lowercase();
    match issue {
        CommandAvailabilityIssue::StageInactive => format!("You can't {verb} right now."),
        CommandAvailabilityIssue::MissingBundleProgress(labels) => {
            format!("You can't {verb} yet. Still needed: {}.", labels.join(", "))
        }
        CommandAvailabilityIssue::BlockedByBundleProgress(labels) => format!(
            "You can't {verb} again right now. Already ready: {}.",
            labels.join(", ")
        ),
    }
}

pub(crate) fn apply_command_bundle_progress_effects(
    state: &mut WorldState,
    action: &ActionDefinition,
) {
    for progress in &action.sets_bundle_progress {
        state.story_vars.set_unchecked(
            &bundle_progress_story_var_key(&progress.bundle_id, &progress.key),
            "true",
        );
    }
    for progress in &action.clears_bundle_progress {
        state
            .story_vars
            .values_mut()
            .remove(&bundle_progress_story_var_key(
                &progress.bundle_id,
                &progress.key,
            ));
    }
}

pub fn action_is_available(
    content: &ContentPack,
    state: &WorldState,
    action: &ActionDefinition,
    context_room_id: &str,
) -> bool {
    let a = &action.available;

    if a.requires_actor_in_room {
        let has_actor = content.actors.iter().any(|act| {
            let room_id = state.actor_room_id(&act.id, &act.room_id);
            room_id == context_room_id
        });
        if !has_actor {
            return false;
        }
    }

    if !a.allowed_rooms.is_empty() && !a.allowed_rooms.contains(&context_room_id.to_string()) {
        return false;
    }

    let room_is_flagged = state.flagged_rooms.contains(context_room_id);
    if a.requires_room_flagged && !room_is_flagged {
        return false;
    }
    if a.requires_room_unflagged && room_is_flagged {
        return false;
    }

    if !a.available_during.is_empty()
        && !a
            .available_during
            .iter()
            .any(|stage_id| state.active_objective_stage_ids.contains(stage_id))
    {
        return false;
    }

    for progress in &a.required_bundle_progress {
        if !bundle_progress_is_met(content, state, progress) {
            return false;
        }
    }

    for progress in &a.blocked_by_bundle_progress {
        if bundle_progress_is_met(content, state, progress) {
            return false;
        }
    }

    true
}

fn bundle_guidance_notes_for_actor(
    content: &ContentPack,
    state: &WorldState,
    bundle: &RuleBundleDefinition,
    actor_id: &str,
) -> Vec<String> {
    let mut notes = Vec::new();
    let completed_actor_count = content
        .actors
        .iter()
        .map(|actor| actor.id.as_str())
        .filter(|actor_id| actor_is_complete(state, bundle, actor_id))
        .count();
    if completed_actor_count < content.actors.len() {
        let actor_is_complete = actor_is_complete(state, bundle, actor_id);
        if actor_is_complete {
            if !bundle
                .guidance
                .prompt_note_if_others_incomplete
                .trim()
                .is_empty()
            {
                notes.push(bundle.guidance.prompt_note_if_others_incomplete.clone());
            }
        } else if !bundle
            .guidance
            .prompt_note_if_actor_incomplete
            .trim()
            .is_empty()
        {
            notes.push(bundle.guidance.prompt_note_if_actor_incomplete.clone());
        }
    }
    for conditional in matching_conditional_guidance(content, state, bundle) {
        if !conditional.prompt_note.trim().is_empty() {
            notes.push(conditional.prompt_note.clone());
        }
    }
    notes
}

fn apply_bundle_affordance_priorities(
    bundle: &RuleBundleDefinition,
    content: &ContentPack,
    state: &WorldState,
    request: &mut ActorTurnActionRequest,
) {
    let priorities = bundle
        .guidance
        .conditional
        .iter()
        .filter(|conditional| conditional_guidance_matches(content, state, conditional))
        .flat_map(|conditional| conditional.prioritize.iter())
        .chain(bundle.guidance.prioritize.iter())
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
                RuleBundleAffordanceTarget::Any => true,
                RuleBundleAffordanceTarget::Actor => target_actor_id.is_some(),
                RuleBundleAffordanceTarget::Room => target_actor_id.is_none(),
            };
            if *command_id == priority.command_id && target_matches {
                return index;
            }
        }
        usize::MAX
    });
}

fn active_bundles<'a>(
    content: &'a ContentPack,
    state: &WorldState,
) -> impl Iterator<Item = &'a RuleBundleDefinition> {
    content.rule_bundles.bundles.iter().filter(|bundle| {
        bundle_stage_ids(bundle).into_iter().any(|stage_id| {
            state
                .active_objective_stage_ids
                .iter()
                .any(|active| active == stage_id)
        })
    })
}

fn bundle_applies_to_actor(
    content: &ContentPack,
    state: &WorldState,
    bundle: &RuleBundleDefinition,
    actor_id: &str,
) -> bool {
    let active_stages = bundle_stage_ids(bundle)
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

pub(crate) fn clear_inactive_bundle_state(content: &ContentPack, state: &mut WorldState) {
    for bundle in content.rule_bundles.bundles.iter().filter(|bundle| {
        !bundle_stage_ids(bundle).into_iter().any(|stage_id| {
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
                .remove(&bundle_actor_complete_key(&bundle.id, &actor.id));
        }
        for progress in &bundle.progress.keys {
            state
                .story_vars
                .values_mut()
                .remove(&bundle_progress_story_var_key(&bundle.id, &progress.key));
        }
    }
}

fn bundle_stage_ids(bundle: &RuleBundleDefinition) -> Vec<&str> {
    bundle.stage_ids.iter().map(String::as_str).collect()
}

fn actor_is_complete(state: &WorldState, bundle: &RuleBundleDefinition, actor_id: &str) -> bool {
    state
        .story_vars
        .get(&bundle_actor_complete_key(&bundle.id, actor_id))
        .is_some_and(|value| value == "true")
}

fn bundle_actor_complete_key(bundle_id: &str, actor_id: &str) -> String {
    format!("{BUNDLE_ACTOR_COMPLETE_STORY_VAR_PREFIX}:{bundle_id}:{actor_id}")
}

fn bundle_progress_story_var_key(bundle_id: &str, key: &str) -> String {
    format!("{BUNDLE_PROGRESS_STORY_VAR_PREFIX}:{bundle_id}:{key}")
}

fn bundle_progress_is_met(
    content: &ContentPack,
    state: &WorldState,
    progress: &RuleBundleProgressRef,
) -> bool {
    content
        .rule_bundles
        .bundles
        .iter()
        .any(|bundle| bundle.id == progress.bundle_id)
        && state
            .story_vars
            .get(&bundle_progress_story_var_key(
                &progress.bundle_id,
                &progress.key,
            ))
            .is_some_and(|value| value == "true")
}

fn bundle_progress_label(content: &ContentPack, progress: &RuleBundleProgressRef) -> String {
    content
        .rule_bundles
        .bundles
        .iter()
        .find(|bundle| bundle.id == progress.bundle_id)
        .and_then(|bundle| {
            bundle
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
    bundle: &'a RuleBundleDefinition,
) -> impl Iterator<Item = &'a RuleBundleConditionalGuidanceDefinition> {
    bundle
        .guidance
        .conditional
        .iter()
        .filter(|conditional| conditional_guidance_matches(content, state, conditional))
}

fn conditional_guidance_matches(
    content: &ContentPack,
    state: &WorldState,
    conditional: &RuleBundleConditionalGuidanceDefinition,
) -> bool {
    conditional
        .required_bundle_progress
        .iter()
        .all(|progress| bundle_progress_is_met(content, state, progress))
        && conditional
            .blocked_by_bundle_progress
            .iter()
            .all(|progress| !bundle_progress_is_met(content, state, progress))
}

fn speech_trigger_matches(trigger: RuleBundleCompletionTrigger, event: BundleSpeechEvent) -> bool {
    matches!(
        (trigger, event),
        (
            RuleBundleCompletionTrigger::SpeechToActor,
            BundleSpeechEvent::ToActor
        ) | (
            RuleBundleCompletionTrigger::SpeechToRoom,
            BundleSpeechEvent::ToRoom
        )
    )
}
