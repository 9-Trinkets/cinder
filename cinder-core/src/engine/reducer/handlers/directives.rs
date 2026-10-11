use super::menus::handle_menu_opened;
use crate::engine::hooks::apply_narrating_world_hook_effects;
use crate::engine::reducer::actor_commands::{ActorCommandContext, handle_actor_command_used};
use crate::engine::reducer::beat_advance::advance_objective_for_signal;
use crate::engine::reducer::context::ReduceCtx;
use crate::engine::reducer::observation::render_story_text;
use serde_json::{Value, json};

pub(crate) fn handle_actor_command_used_event(
    ctx: &mut ReduceCtx<'_>,
    command_id: &str,
    command_context: &ActorCommandContext<'_>,
) {
    let state = &mut *ctx.state;
    let content = ctx.content;
    let lines = &mut *ctx.lines;
    let outbox = &mut *ctx.outbox;
    if let Some(new_lines) =
        handle_actor_command_used(state, content, command_id, command_context, outbox)
    {
        lines.extend(new_lines.0);
    }
}

pub(crate) fn apply_content_event(
    ctx: &mut ReduceCtx<'_>,
    event_id: &str,
    payload: &std::collections::BTreeMap<String, String>,
) {
    let state = &mut *ctx.state;
    let content = ctx.content;
    let lines = &mut *ctx.lines;
    let event = content
        .content_event(event_id)
        .unwrap_or_else(|| panic!("missing content event definition '{event_id}'"));
    let template_values: Vec<_> = payload
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    if !event.event_text.is_empty() {
        lines.narration(render_story_text(
            &content.render_template(&event.event_text, &template_values),
            state,
        ));
    }
    if !event.hook_id.is_empty() {
        let mut input = serde_json::Map::new();
        input.insert("event_id".to_string(), json!(event.id));
        for (key, value) in payload {
            input.insert(key.clone(), json!(value));
        }
        input.insert("actor_stats".to_string(), json!(state.combat.actor_stats));
        apply_narrating_world_hook_effects(
            state,
            content,
            &event.hook_id,
            Value::Object(input),
            lines,
        )
        .unwrap_or_else(|error| eprintln!("[cinder] hook warning (content_event): {error}"));
    }
    for signal in &event.signals {
        let rendered_signal =
            render_story_text(&content.render_template(signal, &template_values), state);
        lines.extend_lines(advance_objective_for_signal(
            state,
            content,
            &rendered_signal,
        ));
    }
    if !event.open_menu.is_empty() {
        handle_menu_opened(state, content, &event.open_menu, lines);
    }
}
