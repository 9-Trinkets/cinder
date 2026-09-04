use crate::content::types::{ActorDefinition, CommandTargetMode, ContentPack};
use crate::engine::dialogue::{
    ActorTurnAffordanceTarget, ActorTurnConsumeCandidate, ActorTurnSpeakCandidate,
    build_actor_turn_affordance_option,
};

use super::affordances::ActorAffordanceCandidate;
use super::context::ActorTurnInspectFeatureCandidate;

pub(crate) fn extend_with_scripted_npc_actions(
    content: &ContentPack,
    actor: &ActorDefinition,
    current_room_id: &str,
    speak_candidates: &[ActorTurnSpeakCandidate],
    consume_candidates: &[ActorTurnConsumeCandidate],
    inspect_feature_cands: &[ActorTurnInspectFeatureCandidate],
    affordance_candidates: &mut Vec<ActorAffordanceCandidate>,
) {
    if content.actions.is_empty() {
        return;
    }
    for action in content
        .actions
        .iter()
        .filter(|a| a.npc.is_some() && a.command == "FOLLOW")
    {
        if let Some(npc) = &action.npc
            && let Some(ref scope) = npc.actor_scope
            && !scope.iter().any(|id| id == &actor.id)
        {
            continue;
        }
        if !action.available.allowed_rooms.is_empty()
            && !action.available.allowed_rooms.iter().any(|r| r == current_room_id)
        {
            continue;
        }
        if affordance_candidates
            .iter()
            .any(|c| c.option.affordance_id == action.id)
        {
            continue;
        }
        match action.target_mode {
            CommandTargetMode::Actor | CommandTargetMode::ActorOptional => {
                let npc = action.npc.as_ref().unwrap();
                affordance_candidates.extend(speak_candidates.iter().map(|candidate| {
                    ActorAffordanceCandidate {
                        order: action.ui.sort_order,
                        visible_by_default: npc.visible_by_default,
                        option: build_actor_turn_affordance_option(
                            &content.system_text,
                            &action.id,
                            &action.group,
                            &npc.prompt_verb,
                            (!npc.prompt_reply_verb.is_empty())
                                .then_some(npc.prompt_reply_verb.as_str()),
                            action,
                            ActorTurnAffordanceTarget::Hug {
                                actor_id: &candidate.actor_id,
                                actor_name: &candidate.actor_name,
                            },
                        ),
                    }
                }));
            }
            _ => {}
        }
    }
    for action in content
        .actions
        .iter()
        .filter(|a| a.npc.is_some() && a.command != "FOLLOW")
    {
        if let Some(npc) = &action.npc {
            if let Some(ref scope) = npc.actor_scope
                && !scope.iter().any(|id| id == &actor.id)
            {
                continue;
            }
            if !action.available.allowed_rooms.is_empty()
                && !action.available.allowed_rooms.iter().any(|r| r == current_room_id)
            {
                continue;
            }
            if affordance_candidates
                .iter()
                .any(|c| c.option.affordance_id == action.id)
            {
                continue;
            }
            match action.target_mode {
                CommandTargetMode::None => {
                    affordance_candidates.push(ActorAffordanceCandidate {
                        order: action.ui.sort_order,
                        visible_by_default: npc.visible_by_default,
                        option: build_actor_turn_affordance_option(
                            &content.system_text,
                            &action.id,
                            &action.group,
                            &npc.prompt_verb,
                            None,
                            action,
                            ActorTurnAffordanceTarget::Act,
                        ),
                    });
                }
                CommandTargetMode::Actor | CommandTargetMode::ActorOptional => {
                    affordance_candidates.extend(speak_candidates.iter().map(|candidate| {
                        ActorAffordanceCandidate {
                            order: action.ui.sort_order,
                            visible_by_default: npc.visible_by_default,
                            option: build_actor_turn_affordance_option(
                                &content.system_text,
                                &action.id,
                                &action.group,
                                &npc.prompt_verb,
                                (!npc.prompt_reply_verb.is_empty())
                                    .then_some(npc.prompt_reply_verb.as_str()),
                                action,
                                ActorTurnAffordanceTarget::Hug {
                                    actor_id: &candidate.actor_id,
                                    actor_name: &candidate.actor_name,
                                },
                            ),
                        }
                    }));
                }
                CommandTargetMode::Consumable => {
                    affordance_candidates.extend(consume_candidates.iter().filter_map(
                        |candidate| {
                            if action
                                .consumable_kind
                                .is_some_and(|kind| kind != candidate.kind)
                            {
                                return None;
                            }
                            Some(ActorAffordanceCandidate {
                                order: action.ui.sort_order,
                                visible_by_default: npc.visible_by_default,
                                option: build_actor_turn_affordance_option(
                                    &content.system_text,
                                    &action.id,
                                    &action.group,
                                    &npc.prompt_verb,
                                    None,
                                    action,
                                    ActorTurnAffordanceTarget::Consume {
                                        item_id: &candidate.item_id,
                                        item_label: &candidate.item_label,
                                        feature_label: &candidate.feature_label,
                                        kind: candidate.kind,
                                    },
                                ),
                            })
                        },
                    ));
                }
                CommandTargetMode::Feature => {
                    affordance_candidates.extend(inspect_feature_cands.iter().map(
                        |candidate| ActorAffordanceCandidate {
                            order: action.ui.sort_order,
                            visible_by_default: npc.visible_by_default,
                            option: build_actor_turn_affordance_option(
                                &content.system_text,
                                &action.id,
                                &action.group,
                                &npc.prompt_verb,
                                None,
                                action,
                                ActorTurnAffordanceTarget::InspectFeature {
                                    feature_id: &candidate.feature_id,
                                    feature_label: &candidate.label,
                                },
                            ),
                        },
                    ));
                }
                CommandTargetMode::Room | CommandTargetMode::ContextLabel => {}
            }
        }
    }
}
