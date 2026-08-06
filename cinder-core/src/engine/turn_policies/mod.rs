use crate::content::types::{ContentPack, FirstMeetingIntroductionBundleDefinition};
use crate::engine::dialogue::{ActorTurnActionRequest, ActorTurnCommandInvocation};
use crate::engine::state::WorldState;

const INTRO_STORY_VAR_PREFIX: &str = "rule_bundle:first_meeting:introduced";

pub(crate) fn apply_actor_turn_policies(
    content: &ContentPack,
    state: &WorldState,
    request: &mut ActorTurnActionRequest,
) {
    apply_first_meeting_introduction_policy(content, state, request);
}

pub(crate) fn mark_actor_room_introduction_if_applicable(
    content: &ContentPack,
    state: &mut WorldState,
    actor_id: &str,
) {
    let Some(bundle) = active_first_meeting_bundle(content, state) else {
        return;
    };
    let key = intro_story_var_key(&bundle.id, actor_id);
    state.story_vars.set_unchecked(&key, "true");
}

fn apply_first_meeting_introduction_policy(
    content: &ContentPack,
    state: &WorldState,
    request: &mut ActorTurnActionRequest,
) {
    let Some(bundle) = active_first_meeting_bundle(content, state) else {
        return;
    };
    let introduced_actor_count = content
        .actors
        .iter()
        .filter(|actor| actor_is_introduced(state, bundle, &actor.id))
        .count();
    if introduced_actor_count >= content.actors.len() {
        return;
    }
    let actor_is_introduced = actor_is_introduced(state, bundle, &request.actor_id);
    if actor_is_introduced {
        if !bundle.prompt_note_wait_for_others.trim().is_empty() {
            request
                .current_beat_notes
                .push(bundle.prompt_note_wait_for_others.clone());
        }
        return;
    }
    if !bundle.prompt_note_not_introduced.trim().is_empty() {
        request
            .current_beat_notes
            .push(bundle.prompt_note_not_introduced.clone());
    }
    request.affordances.sort_by_key(|affordance| {
        let speak_room = matches!(
            &affordance.invocation,
            ActorTurnCommandInvocation::Command {
                command_id,
                target_actor_id: None,
                ..
            } if command_id == "speak"
        );
        if speak_room {
            0
        } else {
            1
        }
    });
}

fn active_first_meeting_bundle<'a>(
    content: &'a ContentPack,
    state: &WorldState,
) -> Option<&'a FirstMeetingIntroductionBundleDefinition> {
    content
        .rule_bundles
        .first_meeting_introductions
        .iter()
        .find(|bundle| {
            !bundle.stage_id.is_empty() && state.active_objective_stage_ids.contains(&bundle.stage_id)
        })
}

fn actor_is_introduced(
    state: &WorldState,
    bundle: &FirstMeetingIntroductionBundleDefinition,
    actor_id: &str,
) -> bool {
    state
        .story_vars
        .get(&intro_story_var_key(&bundle.id, actor_id))
        .is_some_and(|value| value == "true")
}

fn intro_story_var_key(bundle_id: &str, actor_id: &str) -> String {
    format!("{INTRO_STORY_VAR_PREFIX}:{bundle_id}:{actor_id}")
}
