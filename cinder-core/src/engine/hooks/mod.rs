mod effects;
use effects::WorldHookEffect;

use crate::content::types::ContentPack;
use crate::engine::hook_ids;
use crate::engine::narrative::NarrativeLines;
use crate::engine::neuron::evaluate_symbolic_value;
use crate::engine::state::WorldState;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{Value, json};
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

pub(crate) fn pair_state_note(
    content: &ContentPack,
    state: &WorldState,
    participant_a_id: &str,
    participant_b_id: &str,
    other_person_name: &str,
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
        }),
    ))
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
    apply_hook_effects(state, content, hook_id, input, None)
}

/// Applies a hook's effects and also renders any narration carried by
/// relationship-changing effects (e.g. `ConvertActorToAlly`) into `lines`.
pub(crate) fn apply_narrating_world_hook_effects(
    state: &mut WorldState,
    content: &ContentPack,
    hook_id: &str,
    input: Value,
    lines: &mut NarrativeLines,
) -> Result<(), String> {
    apply_hook_effects(state, content, hook_id, input, Some(lines))
}

fn apply_hook_effects(
    state: &mut WorldState,
    content: &ContentPack,
    hook_id: &str,
    input: Value,
    mut lines: Option<&mut NarrativeLines>,
) -> Result<(), String> {
    let effects = evaluate_hook_effects::<WorldHookEffect>(content, hook_id, input.clone())?;
    for effect in effects {
        effect.apply(state, content, &input, lines.as_deref_mut())?;
    }
    Ok(())
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
struct RoomCandidateScorePayload {
    #[serde(default)]
    score: i32,
}
