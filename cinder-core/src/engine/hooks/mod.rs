use crate::content::types::ContentPack;
use crate::engine::hook_ids;
use crate::engine::narrative::NarrativeLines;
use crate::engine::neuron::evaluate_symbolic_value;
use crate::engine::reducer::handlers::push_rendered_message;
use crate::engine::state::{ActorStance, WorldState};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer};
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
            } => state.adjust_actor_stat(content, &actor_id, &stat, delta)?,
            WorldHookEffect::ConvertActorToAlly {
                actor_id,
                follows_player,
                messages,
            } => {
                let mut relationship = state.relationship(&actor_id);
                relationship.stance = ActorStance::Allied;
                relationship.follows_player = follows_player;
                state.set_relationship(&actor_id, relationship);
                state.initialize_party_order(content, &actor_id);
                if let Some(lines) = lines.as_deref_mut() {
                    let actor_name = content
                        .actor(&actor_id)
                        .map(|actor| actor.name.as_str())
                        .unwrap_or(&actor_id);
                    for key in &messages {
                        if let Some(line) = content.render_message(key, &[("actor", actor_name)]) {
                            lines.narration(line);
                        }
                    }
                }
            }
            WorldHookEffect::SetStanceByTag {
                tag,
                stance,
                from_stances,
                follows_player,
                messages,
            } => {
                let health_stat_id = &content.settings.combat.health_stat_id;
                let tagged_actors: Vec<(String, String)> = state
                    .actors(content)
                    .filter(|actor| actor.tags.iter().any(|actor_tag| actor_tag.as_str() == tag))
                    .map(|actor| (actor.id.clone(), actor.name.clone()))
                    .collect();
                for (actor_id, actor_name) in tagged_actors {
                    if content.is_player_actor(&actor_id) {
                        continue;
                    }
                    if state.actor_current_room_id(content, &actor_id).is_empty() {
                        continue;
                    }
                    if state.actor_is_defeated(&actor_id, health_stat_id) {
                        continue;
                    }
                    let mut relationship = state.relationship(&actor_id);
                    if !from_stances.is_empty() && !from_stances.contains(&relationship.stance) {
                        continue;
                    }
                    if relationship.stance == stance {
                        continue;
                    }
                    relationship.stance = stance;
                    relationship.follows_player = follows_player;
                    state.set_relationship(&actor_id, relationship);
                    if stance == ActorStance::Allied {
                        state.initialize_party_order(content, &actor_id);
                    }
                    if let Some(lines) = lines.as_deref_mut() {
                        for key in &messages {
                            if let Some(line) =
                                content.render_message(key, &[("actor", actor_name.as_str())])
                            {
                                lines.narration(line);
                            }
                        }
                    }
                }
            }
            WorldHookEffect::SetStoryVar { key, value } => {
                state.story_vars.set_unchecked(key.as_str(), value.as_str());
            }
            WorldHookEffect::NarrateMessage { key, vars } => {
                if let Some(lines) = lines.as_deref_mut() {
                    let replacements: Vec<(&str, &str)> =
                        vars.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
                    if let Some(line) = content.render_message(&key, &replacements) {
                        push_rendered_message(lines, content, line, content.message_voice(&key));
                    }
                }
            }
            WorldHookEffect::DefeatActorsByTag { tag } => {
                let health_stat_id = &content.settings.combat.health_stat_id;
                let tagged_actor_ids: Vec<String> = state
                    .actors(content)
                    .filter(|actor| {
                        !content.is_player_actor(&actor.id)
                            && actor.tags.iter().any(|actor_tag| actor_tag.as_str() == tag)
                    })
                    .map(|actor| actor.id.clone())
                    .collect();
                for actor_id in tagged_actor_ids {
                    // A large negative delta clamps to the stat's min (0).
                    state
                        .adjust_actor_stat(content, &actor_id, health_stat_id, i32::MIN / 2)
                        .unwrap_or_else(|error| eprintln!("[cinder] defeat stat error: {error}"));
                }
            }
            WorldHookEffect::SpawnActor {
                template_id,
                room_id,
                stance,
                follows_player,
                messages,
                scale_with_actor_id,
                scale_stat,
                max_active_instances,
                max_instances_messages,
            } => {
                let Some(template) = state.actor(content, &template_id).cloned() else {
                    eprintln!("[cinder] spawn actor: template '{template_id}' not found");
                    continue;
                };
                if let Some(max) = max_active_instances {
                    let active_count = state.active_spawned_actor_count(content, &template_id);
                    if active_count >= max {
                        if let Some(lines) = lines.as_deref_mut() {
                            let max_str = max.to_string();
                            for key in &max_instances_messages {
                                if let Some(line) = content.render_message(
                                    key,
                                    &[
                                        ("actor", template.name.as_str()),
                                        ("template_id", template_id.as_str()),
                                        ("max", max_str.as_str()),
                                    ],
                                ) {
                                    push_rendered_message(lines, content, line, content.message_voice(key));
                                }
                            }
                        }
                        continue;
                    }
                }
                let target_room_id = room_id
                    .filter(|r| !r.is_empty())
                    .unwrap_or_else(|| state.current_room_id.clone());
                state.spawn_counter += 1;
                let instance_id = format!("{template_id}-{}", state.spawn_counter);
                let mut instance = template;
                instance.id = instance_id.clone();
                instance.room_id = target_room_id.clone();

                let health_stat_id = if content.settings.combat.health_stat_id.is_empty() {
                    "hp"
                } else {
                    &content.settings.combat.health_stat_id
                };

                let (scaled_hp, scaled_str, scaled_intel, scaler_val) =
                    if let Some(scaler_id) = scale_with_actor_id {
                        let stat_name = scale_stat.as_deref().unwrap_or("intelligence");
                        let scaler = state.effective_actor_stat(content, &scaler_id, stat_name);
                        let base_hp = instance
                            .initial_stats
                            .get(health_stat_id)
                            .copied()
                            .unwrap_or(4);
                        let base_str = instance.initial_stats.get("strength").copied().unwrap_or(2);
                        let base_intel = instance
                            .initial_stats
                            .get("intelligence")
                            .copied()
                            .unwrap_or(2);
                        let hp = (base_hp + scaler).max(1);
                        let str_val = (base_str + scaler / 3).max(1);
                        let intel_val = (base_intel + scaler / 2).max(1);
                        instance.initial_stats.insert(health_stat_id.to_string(), hp);
                        instance
                            .initial_stats
                            .insert("strength".to_string(), str_val);
                        instance
                            .initial_stats
                            .insert("intelligence".to_string(), intel_val);
                        (hp, str_val, intel_val, scaler)
                    } else {
                        let hp = instance
                            .initial_stats
                            .get(health_stat_id)
                            .copied()
                            .unwrap_or(4);
                        let str_val = instance.initial_stats.get("strength").copied().unwrap_or(2);
                        let intel_val = instance
                            .initial_stats
                            .get("intelligence")
                            .copied()
                            .unwrap_or(2);
                        (hp, str_val, intel_val, 0)
                    };

                let actor_name = instance.name.clone();
                state
                    .actor_stats
                    .insert(instance_id.clone(), instance.initial_stats.clone());
                state
                    .initial_actor_stats
                    .insert(instance_id.clone(), instance.initial_stats.clone());
                state
                    .actor_room_overrides
                    .insert(instance_id.clone(), target_room_id.clone());
                state.mark_actor_room_visited(&instance_id, &target_room_id);
                state.spawned_actors.insert(instance_id.clone(), instance);

                let stance = stance.unwrap_or(ActorStance::Allied);
                let relationship =
                    crate::engine::state::ActorRelationship { stance, follows_player };
                state.set_relationship(&instance_id, relationship);
                if stance == ActorStance::Allied {
                    state.initialize_party_order(content, &instance_id);
                }

                if let Some(lines) = lines.as_deref_mut() {
                    let hp_str = scaled_hp.to_string();
                    let str_str = scaled_str.to_string();
                    let intel_str = scaled_intel.to_string();
                    let scaler_val_str = scaler_val.to_string();
                    for key in &messages {
                        if let Some(line) = content.render_message(
                            key,
                            &[
                                ("actor", actor_name.as_str()),
                                ("instance_id", instance_id.as_str()),
                                ("room_id", target_room_id.as_str()),
                                ("hp", hp_str.as_str()),
                                ("strength", str_str.as_str()),
                                ("intelligence", intel_str.as_str()),
                                ("intel", scaler_val_str.as_str()),
                            ],
                        ) {
                            push_rendered_message(lines, content, line, content.message_voice(key));
                        }
                    }
                }
            }
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
    /// Turns an actor into an ally (and optionally a follower). `messages` are
    /// pack-authored message keys rendered with `{actor}` for narration.
    ConvertActorToAlly {
        actor_id: String,
        #[serde(default)]
        follows_player: bool,
        #[serde(default)]
        messages: Vec<String>,
    },
    /// Sets the stance of matching living actors carrying `tag` (e.g. charming
    /// a golem army into allies, or hostile elves standing down once their king
    /// falls). `from_stances` can preserve actors already changed by another
    /// effect. Also optionally converts matching actors into followers.
    /// Defeated actors and actors already in the target stance are left
    /// untouched, so re-firing (e.g. re-equipping an item) is idempotent.
    SetStanceByTag {
        tag: String,
        stance: ActorStance,
        #[serde(default)]
        from_stances: Vec<ActorStance>,
        #[serde(default)]
        follows_player: bool,
        #[serde(default)]
        messages: Vec<String>,
    },
    /// Sets a story variable (e.g. a flag marking a boss as defeated).
    SetStoryVar { key: String, value: String },
    /// Defeats every living actor carrying `tag` (e.g. an army crumbling when
    /// its commander falls).
    DefeatActorsByTag { tag: String },
    /// Plot beat rendered through the pack's message table (with any `vars`),
    /// honoring the key's voice (e.g. handler-voiced comms when a pack fronts
    /// feedback through a handler channel). Called with the narrating entry
    /// point so the text reaches the player.
    NarrateMessage {
        key: String,
        #[serde(default)]
        vars: Vec<(String, String)>,
    },
    /// Instantiates a new runtime actor from an authored template.
    /// Supports intelligence-based stat scaling, party recruitment, and room placement.
    SpawnActor {
        template_id: String,
        #[serde(default)]
        room_id: Option<String>,
        #[serde(default)]
        stance: Option<ActorStance>,
        #[serde(default)]
        follows_player: bool,
        #[serde(default)]
        messages: Vec<String>,
        #[serde(default)]
        scale_with_actor_id: Option<String>,
        #[serde(default)]
        scale_stat: Option<String>,
        #[serde(default)]
        max_active_instances: Option<usize>,
        #[serde(default)]
        max_instances_messages: Vec<String>,
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
struct RoomCandidateScorePayload {
    #[serde(default)]
    score: i32,
}
