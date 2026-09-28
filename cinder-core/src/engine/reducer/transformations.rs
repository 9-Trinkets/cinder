use crate::content::types::{ActorTransformation, ContentPack, TransformationTrigger};
use crate::engine::narrative::NarrativeLines;
use crate::engine::reducer::beat_advance::advance_objective_for_signal;
use crate::engine::state::{ActorStance, WorldState};

/// Standard wake/evolution narration keys. The engine only *requests* these;
/// the pack owns their text. A transformation may override them with
/// `narration_keys`.
const WAKE_NARRATION_KEYS: [&str; 3] = [
    "transformation.wake.intro",
    "transformation.wake.fragment",
    "transformation.wake.reveal",
];

/// Applies the actor's content-declared transformation stages whose trigger
/// is met (job change, evolution, awakening, promotion, ...). Stage
/// application is recorded in state and its effects — rename, stance,
/// narration, and beat signals — are driven entirely by pack content.
pub fn maybe_apply_transformations(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    lines: &mut NarrativeLines,
) -> bool {
    if content.is_player_actor(actor_id) {
        return false;
    }
    let Some(actor) = content.actor(actor_id) else {
        return false;
    };
    let mut applied = false;
    for transformation in &actor.transformations {
        if state.is_actor_transformed(actor_id, &transformation.id) {
            continue;
        }
        if !trigger_met(state, content, actor_id, transformation) {
            continue;
        }
        apply_transformation(state, content, actor_id, transformation, lines);
        applied = true;
    }
    applied
}

fn trigger_met(
    state: &WorldState,
    content: &ContentPack,
    actor_id: &str,
    transformation: &ActorTransformation,
) -> bool {
    match &transformation.trigger {
        TransformationTrigger::Always => true,
        TransformationTrigger::Stat { stat, gte } => {
            state.effective_actor_stat(content, actor_id, stat) >= *gte
        }
        TransformationTrigger::StoryVar { key, value } => state.story_vars.get(key) == Some(value),
    }
}

fn apply_transformation(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    transformation: &ActorTransformation,
    lines: &mut NarrativeLines,
) {
    let original_name = content
        .actor(actor_id)
        .map(|actor| actor.name.clone())
        .unwrap_or_default();
    let fragment = transformation
        .rename
        .as_ref()
        .map(|rename| rename.fragment.clone())
        .unwrap_or_default();
    let woken_name = transformation
        .rename
        .as_ref()
        .map(|rename| rename.name.clone())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| original_name.clone());

    state.set_transformation_applied(actor_id, &transformation.id);

    if let Some(rename) = &transformation.rename
        && !rename.name.is_empty()
    {
        state
            .actor_name_overrides
            .insert(actor_id.to_string(), rename.name.clone());
    }

    if let Some(stance_text) = &transformation.stance {
        let stance =
            serde_json::from_value::<ActorStance>(serde_json::Value::String(stance_text.clone()))
                .unwrap_or(ActorStance::Allied);
        state.set_actor_stance(content, actor_id, stance, transformation.follows_player);
    }

    let vars: [(&str, &str); 3] = [
        ("actor_name", original_name.as_str()),
        ("woken_name", woken_name.as_str()),
        ("fragment", fragment.as_str()),
    ];
    let keys: Vec<&str> = if transformation.narration_keys.is_empty() {
        WAKE_NARRATION_KEYS.to_vec()
    } else {
        transformation
            .narration_keys
            .iter()
            .map(|key| key.as_str())
            .collect()
    };
    for key in keys {
        if let Some(line) = content.render_message(key, &vars) {
            lines.narration(line);
        }
    }

    for signal in &transformation.signals {
        lines.extend_narration(advance_objective_for_signal(state, content, signal));
    }
}
