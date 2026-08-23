use crate::content::types::{
    ActorDefinition, ActorMovementRulesDefinition, CommandTargetMode, ContentPack,
};
use crate::engine::actor_tick::decide_movement;
use crate::engine::dialogue::{
    ActorTurnActionRequest, ActorTurnAffordanceTarget, ActorTurnCommandInvocation,
    ActorTurnSpeakCandidate, build_actor_turn_affordance_option,
};
use crate::engine::dialogue_grounding::{
    current_objective_beat_notes, latest_other_person_message, recent_exchange_memory,
};
use crate::engine::events::WorldEvent;
use crate::engine::hooks::{actor_state_notes, pair_state_note};
use crate::engine::state::WorldState;
use crate::engine::turn_policies::{action_is_available, apply_actor_turn_policies};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::sync::Arc;

use super::affordances::{
    ActorAffordanceCandidate, require_actor_affordance_for_command_id,
    require_actor_affordance_for_consumable_kind,
};
use super::context::{
    SpeakCandidateContext, actors_in_room_except, available_consume_candidates,
    inspect_actor_candidates, inspect_feature_candidates,
    preferred_hunger_recovery_consume_item_id, recovery_context_label,
    reply_pending_from_candidate,
};
use super::dialogue::{actor_turn_setting_notes, recent_actor_turn_memory};
use super::movement::{
    MovementSuppressionContext, exploration_move_target, is_actor_movement_locked,
    pair_stats_move_target,
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
            .map(|candidate| SpeakCandidateContext {
                actor: candidate,
                latest_message: latest_other_person_message(state, &actor.id, &candidate.id),
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
    let inspect_feature_cands =
        inspect_feature_candidates(content.as_ref(), state, actor, &current_room_id);
    let inspect_actor_cands = inspect_actor_candidates(state, actor, &talk_candidate_contexts);
    let hide_move = is_actor_movement_locked(
        content.as_ref(),
        state,
        &actor.id,
        &MovementSuppressionContext {
            rest_available: rest_context.is_some(),
            eat_option_count: consume_candidates
                .iter()
                .filter(|candidate| candidate.kind == crate::content::types::ConsumableKind::Eat)
                .count(),
            drink_option_count: consume_candidates
                .iter()
                .filter(|candidate| candidate.kind == crate::content::types::ConsumableKind::Drink)
                .count(),
            consume_option_count: consume_candidates
                .iter()
                .filter(|candidate| {
                    candidate.kind == crate::content::types::ConsumableKind::Consume
                })
                .count(),
        },
    )?;
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
                affordances: std::collections::BTreeMap::new(),
                interaction_note: pair_state_note(
                    content.as_ref(),
                    state,
                    &actor.id,
                    &candidate.actor.id,
                    &candidate.actor.name,
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
    let mut affordance_candidates = Vec::new();
    if let Some((room_id, room_title)) = move_option.as_ref()
        && let Some(action) = content.action("move")
    {
        let npc = action.npc.as_ref();
        affordance_candidates.push(ActorAffordanceCandidate::new(
            action,
            build_actor_turn_affordance_option(
                &content.system_text,
                &action.id,
                &action.group,
                npc.map_or("", |n| n.prompt_verb.as_str()),
                None,
                action,
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
    if let Some(action) = content.action("speak") {
        let npc = action.npc.as_ref();
        affordance_candidates.extend(speak_candidates.iter().map(|candidate| {
            ActorAffordanceCandidate::new(
                action,
                build_actor_turn_affordance_option(
                    &content.system_text,
                    &action.id,
                    &action.group,
                    npc.map_or("", |n| n.prompt_verb.as_str()),
                    npc.and_then(|n| {
                        if n.prompt_reply_verb.is_empty() {
                            None
                        } else {
                            Some(n.prompt_reply_verb.as_str())
                        }
                    }),
                    action,
                    ActorTurnAffordanceTarget::Speak {
                        actor_id: &candidate.actor_id,
                        actor_name: &candidate.actor_name,
                        reply_now: candidate.reply_now,
                    },
                ),
            )
        }));
        if speak_candidates.len() >= content.speech.room_min_audience {
            affordance_candidates.push(ActorAffordanceCandidate::new(
                action,
                build_actor_turn_affordance_option(
                    &content.system_text,
                    &action.id,
                    &action.group,
                    npc.map_or("", |n| n.prompt_verb.as_str()),
                    None,
                    action,
                    ActorTurnAffordanceTarget::SpeakRoom {
                        audience_label: "everyone here",
                    },
                ),
            ));
        }
    }
    if let Some(action) = content.action("hug") {
        let npc = action.npc.as_ref();
        affordance_candidates.extend(speak_candidates.iter().map(|candidate| {
            ActorAffordanceCandidate::new(
                action,
                build_actor_turn_affordance_option(
                    &content.system_text,
                    &action.id,
                    &action.group,
                    npc.map_or("", |n| n.prompt_verb.as_str()),
                    None,
                    action,
                    ActorTurnAffordanceTarget::Hug {
                        actor_id: &candidate.actor_id,
                        actor_name: &candidate.actor_name,
                    },
                ),
            )
        }));
    }
    if let Some(context_label) = rest_context.as_deref()
        && let Ok(action) = require_actor_affordance_for_command_id(content.as_ref(), "rest")
    {
        let npc = action.npc.as_ref();
        affordance_candidates.push(ActorAffordanceCandidate::new(
            action,
            build_actor_turn_affordance_option(
                &content.system_text,
                &action.id,
                &action.group,
                npc.map_or("", |n| n.prompt_verb.as_str()),
                None,
                action,
                ActorTurnAffordanceTarget::Rest { context_label },
            ),
        ));
    }
    for candidate in &consume_candidates {
        if let Ok(action) =
            require_actor_affordance_for_consumable_kind(content.as_ref(), candidate.kind)
        {
            let npc = action.npc.as_ref();
            affordance_candidates.push(ActorAffordanceCandidate::new(
                action,
                build_actor_turn_affordance_option(
                    &content.system_text,
                    &action.id,
                    &action.group,
                    npc.map_or("", |n| n.prompt_verb.as_str()),
                    None,
                    action,
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
    if let Some(action) = content.action("inspect_feature") {
        let npc = action.npc.as_ref();
        affordance_candidates.extend(inspect_feature_cands.iter().map(|candidate| {
            ActorAffordanceCandidate::new(
                action,
                build_actor_turn_affordance_option(
                    &content.system_text,
                    &action.id,
                    &action.group,
                    npc.map_or("", |n| n.prompt_verb.as_str()),
                    None,
                    action,
                    ActorTurnAffordanceTarget::InspectFeature {
                        feature_id: &candidate.feature_id,
                        feature_label: &candidate.label,
                    },
                ),
            )
        }));
    }
    if let Some(action) = content.action("inspect_actor") {
        let npc = action.npc.as_ref();
        affordance_candidates.extend(inspect_actor_cands.iter().map(|candidate| {
            ActorAffordanceCandidate::new(
                action,
                build_actor_turn_affordance_option(
                    &content.system_text,
                    &action.id,
                    &action.group,
                    npc.map_or("", |n| n.prompt_verb.as_str()),
                    None,
                    action,
                    ActorTurnAffordanceTarget::InspectActor {
                        actor_id: &candidate.actor_id,
                        actor_name: &candidate.actor_name,
                    },
                ),
            )
        }));
    }
    if let Ok(action) = require_actor_affordance_for_command_id(content.as_ref(), "act") {
        let npc = action.npc.as_ref();
        affordance_candidates.push(ActorAffordanceCandidate::new(
            action,
            build_actor_turn_affordance_option(
                &content.system_text,
                &action.id,
                &action.group,
                npc.map_or("", |n| n.prompt_verb.as_str()),
                None,
                action,
                ActorTurnAffordanceTarget::Act,
            ),
        ));
    }
    if !content.actions.is_empty() {
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
                && !action.available.allowed_rooms.contains(&current_room_id)
            {
                continue;
            }
            if !affordance_candidates
                .iter()
                .any(|c| c.option.affordance_id == action.id)
            {
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
                    && !action.available.allowed_rooms.contains(&current_room_id)
                {
                    continue;
                }
                if !affordance_candidates
                    .iter()
                    .any(|c| c.option.affordance_id == action.id)
                {
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
                            affordance_candidates.extend(speak_candidates.iter().map(
                                |candidate| {
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
                                },
                            ));
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
    }
    let hide_inspect_feature = false;
    let hide_inspect_actor = false;
    let mut affordances = affordance_candidates
        .into_iter()
        .filter(|candidate| candidate.visible_by_default)
        .filter(|candidate| {
            let ActorTurnCommandInvocation::Command { command_id, .. } =
                &candidate.option.invocation;
            content.action(command_id).is_some_and(|action| {
                action_is_available(content.as_ref(), state, action, &current_room_id)
            })
        })
        .collect::<Vec<_>>();
    affordances.sort_by(|left, right| left.order.cmp(&right.order));
    let mut request = ActorTurnActionRequest {
        actor_id: actor.id.clone(),
        actor_name: actor.name.clone(),
        locale: content.locale.clone(),
        system_text: content.system_text.clone(),
        character_notes: actor.prompt_context.character_notes.clone(),
        setting_notes: actor_turn_setting_notes(content.as_ref(), state, actor, &current_room_id),
        current_beat_notes: current_objective_beat_notes(content.as_ref(), state, Some(&actor.id)),
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
        has_food_consumable: consume_candidates
            .iter()
            .any(|candidate| candidate.kind == crate::content::types::ConsumableKind::Eat),
        has_cook_affordance: affordances.iter().any(|affordance| {
            matches!(
                &affordance.option.invocation,
                ActorTurnCommandInvocation::Command {
                    command_id, ..
                } if command_id == "cook" || command_id == "brew"
            )
        }),
        cooking_needed: state
            .story_vars
            .get("cooking_needed")
            .map(|v| v == "true")
            .unwrap_or(false),
        food_stock: consume_candidates
            .iter()
            .filter(|c| c.kind == crate::content::types::ConsumableKind::Eat)
            .count(),
        actor_count: state.actor_stats.len(),
        consume_target_item_id: preferred_hunger_recovery_consume_item_id(&consume_candidates),
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
    apply_actor_turn_policies(content.as_ref(), state, &mut request);
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

#[cfg(test)]
mod tests {
    use super::build_actor_turn;
    use crate::content::loader::load_named_pack;
    use crate::engine::state::WorldState;
    use std::sync::Arc;

    #[test]
    fn custom_actor_target_room_commands_keep_actor_targets() {
        let content = load_named_pack("aera", Some("en")).expect("load aera");
        let mut state = WorldState::new(&content);
        state.active_objective_stage_ids =
            vec!["day2-event-a".to_string(), "day2-event-b".to_string()];
        state.story_vars.set_unchecked("day2_group_a", "aera,mio");
        state.story_vars.set_unchecked("day2_group_b", "ren,daichi");
        state
            .actor_room_overrides
            .insert("aera".to_string(), "studio".to_string());
        state
            .actor_room_overrides
            .insert("mio".to_string(), "studio".to_string());
        state
            .actor_room_overrides
            .insert("ren".to_string(), "patio".to_string());
        state
            .actor_room_overrides
            .insert("daichi".to_string(), "patio".to_string());

        let actor = content.actor("aera").expect("aera actor");
        let rules = content.movement_rules("aera");
        let build = build_actor_turn(Arc::new(content.clone()), &state, actor, &rules)
            .expect("build actor turn");

        assert!(build.request.affordances.iter().any(|affordance| {
            affordance.command_id == "edit"
                && affordance.decision_label == "EDIT mio"
                && matches!(
                    &affordance.invocation,
                    crate::engine::dialogue::ActorTurnCommandInvocation::Command {
                        target_actor_id: Some(target_actor_id),
                        ..
                    } if target_actor_id == "mio"
                )
        }));
        assert!(!build.request.affordances.iter().any(|affordance| {
            affordance.command_id == "edit" && affordance.decision_label == "EDIT"
        }));
    }
}
