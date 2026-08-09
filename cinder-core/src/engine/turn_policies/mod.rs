use crate::content::types::{
    CommandDefinition, ContentPack, RuleBundleAffordanceTarget, RuleBundleCompletionTrigger,
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
    command: &CommandDefinition,
) -> Option<CommandAvailabilityIssue> {
    if !command.available_during.is_empty()
        && !command
            .available_during
            .iter()
            .any(|stage_id| state.active_objective_stage_ids.contains(stage_id))
    {
        return Some(CommandAvailabilityIssue::StageInactive);
    }

    let missing = command
        .required_bundle_progress
        .iter()
        .filter(|progress| !bundle_progress_is_met(content, state, progress))
        .map(|progress| bundle_progress_label(content, progress))
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Some(CommandAvailabilityIssue::MissingBundleProgress(missing));
    }

    let blocked = command
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

pub(crate) fn command_is_available(
    content: &ContentPack,
    state: &WorldState,
    command: &CommandDefinition,
) -> bool {
    command_availability_issue(content, state, command).is_none()
}

pub(crate) fn command_unavailable_message(
    _content: &ContentPack,
    command: &CommandDefinition,
    issue: &CommandAvailabilityIssue,
) -> String {
    let verb = command.command.to_ascii_lowercase();
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
    command: &CommandDefinition,
) {
    for progress in &command.sets_bundle_progress {
        state.story_vars.set_unchecked(
            &bundle_progress_story_var_key(&progress.bundle_id, &progress.key),
            "true",
        );
    }
    for progress in &command.clears_bundle_progress {
        state
            .story_vars
            .values_mut()
            .remove(&bundle_progress_story_var_key(
                &progress.bundle_id,
                &progress.key,
            ));
    }
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

#[cfg(test)]
mod tests {
    use super::{
        CommandAvailabilityIssue, apply_actor_turn_policies, apply_command_bundle_progress_effects,
        clear_inactive_bundle_state, command_availability_issue,
    };
    use crate::content::loader::load_named_pack;
    use crate::content::types::{CommandDefinition, CommandInputMode, RuleBundleProgressRef};
    use crate::engine::dialogue::{
        ActorTurnActionRequest, ActorTurnAffordanceOption, ActorTurnCommandInvocation,
    };
    use crate::engine::state::WorldState;
    use std::collections::BTreeMap;

    #[test]
    fn command_requires_bundle_progress_before_becoming_available() {
        let content = load_named_pack("aera", Some("en")).expect("load aera");
        let mut state = WorldState::new(&content);
        state.active_objective_stage_ids = vec!["dinner-prep".to_string()];

        let command = content.command("cook").expect("cook command");
        let issue = command_availability_issue(&content, &state, command);
        assert!(matches!(
            issue,
            Some(CommandAvailabilityIssue::MissingBundleProgress(_))
        ));

        state.story_vars.set_unchecked(
            "rule_bundle:progress:dinner-prep-cook-and-check-in:vegetables_chopped",
            "true",
        );
        state.story_vars.set_unchecked(
            "rule_bundle:progress:dinner-prep-cook-and-check-in:water_boiled",
            "true",
        );

        assert!(command_availability_issue(&content, &state, command).is_none());
    }

    #[test]
    fn command_bundle_progress_effects_set_and_clear_flags() {
        let content = load_named_pack("aera", Some("en")).expect("load aera");
        let mut state = WorldState::new(&content);
        let command = CommandDefinition {
            sets_bundle_progress: vec![RuleBundleProgressRef {
                bundle_id: "bundle".to_string(),
                key: "meal_ready".to_string(),
            }],
            clears_bundle_progress: vec![RuleBundleProgressRef {
                bundle_id: "bundle".to_string(),
                key: "water_boiled".to_string(),
            }],
            ..CommandDefinition::default()
        };
        state
            .story_vars
            .set_unchecked("rule_bundle:progress:bundle:water_boiled", "true");

        apply_command_bundle_progress_effects(&mut state, &command);

        assert_eq!(
            state
                .story_vars
                .get("rule_bundle:progress:bundle:meal_ready"),
            Some("true")
        );
        assert_eq!(
            state
                .story_vars
                .get("rule_bundle:progress:bundle:water_boiled"),
            None
        );
    }

    #[test]
    fn conditional_bundle_guidance_adds_finish_notes_and_prioritizes_terminal_actions() {
        let content = load_named_pack("aera", Some("en")).expect("load aera");
        let mut state = WorldState::new(&content);
        state.active_objective_stage_ids = vec!["dinner-prep".to_string()];
        state.story_vars.set_unchecked(
            "rule_bundle:progress:dinner-prep-cook-and-check-in:vegetables_chopped",
            "true",
        );
        state.story_vars.set_unchecked(
            "rule_bundle:progress:dinner-prep-cook-and-check-in:water_boiled",
            "true",
        );
        state.story_vars.set_unchecked(
            "rule_bundle:progress:dinner-prep-cook-and-check-in:coffee_ground",
            "true",
        );
        let mut request = ActorTurnActionRequest {
            actor_id: "aera".to_string(),
            actor_name: "Aera".to_string(),
            locale: content.locale.clone(),
            system_text: content.system_text.clone(),
            character_notes: vec![],
            setting_notes: vec![],
            current_beat_notes: vec![],
            subtext_notes: vec![],
            behavior_examples: vec![],
            actor_stats: BTreeMap::new(),
            has_rest_affordance: false,
            has_hunger_recovery_consumable: false,
            has_food_consumable: false,
            has_cook_affordance: true,
            cooking_needed: true,
            food_stock: 0,
            actor_count: content.actors.len(),
            consume_target_item_id: None,
            move_target_room_id: None,
            move_target_room_title: None,
            move_target_actor_name: None,
            move_target_social_note: None,
            affordances: vec![
                ActorTurnAffordanceOption {
                    affordance_id: "speak".to_string(),
                    command_id: "speak".to_string(),
                    group: "social".to_string(),
                    available_text: "You could speak.".to_string(),
                    decision_label: "SPEAK ROOM".to_string(),
                    decision_prefix: None,
                    invocation: ActorTurnCommandInvocation::Command {
                        command_id: "speak".to_string(),
                        target_room_id: None,
                        target_actor_id: None,
                        feature_id: None,
                        consumable_id: None,
                        context_label: None,
                        input_mode: CommandInputMode::None,
                    },
                },
                ActorTurnAffordanceOption {
                    affordance_id: "brew".to_string(),
                    command_id: "brew".to_string(),
                    group: "kitchen".to_string(),
                    available_text: "You could brew.".to_string(),
                    decision_label: "BREW".to_string(),
                    decision_prefix: None,
                    invocation: ActorTurnCommandInvocation::Command {
                        command_id: "brew".to_string(),
                        target_room_id: None,
                        target_actor_id: None,
                        feature_id: None,
                        consumable_id: None,
                        context_label: None,
                        input_mode: CommandInputMode::None,
                    },
                },
                ActorTurnAffordanceOption {
                    affordance_id: "cook".to_string(),
                    command_id: "cook".to_string(),
                    group: "kitchen".to_string(),
                    available_text: "You could cook.".to_string(),
                    decision_label: "COOK".to_string(),
                    decision_prefix: None,
                    invocation: ActorTurnCommandInvocation::Command {
                        command_id: "cook".to_string(),
                        target_room_id: None,
                        target_actor_id: None,
                        feature_id: None,
                        consumable_id: None,
                        context_label: None,
                        input_mode: CommandInputMode::None,
                    },
                },
            ],
            speak_candidates: vec![],
            recent_memory: vec![],
        };

        apply_actor_turn_policies(&content, &state, &mut request);

        assert!(
            request
                .current_beat_notes
                .iter()
                .any(|note| note.contains("someone should cook"))
        );
        assert!(
            request
                .current_beat_notes
                .iter()
                .any(|note| note.contains("brewing the coffee now"))
        );
        assert_eq!(request.affordances[0].command_id, "cook");
        assert_eq!(request.affordances[1].command_id, "brew");
    }

    #[test]
    fn clearing_inactive_bundle_state_removes_progress_for_finished_stage() {
        let content = load_named_pack("aera", Some("en")).expect("load aera");
        let mut state = WorldState::new(&content);
        state.active_objective_stage_ids = vec!["share-dinner-in-kitchen".to_string()];
        state.story_vars.set_unchecked(
            "rule_bundle:progress:dinner-prep-cook-and-check-in:vegetables_chopped",
            "true",
        );
        state.story_vars.set_unchecked(
            "rule_bundle:actor_complete:first-meeting-lounge-intros:aera",
            "true",
        );

        clear_inactive_bundle_state(&content, &mut state);

        assert_eq!(
            state
                .story_vars
                .get("rule_bundle:progress:dinner-prep-cook-and-check-in:vegetables_chopped"),
            None
        );
        assert_eq!(
            state
                .story_vars
                .get("rule_bundle:actor_complete:first-meeting-lounge-intros:aera"),
            None
        );
    }

    #[test]
    fn stage_targeted_bundles_only_apply_to_the_matching_group() {
        let content = load_named_pack("aera", Some("en")).expect("load aera");
        let mut state = WorldState::new(&content);
        state.active_objective_stage_ids =
            vec!["day2-event-a".to_string(), "day2-event-b".to_string()];
        state.story_vars.set_unchecked("day2_group_a", "aera,mio");
        state.story_vars.set_unchecked("day2_group_b", "daichi,ren");

        let aera_notes = super::actor_bundle_guidance_notes(&content, &state, "aera");
        let daichi_notes = super::actor_bundle_guidance_notes(&content, &state, "daichi");

        assert_eq!(
            aera_notes
                .iter()
                .filter(|note| note.contains("small-group activity scene"))
                .count(),
            1
        );
        assert_eq!(
            daichi_notes
                .iter()
                .filter(|note| note.contains("small-group activity scene"))
                .count(),
            1
        );
    }

    #[test]
    fn day2_event_b_bundle_prioritizes_studio_editing_actions() {
        let content = load_named_pack("aera", Some("en")).expect("load aera");
        let mut state = WorldState::new(&content);
        state.active_objective_stage_ids =
            vec!["day2-event-a".to_string(), "day2-event-b".to_string()];
        state.story_vars.set_unchecked("day2_group_a", "daichi,ren");
        state.story_vars.set_unchecked("day2_group_b", "aera,mio");

        let mut request = ActorTurnActionRequest {
            actor_id: "aera".to_string(),
            actor_name: "Aera".to_string(),
            locale: content.locale.clone(),
            system_text: content.system_text.clone(),
            character_notes: vec![],
            setting_notes: vec![],
            current_beat_notes: vec![],
            subtext_notes: vec![],
            behavior_examples: vec![],
            actor_stats: BTreeMap::new(),
            has_rest_affordance: false,
            has_hunger_recovery_consumable: false,
            has_food_consumable: false,
            has_cook_affordance: false,
            cooking_needed: false,
            food_stock: 0,
            actor_count: content.actors.len(),
            consume_target_item_id: None,
            move_target_room_id: None,
            move_target_room_title: None,
            move_target_actor_name: None,
            move_target_social_note: None,
            affordances: vec![
                ActorTurnAffordanceOption {
                    affordance_id: "speak".to_string(),
                    command_id: "speak".to_string(),
                    group: "social".to_string(),
                    available_text: "You could speak.".to_string(),
                    decision_label: "SPEAK MIO".to_string(),
                    decision_prefix: None,
                    invocation: ActorTurnCommandInvocation::Command {
                        command_id: "speak".to_string(),
                        target_room_id: None,
                        target_actor_id: Some("mio".to_string()),
                        feature_id: None,
                        consumable_id: None,
                        context_label: None,
                        input_mode: CommandInputMode::FreeformText,
                    },
                },
                ActorTurnAffordanceOption {
                    affordance_id: "edit-mio".to_string(),
                    command_id: "edit".to_string(),
                    group: "studio".to_string(),
                    available_text: "You could edit.".to_string(),
                    decision_label: "EDIT MIO".to_string(),
                    decision_prefix: None,
                    invocation: ActorTurnCommandInvocation::Command {
                        command_id: "edit".to_string(),
                        target_room_id: None,
                        target_actor_id: Some("mio".to_string()),
                        feature_id: None,
                        consumable_id: None,
                        context_label: None,
                        input_mode: CommandInputMode::None,
                    },
                },
            ],
            speak_candidates: vec![],
            recent_memory: vec![],
        };

        apply_actor_turn_policies(&content, &state, &mut request);

        assert_eq!(request.affordances[0].command_id, "edit");
    }

    #[test]
    fn multi_stage_bundle_progress_persists_across_included_stages() {
        let content = load_named_pack("isla", Some("en")).expect("load isla");
        let mut state = WorldState::new(&content);
        state.active_objective_stage_ids = vec!["reading-quarter".to_string()];
        state.story_vars.set_unchecked(
            "rule_bundle:progress:reading-service-ritual:coffee_ready",
            "true",
        );

        state.active_objective_stage_ids = vec!["request-quarter".to_string()];
        clear_inactive_bundle_state(&content, &mut state);

        assert_eq!(
            state
                .story_vars
                .get("rule_bundle:progress:reading-service-ritual:coffee_ready"),
            Some("true")
        );
    }

    #[test]
    fn ella_snack_prep_is_not_exposed_as_authored_commands() {
        let content = load_named_pack("ella", Some("en")).expect("load ella");

        for id in ["prep_snack_ingredients", "cook_snack", "serve_snack"] {
            assert!(
                content.command(id).is_none(),
                "{id} should not exist as a command"
            );
            assert!(
                content.affordance(id).is_none(),
                "{id} should not exist as an affordance"
            );
        }
    }

    #[test]
    fn isla_serve_coffee_is_available_during_reading_stage_once_brewed() {
        let content = load_named_pack("isla", Some("en")).expect("load isla");
        let mut state = WorldState::new(&content);
        state.active_objective_stage_ids = vec!["reading-quarter".to_string()];
        state.story_vars.set_unchecked(
            "rule_bundle:progress:reading-service-ritual:coffee_ready",
            "true",
        );

        let command = content
            .command("serve_coffee")
            .expect("serve coffee command");

        assert!(command_availability_issue(&content, &state, command).is_none());
    }
}
