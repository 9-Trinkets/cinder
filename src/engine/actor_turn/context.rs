use crate::content::types::{ActorDefinition, ConsumableDefinition, ConsumableKind, ContentPack};
use crate::engine::dialogue::ActorTurnConsumeCandidate;
use crate::engine::hooks::{ConversationCandidateAssessment, evaluate_hook_effects};
use crate::engine::state::WorldState;
use serde::Deserialize;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

const HIDDEN_EXPLORATION_ACTIONS_HOOK: &str = "npc.hidden_exploration_actions";

#[derive(Debug, Clone)]
pub(crate) struct SpeakCandidateContext<'a> {
    pub(crate) actor: &'a ActorDefinition,
    pub(crate) latest_message: Option<String>,
    pub(crate) evaluation: ConversationCandidateAssessment,
}

#[derive(Debug, Clone)]
pub(crate) struct ActorTurnInspectFeatureCandidate {
    pub(crate) feature_id: String,
    pub(crate) label: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ActorTurnInspectActorCandidate {
    pub(crate) actor_id: String,
    pub(crate) actor_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HiddenAffordanceAction {
    Move,
    InspectFeature,
    InspectActor,
}

#[derive(Debug, Deserialize)]
struct HiddenAffordanceEffect {
    action: HiddenAffordanceAction,
}

pub(crate) fn actors_in_room_except<'a>(
    content: &'a ContentPack,
    state: &WorldState,
    room_id: &str,
    actor_id: &str,
) -> Vec<&'a ActorDefinition> {
    content
        .actors
        .iter()
        .filter(|other| {
            other.id != actor_id && state.actor_room_id(&other.id, &other.room_id) == room_id
        })
        .collect()
}

pub(crate) fn reply_pending_from_candidate(
    state: &WorldState,
    actor_id: &str,
    candidate_id: &str,
) -> bool {
    state
        .pending_reply(candidate_id, actor_id)
        .is_some_and(|pending| {
            pending.speaker_id == candidate_id && pending.listener_id == actor_id
        })
}

pub(crate) fn hidden_exploration_actions(
    content: &ContentPack,
    actor_stats: &BTreeMap<String, i32>,
    rest_available: bool,
    eat_candidate_count: usize,
    drink_candidate_count: usize,
    consume_candidate_count: usize,
) -> Result<BTreeSet<HiddenAffordanceAction>, String> {
    Ok(evaluate_hook_effects::<HiddenAffordanceEffect>(
        content,
        HIDDEN_EXPLORATION_ACTIONS_HOOK,
        json!({
            "actor_stats": actor_stats,
            "affordances": {
                "rest": {
                    "available": rest_available,
                    "option_count": usize::from(rest_available),
                },
                "eat": {
                    "available": eat_candidate_count > 0,
                    "option_count": eat_candidate_count,
                },
                "drink": {
                    "available": drink_candidate_count > 0,
                    "option_count": drink_candidate_count,
                },
                "consume": {
                    "available": consume_candidate_count > 0,
                    "option_count": consume_candidate_count,
                },
            },
        }),
    )?
    .into_iter()
    .map(|effect| effect.action)
    .collect())
}

pub(crate) fn available_consume_candidates(
    content: &ContentPack,
    state: &WorldState,
    actor: &ActorDefinition,
    current_room_id: &str,
) -> Vec<ActorTurnConsumeCandidate> {
    content
        .room_consumables(current_room_id)
        .into_iter()
        .filter(|candidate| {
            state.remaining_consumable_stock(
                current_room_id,
                &candidate.feature.id,
                &candidate.consumable.id,
            ) > 0
        })
        .filter(|candidate| consumable_matches_actor_requirements(actor, candidate.consumable))
        .map(|candidate| ActorTurnConsumeCandidate {
            item_id: candidate.consumable.id.clone(),
            item_label: candidate.consumable.label.clone(),
            feature_label: candidate.feature.label.clone(),
            kind: candidate.consumable.kind,
            hunger_recovery: candidate.consumable.hunger_recovery,
        })
        .collect()
}

pub(crate) fn preferred_hunger_recovery_consume_item_id(
    consume_candidates: &[ActorTurnConsumeCandidate],
) -> Option<String> {
    consume_candidates
        .iter()
        .filter(|candidate| candidate.hunger_recovery > 0)
        .max_by_key(|candidate| {
            (
                candidate.hunger_recovery,
                match candidate.kind {
                    ConsumableKind::Eat => 2,
                    ConsumableKind::Drink => 1,
                    ConsumableKind::Consume => 0,
                },
            )
        })
        .map(|candidate| candidate.item_id.clone())
}

pub(crate) fn recovery_context_label(
    content: &ContentPack,
    current_room_id: &str,
) -> Option<String> {
    let room = content.room(current_room_id)?;
    room.features
        .iter()
        .find(|feature| feature.allow_rest)
        .map(|feature| feature.label.clone())
        .or_else(|| {
            if room.allow_rest {
                Some(room.title.to_ascii_lowercase())
            } else {
                None
            }
        })
}

pub(crate) fn inspect_feature_candidates(
    content: &ContentPack,
    state: &WorldState,
    actor: &ActorDefinition,
    current_room_id: &str,
) -> Vec<ActorTurnInspectFeatureCandidate> {
    content
        .room(current_room_id)
        .map(|room| {
            room.features
                .iter()
                .filter(|feature| {
                    !state.actor_has_seen_feature(&actor.id, current_room_id, &feature.id)
                })
                .map(|feature| ActorTurnInspectFeatureCandidate {
                    feature_id: feature.id.clone(),
                    label: feature.label.clone(),
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn inspect_actor_candidates(
    state: &WorldState,
    actor: &ActorDefinition,
    speak_candidates: &[SpeakCandidateContext<'_>],
) -> Vec<ActorTurnInspectActorCandidate> {
    speak_candidates
        .iter()
        .filter(|candidate| !state.actor_has_studied_actor(&actor.id, &candidate.actor.id))
        .map(|candidate| ActorTurnInspectActorCandidate {
            actor_id: candidate.actor.id.clone(),
            actor_name: candidate.actor.name.clone(),
        })
        .collect()
}

fn consumable_matches_actor_requirements(
    actor: &ActorDefinition,
    consumable: &ConsumableDefinition,
) -> bool {
    actor.required_consumable_tags.is_empty()
        || actor.required_consumable_tags.iter().all(|tag| {
            consumable
                .tags
                .iter()
                .any(|consumable_tag| consumable_tag == tag)
        })
}
