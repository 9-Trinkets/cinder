use crate::content::types::{ActorDefinition, ActorMovementRulesDefinition, ContentPack};
use crate::engine::actor_tick::decide_movement;
use crate::engine::dialogue::{
    ActorTurnActionRequest, ActorTurnAffordanceTarget, ActorTurnSpeakCandidate,
    build_actor_turn_affordance_option,
};
use crate::engine::dialogue_grounding::{
    current_objective_beat_notes, latest_other_person_message, recent_exchange_memory,
};
use crate::engine::events::WorldEvent;
use crate::engine::hooks::{
    ActorTurnGuidanceInput, ActorTurnGuidanceSpeakCandidateInput, CandidateAffordanceInput,
    actor_state_notes, actor_turn_guidance, conversation_candidate_assessment, pair_state_note,
};
use crate::engine::neuron::WorkflowDefinition;
use crate::engine::state::WorldState;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::sync::Arc;

use super::affordances::{
    ActorAffordanceCandidate, guidance_affordance_inputs, require_actor_affordance_for_command_id,
    require_actor_affordance_for_consumable_kind, require_affordance_command,
};
use super::context::{
    HiddenAffordanceAction, SpeakCandidateContext, actors_in_room_except,
    available_consume_candidates, hidden_exploration_actions, inspect_actor_candidates,
    inspect_feature_candidates, preferred_hunger_recovery_consume_item_id, recovery_context_label,
    reply_pending_from_candidate,
};
use super::dialogue::{actor_turn_setting_notes, recent_actor_turn_memory};
use super::movement::{
    exploration_move_target, pair_stats_move_target, resolved_movement_rule_target_room_id,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorTurnRealizationContext {
    pub current_room_id: String,
    pub hide_move: bool,
    pub hide_inspect_feature: bool,
    pub hide_inspect_actor: bool,
    pub move_events: Vec<WorldEvent>,
    pub talk_targets: Vec<ActorTurnTargetContext>,
    pub inspect_actor_targets: Vec<ActorTurnTargetContext>,
    pub inspect_feature_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorTurnTargetContext {
    pub actor_id: String,
    pub actor_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorTurnBuildOutput {
    pub request: ActorTurnActionRequest,
    pub realization_context: ActorTurnRealizationContext,
}

pub fn build_actor_turn(
    content: Arc<ContentPack>,
    workflow: &WorkflowDefinition,
    state: &WorldState,
    actor: &ActorDefinition,
    rules: &ActorMovementRulesDefinition,
) -> Result<ActorTurnBuildOutput, Box<dyn Error>> {
    if !content.settings.autonomous_actor_dialogue {
        return Err(Box::new(std::io::Error::other(
            "build_actor_turn requires autonomous_actor_dialogue=true",
        )));
    }
    let current_room_id = state.actor_room_id(&actor.id, &actor.room_id).to_string();
    let mut talk_candidate_contexts =
        actors_in_room_except(content.as_ref(), state, &current_room_id, &actor.id)
            .into_iter()
            .filter_map(|candidate| {
                let evaluation = conversation_candidate_assessment(
                    content.as_ref(),
                    state,
                    &actor.id,
                    &candidate.id,
                );
                evaluation.visible.then_some(SpeakCandidateContext {
                    actor: candidate,
                    latest_message: latest_other_person_message(state, &actor.id, &candidate.id),
                    evaluation,
                })
            })
            .collect::<Vec<_>>();
    talk_candidate_contexts.sort_by(|left, right| {
        let left_reply_now =
            reply_pending_from_candidate(state, actor.id.as_str(), left.actor.id.as_str());
        let right_reply_now =
            reply_pending_from_candidate(state, actor.id.as_str(), right.actor.id.as_str());
        right_reply_now
            .cmp(&left_reply_now)
            .then_with(|| left.actor.name.cmp(&right.actor.name))
    });
    let actor_stats = state.actor_stats_snapshot(&actor.id);
    let rest_context = recovery_context_label(content.as_ref(), &current_room_id);
    let consume_candidates =
        available_consume_candidates(content.as_ref(), state, actor, &current_room_id);
    let hidden_actions = hidden_exploration_actions(
        content.as_ref(),
        &actor_stats,
        rest_context.is_some(),
        consume_candidates
            .iter()
            .filter(|candidate| candidate.kind == crate::content::types::ConsumableKind::Eat)
            .count(),
        consume_candidates
            .iter()
            .filter(|candidate| candidate.kind == crate::content::types::ConsumableKind::Drink)
            .count(),
        consume_candidates
            .iter()
            .filter(|candidate| candidate.kind == crate::content::types::ConsumableKind::Consume)
            .count(),
    )
    .map_err(std::io::Error::other)?;
    let hide_move = hidden_actions.contains(&HiddenAffordanceAction::Move);
    let hide_inspect_feature = hidden_actions.contains(&HiddenAffordanceAction::InspectFeature);
    let hide_inspect_actor = hidden_actions.contains(&HiddenAffordanceAction::InspectActor);
    let inspect_feature_cands = if hide_inspect_feature {
        Vec::new()
    } else {
        inspect_feature_candidates(content.as_ref(), state, actor, &current_room_id)
    };
    let inspect_actor_cands = if hide_inspect_actor {
        Vec::new()
    } else {
        inspect_actor_candidates(state, actor, &talk_candidate_contexts)
    };
    let move_target = (!hide_move)
        .then(|| {
            exploration_move_target(content.as_ref(), state, actor, &current_room_id).or_else(
                || pair_stats_move_target(content.as_ref(), state, actor, &current_room_id),
            )
        })
        .flatten();
    let move_events = if hide_move {
        Vec::new()
    } else {
        decide_movement(
            Arc::clone(&content),
            workflow,
            state,
            actor,
            rules,
            &current_room_id,
            move_target.as_ref().map(|target| target.room_id.as_str()),
        )?
    };
    let move_option = move_events.iter().find_map(|event| match event {
        WorldEvent::ActorMoved { to_room_id, .. } => content
            .room(to_room_id)
            .map(|room| (to_room_id.clone(), room.title.clone())),
        _ => None,
    });
    let speak_candidates = talk_candidate_contexts
        .iter()
        .map(|candidate| {
            let pair_stats = state.pair_stats_snapshot(&actor.id, &candidate.actor.id);
            ActorTurnSpeakCandidate {
                actor_id: candidate.actor.id.clone(),
                actor_name: candidate.actor.name.clone(),
                reply_now: reply_pending_from_candidate(
                    state,
                    actor.id.as_str(),
                    candidate.actor.id.as_str(),
                ),
                pair_stats,
                affordances: candidate
                    .evaluation
                    .affordances
                    .iter()
                    .map(|(id, affordance)| (id.clone(), affordance.available))
                    .collect(),
                interaction_note: pair_state_note(
                    content.as_ref(),
                    state,
                    &actor.id,
                    &candidate.actor.id,
                    &candidate.actor.name,
                    &candidate.evaluation.affordances,
                ),
                recent_summary: state
                    .conversation_summary(&actor.id, &candidate.actor.id)
                    .map(str::to_string),
                recent_memory: recent_exchange_memory(
                    state,
                    &actor.id,
                    &candidate.actor.id,
                    candidate.latest_message.as_deref(),
                ),
                latest_message: candidate.latest_message.clone(),
            }
        })
        .collect::<Vec<_>>();
    let unmet_speak_candidate_count = talk_candidate_contexts
        .iter()
        .filter(|candidate| {
            !state
                .conversation_history(&actor.id, &candidate.actor.id)
                .iter()
                .any(|line| line.kind == crate::engine::state::ConversationMemoryKind::Speech)
        })
        .count();
    let unvisited_room_count = content
        .rooms
        .iter()
        .filter(|room| !state.actor_has_visited_room(&actor.id, &room.id))
        .count();
    let unseen_feature_count = content
        .room(&current_room_id)
        .map(|room| {
            room.features
                .iter()
                .filter(|feature| {
                    !state.actor_has_seen_feature(&actor.id, &current_room_id, &feature.id)
                })
                .count()
        })
        .unwrap_or_default();
    let top_speak_candidate = speak_candidates.first();
    let mut affordance_candidates = Vec::new();
    if let Some((room_id, room_title)) = move_option.as_ref()
        && let Some(affordance) = content.affordance("move")
    {
        let command = require_affordance_command(content.as_ref(), affordance)?;
        affordance_candidates.push(ActorAffordanceCandidate::new(
            affordance,
            build_actor_turn_affordance_option(
                &content.system_text,
                &affordance.id,
                &affordance.group,
                &affordance.prompt_verb,
                None,
                command,
                ActorTurnAffordanceTarget::Move {
                    room_id,
                    room_title,
                    actor_name: move_target
                        .as_ref()
                        .and_then(|target| target.actor_name.as_deref()),
                },
            ),
        ));
    }
    if let Some(affordance) = content.affordance("speak") {
        let command = require_affordance_command(content.as_ref(), affordance)?;
        affordance_candidates.extend(speak_candidates.iter().map(|candidate| {
            ActorAffordanceCandidate::new(
                affordance,
                build_actor_turn_affordance_option(
                    &content.system_text,
                    &affordance.id,
                    &affordance.group,
                    &affordance.prompt_verb,
                    (!affordance.prompt_reply_verb.is_empty())
                        .then_some(affordance.prompt_reply_verb.as_str()),
                    command,
                    ActorTurnAffordanceTarget::Speak {
                        actor_id: &candidate.actor_id,
                        actor_name: &candidate.actor_name,
                        reply_now: candidate.reply_now,
                    },
                ),
            )
        }));
        if speak_candidates.len() >= 2 {
            affordance_candidates.push(ActorAffordanceCandidate::new(
                affordance,
                build_actor_turn_affordance_option(
                    &content.system_text,
                    &affordance.id,
                    &affordance.group,
                    &affordance.prompt_verb,
                    None,
                    command,
                    ActorTurnAffordanceTarget::SpeakRoom {
                        audience_label: "everyone here",
                    },
                ),
            ));
        }
    }
    if let Some(affordance) = content.affordance("hug") {
        let command = require_affordance_command(content.as_ref(), affordance)?;
        affordance_candidates.extend(speak_candidates.iter().map(|candidate| {
            ActorAffordanceCandidate::new(
                affordance,
                build_actor_turn_affordance_option(
                    &content.system_text,
                    &affordance.id,
                    &affordance.group,
                    &affordance.prompt_verb,
                    None,
                    command,
                    ActorTurnAffordanceTarget::Hug {
                        actor_id: &candidate.actor_id,
                        actor_name: &candidate.actor_name,
                    },
                ),
            )
        }));
    }
    if let Some(context_label) = rest_context.as_deref()
        && let Ok((affordance, command)) =
            require_actor_affordance_for_command_id(content.as_ref(), "rest")
    {
        affordance_candidates.push(ActorAffordanceCandidate::new(
            affordance,
            build_actor_turn_affordance_option(
                &content.system_text,
                &affordance.id,
                &affordance.group,
                &affordance.prompt_verb,
                None,
                command,
                ActorTurnAffordanceTarget::Rest { context_label },
            ),
        ));
    }
    for candidate in &consume_candidates {
        if let Ok((affordance, command)) =
            require_actor_affordance_for_consumable_kind(content.as_ref(), candidate.kind)
        {
            affordance_candidates.push(ActorAffordanceCandidate::new(
                affordance,
                build_actor_turn_affordance_option(
                    &content.system_text,
                    &affordance.id,
                    &affordance.group,
                    &affordance.prompt_verb,
                    None,
                    command,
                    ActorTurnAffordanceTarget::Consume {
                        item_id: &candidate.item_id,
                        item_label: &candidate.item_label,
                        feature_label: &candidate.feature_label,
                        kind: candidate.kind,
                    },
                ),
            ));
        }
    }
    if let Some(affordance) = content.affordance("inspect_feature") {
        let command = require_affordance_command(content.as_ref(), affordance)?;
        affordance_candidates.extend(inspect_feature_cands.iter().map(|candidate| {
            ActorAffordanceCandidate::new(
                affordance,
                build_actor_turn_affordance_option(
                    &content.system_text,
                    &affordance.id,
                    &affordance.group,
                    &affordance.prompt_verb,
                    None,
                    command,
                    ActorTurnAffordanceTarget::InspectFeature {
                        feature_id: &candidate.feature_id,
                        feature_label: &candidate.label,
                    },
                ),
            )
        }));
    }
    if let Some(affordance) = content.affordance("inspect_actor") {
        let command = require_affordance_command(content.as_ref(), affordance)?;
        affordance_candidates.extend(inspect_actor_cands.iter().map(|candidate| {
            ActorAffordanceCandidate::new(
                affordance,
                build_actor_turn_affordance_option(
                    &content.system_text,
                    &affordance.id,
                    &affordance.group,
                    &affordance.prompt_verb,
                    None,
                    command,
                    ActorTurnAffordanceTarget::InspectActor {
                        actor_id: &candidate.actor_id,
                        actor_name: &candidate.actor_name,
                    },
                ),
            )
        }));
    }
    if let Ok((affordance, command)) =
        require_actor_affordance_for_command_id(content.as_ref(), "act")
    {
        affordance_candidates.push(ActorAffordanceCandidate::new(
            affordance,
            build_actor_turn_affordance_option(
                &content.system_text,
                &affordance.id,
                &affordance.group,
                &affordance.prompt_verb,
                None,
                command,
                ActorTurnAffordanceTarget::Act,
            ),
        ));
    }
    let guidance_affordances = guidance_affordance_inputs(content.as_ref(), &affordance_candidates);
    let guidance = actor_turn_guidance(
        content.as_ref(),
        ActorTurnGuidanceInput {
            actor_stats: actor_stats.clone(),
            affordances: guidance_affordances,
            speak_candidate_count: speak_candidates.len(),
            unmet_speak_candidate_count,
            top_speak_candidate: top_speak_candidate.map(|candidate| {
                ActorTurnGuidanceSpeakCandidateInput {
                    reply_now: candidate.reply_now,
                    pair_stats: candidate.pair_stats.clone(),
                    affordances: candidate
                        .affordances
                        .iter()
                        .map(|(id, available)| {
                            (
                                id.clone(),
                                CandidateAffordanceInput {
                                    available: *available,
                                },
                            )
                        })
                        .collect(),
                }
            }),
            active_stage_ids: state.active_objective_stage_ids.clone(),
            unvisited_room_count,
            unseen_feature_count,
        },
    );
    let mut affordances = affordance_candidates
        .into_iter()
        .filter(|candidate| {
            !guidance
                .hidden_affordance_ids
                .contains(candidate.option.affordance_id.as_str())
                && (candidate.visible_by_default
                    || guidance
                        .visible_affordance_ids
                        .contains(candidate.option.affordance_id.as_str()))
        })
        .collect::<Vec<_>>();
    affordances.sort_by(|left, right| {
        let left_priority = guidance
            .group_priorities
            .get(left.option.group.as_str())
            .copied()
            .unwrap_or_default()
            + guidance
                .affordance_priorities
                .get(left.option.affordance_id.as_str())
                .copied()
                .unwrap_or_default();
        let right_priority = guidance
            .group_priorities
            .get(right.option.group.as_str())
            .copied()
            .unwrap_or_default()
            + guidance
                .affordance_priorities
                .get(right.option.affordance_id.as_str())
                .copied()
                .unwrap_or_default();
        right_priority
            .cmp(&left_priority)
            .then_with(|| left.order.cmp(&right.order))
    });
    let request = ActorTurnActionRequest {
        actor_id: actor.id.clone(),
        actor_name: actor.name.clone(),
        locale: content.locale.clone(),
        system_text: content.system_text.clone(),
        character_notes: actor.prompt_context.character_notes.clone(),
        setting_notes: actor_turn_setting_notes(content.as_ref(), state, actor, &current_room_id),
        current_beat_notes: current_objective_beat_notes(content.as_ref(), state),
        subtext_notes: {
            let mut notes = actor.prompt_context.subtext_notes.clone();
            notes.extend(actor_state_notes(content.as_ref(), state, &actor.id));
            notes
        },
        behavior_examples: actor.prompt_context.behavior_examples.clone(),
        actor_stats,
        has_rest_affordance: rest_context.is_some(),
        has_hunger_recovery_consumable: consume_candidates
            .iter()
            .any(|candidate| candidate.hunger_recovery > 0),
        consume_target_item_id: preferred_hunger_recovery_consume_item_id(&consume_candidates),
        has_pending_movement_target: resolved_movement_rule_target_room_id(state, rules)
            .is_some_and(|target_room_id| target_room_id != current_room_id),
        move_target_room_id: move_option.as_ref().map(|(room_id, _)| room_id.clone()),
        move_target_room_title: move_option.as_ref().map(|(_, title)| title.clone()),
        move_target_actor_name: move_target
            .as_ref()
            .and_then(|target| target.actor_name.clone()),
        move_target_social_note: move_target
            .as_ref()
            .and_then(|target| target.social_note.clone()),
        affordances: affordances
            .into_iter()
            .map(|candidate| candidate.option)
            .collect(),
        speak_candidates: speak_candidates.clone(),
        recent_memory: recent_actor_turn_memory(state, actor, &talk_candidate_contexts),
    };
    let realization_context = ActorTurnRealizationContext {
        current_room_id: current_room_id.clone(),
        hide_move,
        hide_inspect_feature,
        hide_inspect_actor,
        move_events: move_events.clone(),
        talk_targets: talk_candidate_contexts
            .iter()
            .map(|candidate| ActorTurnTargetContext {
                actor_id: candidate.actor.id.clone(),
                actor_name: candidate.actor.name.clone(),
            })
            .collect(),
        inspect_actor_targets: inspect_actor_cands
            .iter()
            .map(|candidate| ActorTurnTargetContext {
                actor_id: candidate.actor_id.clone(),
                actor_name: candidate.actor_name.clone(),
            })
            .collect(),
        inspect_feature_ids: inspect_feature_cands
            .iter()
            .map(|candidate| candidate.feature_id.clone())
            .collect(),
    };
    Ok(ActorTurnBuildOutput {
        request,
        realization_context,
    })
}
