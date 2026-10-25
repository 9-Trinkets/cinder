use crate::content::types::AdvanceEffect;
use crate::content::types::ContentPack;
use crate::engine::events::ObservationMode;
use crate::engine::interactions::{find_interaction, interaction_used_var};
use crate::engine::narrative::NarrativeLines;
use crate::engine::reducer::beat_advance::advance_objective_for_signal;
use crate::engine::reducer::beat_advance::apply_stage_effects;
use crate::engine::reducer::context::ReduceCtx;
use crate::engine::reducer::observation::{
    render_feature_items_line, render_room_observation, render_story_text,
};
use crate::engine::state::WorldState;

pub(crate) fn handle_current_room_observed(
    state: &mut WorldState,
    content: &ContentPack,
    room_id: &str,
    mode: ObservationMode,
    lines: &mut NarrativeLines,
    insert_at: Option<usize>,
) {
    if let Some(observation) = render_room_observation(content, state, room_id, mode.clone()) {
        if mode == ObservationMode::Summary
            && let Some(idx) = insert_at
            && idx <= lines.0.len()
        {
            lines.0.splice(idx..idx, observation.0);
        } else {
            lines.extend(observation.0);
        }
    } else {
        lines.error(content.presentation.error_text.room_missing.clone());
    }
}

pub(crate) fn handle_feature_observed(ctx: &mut ReduceCtx<'_>, room_id: &str, feature_id: &str) {
    let state = &mut *ctx.state;
    let content = ctx.content;
    let lines = &mut *ctx.lines;
    if let Some(feature) = content.feature(room_id, feature_id) {
        lines.narration(content.feature_inspect_text(state, room_id, feature));
        if let Some(consumables_line) =
            render_feature_items_line(content, state, room_id, feature_id)
        {
            lines.narration(consumables_line.trim_start().to_string());
        }
    } else {
        lines.narration(content.presentation.error_text.room_missing.clone());
    }
    lines.extend_lines(advance_objective_for_signal(
        state,
        content,
        &format!("feature_observed:{room_id}:{feature_id}"),
    ));
    lines.extend_lines(advance_objective_for_signal(
        state,
        content,
        &format!("feature_observed:{feature_id}"),
    ));
}

pub(crate) fn handle_actor_observed(ctx: &mut ReduceCtx<'_>, actor_id: &str) {
    let state = &mut *ctx.state;
    let content = ctx.content;
    let lines = &mut *ctx.lines;
    if let Some(actor) = content.actor(actor_id) {
        let inspect = if let Some(rename) = content
            .applied_transformation(state, actor_id)
            .and_then(|transformation| transformation.rename.as_ref())
            && let Some(transformed_inspect) = &rename.inspect_text
        {
            transformed_inspect.as_str()
        } else {
            actor.inspect_text.as_str()
        };
        lines.narration(render_story_text(inspect, state));
    } else {
        lines.narration(content.presentation.error_text.actor_unknown.clone());
    }
    lines.extend_lines(advance_objective_for_signal(
        state,
        content,
        &format!("actor_observed:{actor_id}"),
    ));
}

pub(crate) fn handle_actor_observed_room(
    ctx: &mut ReduceCtx<'_>,
    actor_id: &str,
    actor_name: &str,
    room_id: &str,
) {
    let state = &mut *ctx.state;
    let content = ctx.content;
    let lines = &mut *ctx.lines;
    if let Some(room) = content.room(room_id) {
        state.mark_actor_observed_room(actor_id, room_id);
        state.push_actor_observation_note(actor_id, room.inspect_text.clone());
        if state.current_room_id == room_id
            && let Some(line) = content.render_message(
                "observation.actor_inspects_room",
                &[
                    ("actor_name", actor_name),
                    ("room_title", room.title.as_str()),
                ],
            )
        {
            lines.narration(line);
        }
    } else {
        lines.narration(content.presentation.error_text.room_missing.clone());
    }
}

pub(crate) fn handle_actor_observed_feature(
    ctx: &mut ReduceCtx<'_>,
    actor_id: &str,
    actor_name: &str,
    room_id: &str,
    feature_id: &str,
) {
    let state = &mut *ctx.state;
    let content = ctx.content;
    let lines = &mut *ctx.lines;
    if let Some(feature) = content.feature(room_id, feature_id) {
        state.mark_actor_feature_seen(actor_id, room_id, feature_id);
        state.push_actor_observation_note(actor_id, feature.inspect_text.clone());
        if state.current_room_id == room_id
            && let Some(line) = content.render_message(
                "observation.actor_inspects_feature",
                &[
                    ("actor_name", actor_name),
                    ("feature_label", feature.label.as_str()),
                ],
            )
        {
            lines.narration(line);
        }
    } else {
        lines.narration(content.presentation.error_text.room_missing.clone());
    }
}

pub(crate) struct ActorObservationContext<'a> {
    pub(crate) actor_id: &'a str,
    pub(crate) actor_name: &'a str,
    pub(crate) target_actor_id: &'a str,
    pub(crate) target_actor_name: &'a str,
}

pub(crate) fn handle_actor_observed_actor(
    ctx: &mut ReduceCtx<'_>,
    room_id: &str,
    aoc: ActorObservationContext<'_>,
) {
    let state = &mut *ctx.state;
    let content = ctx.content;
    let lines = &mut *ctx.lines;
    let actor_id = aoc.actor_id;
    let actor_name = aoc.actor_name;
    let target_actor_id = aoc.target_actor_id;
    let target_actor_name = aoc.target_actor_name;
    if let Some(target_actor) = content.actor(target_actor_id) {
        state.mark_actor_studied_actor(actor_id, target_actor_id);
        state.push_actor_observation_note(actor_id, target_actor.inspect_text.clone());
        if state.current_room_id == room_id
            && let Some(line) = content.render_message(
                "observation.actor_studies_actor",
                &[
                    ("actor_name", actor_name),
                    ("target_actor_name", target_actor_name),
                ],
            )
        {
            lines.narration(line);
        }
    } else {
        lines.narration(content.presentation.error_text.actor_unknown.clone());
    }
}

pub(crate) fn handle_interaction_used(ctx: &mut ReduceCtx<'_>, owner: &str, interaction_id: &str) {
    let state = &mut *ctx.state;
    let content = ctx.content;
    let lines = &mut *ctx.lines;
    let Some(found) = find_interaction(content, owner, interaction_id) else {
        return;
    };
    let interaction = found.interaction;
    for line in &interaction.lines {
        lines.narration(render_story_text(line, state));
    }
    state
        .story_vars
        .set_unchecked(&interaction_used_var(owner, interaction_id), "true");
    // An effect that names no feature acts on the one that owns the interaction.
    let owner_feature = owner.strip_prefix("feature:");
    let effects: Vec<_> = interaction
        .effects
        .iter()
        .cloned()
        .map(|effect| match (effect, owner_feature) {
            (AdvanceEffect::SetFeatureState { feature, state }, Some(owner_feature))
                if feature.is_empty() =>
            {
                AdvanceEffect::SetFeatureState {
                    feature: owner_feature.to_string(),
                    state,
                }
            }
            (effect, _) => effect,
        })
        .collect();
    let mut effect_lines = Vec::new();
    apply_stage_effects(state, content, &effects, &mut effect_lines);
    lines.extend_lines(effect_lines);
    lines.extend_lines(advance_objective_for_signal(
        state,
        content,
        &format!("interaction_done:{}:{interaction_id}", found.owner_id()),
    ));
}
