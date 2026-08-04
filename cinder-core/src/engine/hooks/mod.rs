use crate::content::types::ContentPack;
use crate::engine::hook_ids;
use crate::engine::neuron::evaluate_symbolic_value;
use crate::engine::state::WorldState;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
const ROOM_CANDIDATE_SCORE_HOOK: &str = "npc.room_candidate_score";

pub(crate) fn evaluate_hook_payload<T>(
    content: &ContentPack,
    hook_id: &str,
    input: Value,
) -> Result<Option<T>, String>
where
    T: DeserializeOwned,
{
    let Some(hook) = content.hook(hook_id) else {
        return Ok(None);
    };
    let payload = evaluate_symbolic_value(hook, &input)?;
    serde_json::from_value(payload)
        .map(Some)
        .map_err(|error| error.to_string())
}

pub(crate) fn evaluate_hook_effects<T>(
    content: &ContentPack,
    hook_id: &str,
    input: Value,
) -> Result<Vec<T>, String>
where
    T: DeserializeOwned,
{
    let Some(hook) = content.hook(hook_id) else {
        return Ok(Vec::new());
    };
    let payload = evaluate_symbolic_value(hook, &input)?;
    let effects = payload
        .get("effects")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    effects
        .into_iter()
        .map(|effect| serde_json::from_value(effect).map_err(|error| error.to_string()))
        .collect()
}

pub(crate) fn actor_state_notes(
    content: &ContentPack,
    state: &WorldState,
    actor_id: &str,
) -> Vec<String> {
    collect_hook_notes(
        content,
        hook_ids::STATE_NOTES,
        json!({
            "actor_id": actor_id,
            "actor_stats": actor_stats_input(state, actor_id),
        }),
    )
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ActorTurnGuidance {
    pub visible_affordance_ids: BTreeSet<String>,
    pub hidden_affordance_ids: BTreeSet<String>,
    pub affordance_priorities: BTreeMap<String, i32>,
    pub group_priorities: BTreeMap<String, i32>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ActorTurnGuidanceSpeakCandidateInput {
    pub reply_now: bool,
    pub pair_stats: BTreeMap<String, i32>,
    pub affordances: BTreeMap<String, CandidateAffordanceInput>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ActorTurnGuidanceAffordanceInput {
    pub available: bool,
    pub option_count: usize,
    pub group: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct CandidateAffordanceInput {
    #[serde(default)]
    pub available: bool,
}

pub(crate) struct ActorTurnGuidanceInput {
    pub actor_stats: BTreeMap<String, i32>,
    pub affordances: BTreeMap<String, ActorTurnGuidanceAffordanceInput>,
    pub speak_candidate_count: usize,
    pub unmet_speak_candidate_count: usize,
    pub top_speak_candidate: Option<ActorTurnGuidanceSpeakCandidateInput>,
    pub active_stage_ids: Vec<String>,
    pub unvisited_room_count: usize,
    pub unseen_feature_count: usize,
}

pub(crate) fn actor_turn_guidance(
    content: &ContentPack,
    input: ActorTurnGuidanceInput,
) -> ActorTurnGuidance {
    let effects = evaluate_hook_effects::<ActorTurnGuidanceEffect>(
        content,
        hook_ids::TURN_ACTION_GUIDANCE,
        json!({
            "actor_stats": input.actor_stats,
            "affordances": input.affordances.into_iter().map(|(id, affordance)| {
                (id, json!({
                    "available": affordance.available,
                    "option_count": affordance.option_count,
                    "group": affordance.group,
                }))
            }).collect::<serde_json::Map<String, Value>>(),
            "speak_candidate_count": input.speak_candidate_count,
            "unmet_speak_candidate_count": input.unmet_speak_candidate_count,
            "top_speak_candidate": input.top_speak_candidate.map(|candidate| json!({
                "reply_now": candidate.reply_now,
                "pair_stats": candidate.pair_stats,
                "affordances": affordance_inputs_json(&candidate.affordances),
            })).unwrap_or(Value::Null),
            "active_stage_ids": input.active_stage_ids,
            "unvisited_room_count": input.unvisited_room_count,
            "unseen_feature_count": input.unseen_feature_count,
        }),
    )
    .unwrap_or_default();
    let mut guidance = ActorTurnGuidance::default();
    for effect in effects {
        match effect {
            ActorTurnGuidanceEffect::ShowAffordance { affordance_id } => {
                guidance.visible_affordance_ids.insert(affordance_id);
            }
            ActorTurnGuidanceEffect::HideAffordance { affordance_id } => {
                guidance.hidden_affordance_ids.insert(affordance_id);
            }
            ActorTurnGuidanceEffect::PrioritizeAffordance { affordance_id, by } => {
                *guidance
                    .affordance_priorities
                    .entry(affordance_id)
                    .or_default() += by;
            }
            ActorTurnGuidanceEffect::PrioritizeGroup { group, by } => {
                *guidance.group_priorities.entry(group).or_default() += by;
            }
        }
    }
    guidance
}

pub(crate) fn pair_state_note(
    content: &ContentPack,
    state: &WorldState,
    participant_a_id: &str,
    participant_b_id: &str,
    other_person_name: &str,
    affordances: &BTreeMap<String, CandidateAffordanceInput>,
) -> Option<String> {
    join_hook_notes(collect_hook_notes(
        content,
        hook_ids::PAIR_STATE_NOTES,
        json!({
            "participant_a_id": participant_a_id,
            "participant_b_id": participant_b_id,
            "other_person_name": other_person_name,
            "actor_stats": actor_stats_input(state, participant_a_id),
            "pair_stats": pair_stats_input(state, participant_a_id, participant_b_id),
            "affordances": affordance_inputs_json(affordances),
        }),
    ))
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ConversationCandidateAssessment {
    pub visible: bool,
    pub affordances: BTreeMap<String, CandidateAffordanceInput>,
}

pub(crate) fn conversation_candidate_assessment(
    content: &ContentPack,
    state: &WorldState,
    actor_id: &str,
    other_actor_id: &str,
) -> ConversationCandidateAssessment {
    evaluate_hook_payload::<ConversationCandidatePayload>(
        content,
        hook_ids::CONVERSATION_CANDIDATE,
        json!({
            "actor_id": actor_id,
            "other_actor_id": other_actor_id,
            "actor_stats": actor_stats_input(state, actor_id),
            "pair_stats": pair_stats_input(state, actor_id, other_actor_id),
        }),
    )
    .ok()
    .flatten()
    .map(|payload| ConversationCandidateAssessment {
        visible: payload.visible,
        affordances: payload.affordances,
    })
    .unwrap_or(ConversationCandidateAssessment {
        visible: true,
        affordances: BTreeMap::new(),
    })
}

pub(crate) fn room_candidate_score(
    content: &ContentPack,
    state: &WorldState,
    actor_id: &str,
    other_actor_id: &str,
    current_room_id: &str,
    candidate_room_id: &str,
) -> i32 {
    evaluate_hook_payload::<RoomCandidateScorePayload>(
        content,
        ROOM_CANDIDATE_SCORE_HOOK,
        json!({
            "actor_id": actor_id,
            "other_actor_id": other_actor_id,
            "current_room_id": current_room_id,
            "candidate_room_id": candidate_room_id,
            "actor_stats": actor_stats_input(state, actor_id),
            "pair_stats": pair_stats_input(state, actor_id, other_actor_id),
        }),
    )
    .ok()
    .flatten()
    .map(|payload| payload.score)
    .unwrap_or(0)
}

pub(crate) fn apply_world_hook_effects(
    state: &mut WorldState,
    content: &ContentPack,
    hook_id: &str,
    input: Value,
) -> Result<(), String> {
    let effects = evaluate_hook_effects::<WorldHookEffect>(content, hook_id, input)?;
    for effect in effects {
        match effect {
            WorldHookEffect::AdjustPairStat {
                participant_a_id,
                participant_b_id,
                stat,
                delta,
            } => state.adjust_pair_stat(&participant_a_id, &participant_b_id, &stat, delta)?,
            WorldHookEffect::AdjustActorStat {
                actor_id,
                stat,
                delta,
            } => state.adjust_actor_stat(&actor_id, &stat, delta)?,
        }
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum WorldHookEffect {
    AdjustPairStat {
        participant_a_id: String,
        participant_b_id: String,
        stat: String,
        #[serde(deserialize_with = "deserialize_i32ish")]
        delta: i32,
    },
    AdjustActorStat {
        actor_id: String,
        stat: String,
        #[serde(deserialize_with = "deserialize_i32ish")]
        delta: i32,
    },
}

fn deserialize_i32ish<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::Number(number) => number
            .as_i64()
            .ok_or_else(|| serde::de::Error::custom("delta must be an integer"))
            .and_then(|value| {
                i32::try_from(value).map_err(|_| serde::de::Error::custom("delta out of range"))
            }),
        Value::String(text) => text
            .parse::<i32>()
            .map_err(|_| serde::de::Error::custom("delta string must parse as i32")),
        other => Err(serde::de::Error::custom(format!(
            "delta must be an integer or integer string, got {other}"
        ))),
    }
}

fn actor_stats_input(state: &WorldState, actor_id: &str) -> Value {
    json!(state.actor_stats_snapshot(actor_id))
}

fn pair_stats_input(state: &WorldState, participant_a_id: &str, participant_b_id: &str) -> Value {
    json!(state.pair_stats_snapshot(participant_a_id, participant_b_id))
}

fn collect_hook_notes(content: &ContentPack, hook_id: &str, input: Value) -> Vec<String> {
    let payload = evaluate_hook_payload::<HookNotesPayload>(content, hook_id, input)
        .ok()
        .flatten();
    payload
        .map(|payload| {
            payload
                .notes
                .into_iter()
                .map(|note| note.trim().to_string())
                .filter(|note| !note.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn join_hook_notes(notes: Vec<String>) -> Option<String> {
    match notes.len() {
        0 => None,
        1 => notes.into_iter().next(),
        _ => Some(notes.join(" ")),
    }
}

#[derive(Debug, Deserialize)]
struct HookNotesPayload {
    #[serde(default)]
    notes: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ConversationCandidatePayload {
    #[serde(default = "default_true")]
    visible: bool,
    #[serde(default)]
    affordances: BTreeMap<String, CandidateAffordanceInput>,
}

fn affordance_inputs_json(
    affordances: &BTreeMap<String, CandidateAffordanceInput>,
) -> serde_json::Map<String, Value> {
    affordances
        .iter()
        .map(|(id, affordance)| {
            (
                id.clone(),
                json!({
                    "available": affordance.available,
                }),
            )
        })
        .collect()
}

#[derive(Debug, Deserialize)]
struct RoomCandidateScorePayload {
    #[serde(default)]
    score: i32,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ActorTurnGuidanceEffect {
    ShowAffordance { affordance_id: String },
    HideAffordance { affordance_id: String },
    PrioritizeAffordance { affordance_id: String, by: i32 },
    PrioritizeGroup { group: String, by: i32 },
}

fn default_true() -> bool {
    true
}
