use crate::content::types::{
    ContentPack, RuleBundleAffordanceTarget, RuleBundleCompletionTrigger, RuleBundleDefinition,
};
use crate::engine::dialogue::{ActorTurnActionRequest, ActorTurnCommandInvocation};
use crate::engine::state::WorldState;

const BUNDLE_ACTOR_COMPLETE_STORY_VAR_PREFIX: &str = "rule_bundle:actor_complete";

pub(crate) fn apply_actor_turn_policies(
    content: &ContentPack,
    state: &WorldState,
    request: &mut ActorTurnActionRequest,
) {
    for bundle in active_bundles(content, state) {
        apply_bundle_guidance(content, state, bundle, request);
    }
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

fn apply_bundle_guidance(
    content: &ContentPack,
    state: &WorldState,
    bundle: &RuleBundleDefinition,
    request: &mut ActorTurnActionRequest,
) {
    let completed_actor_count = content
        .actors
        .iter()
        .map(|actor| actor.id.as_str())
        .filter(|actor_id| actor_is_complete(state, bundle, actor_id))
        .count();
    if completed_actor_count >= content.actors.len() {
        return;
    }
    let actor_is_complete = actor_is_complete(state, bundle, &request.actor_id);
    if actor_is_complete {
        if !bundle
            .guidance
            .prompt_note_if_others_incomplete
            .trim()
            .is_empty()
        {
            request
                .current_beat_notes
                .push(bundle.guidance.prompt_note_if_others_incomplete.clone());
        }
        return;
    }
    if !bundle.guidance.prompt_note_if_actor_incomplete.trim().is_empty() {
        request
            .current_beat_notes
            .push(bundle.guidance.prompt_note_if_actor_incomplete.clone());
    }
    if bundle.guidance.prioritize.is_empty() {
        return;
    }
    request.affordances.sort_by_key(|affordance| {
        for (index, priority) in bundle.guidance.prioritize.iter().enumerate() {
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
    content
        .rule_bundles
        .bundles
        .iter()
        .filter(|bundle| {
            !bundle.stage_id.is_empty() && state.active_objective_stage_ids.contains(&bundle.stage_id)
        })
}

fn actor_is_complete(
    state: &WorldState,
    bundle: &RuleBundleDefinition,
    actor_id: &str,
) -> bool {
    state
        .story_vars
        .get(&bundle_actor_complete_key(&bundle.id, actor_id))
        .is_some_and(|value| value == "true")
}

fn bundle_actor_complete_key(bundle_id: &str, actor_id: &str) -> String {
    format!("{BUNDLE_ACTOR_COMPLETE_STORY_VAR_PREFIX}:{bundle_id}:{actor_id}")
}

fn speech_trigger_matches(trigger: RuleBundleCompletionTrigger, event: BundleSpeechEvent) -> bool {
    matches!(
        (trigger, event),
        (
            RuleBundleCompletionTrigger::SpeechToActor,
            BundleSpeechEvent::ToActor
        ) | (RuleBundleCompletionTrigger::SpeechToRoom, BundleSpeechEvent::ToRoom)
    )
}
