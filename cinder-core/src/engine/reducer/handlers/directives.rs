use super::menus::handle_menu_opened;
use crate::engine::reducer::beat_advance::advance_objective_for_signal;
use crate::engine::reducer::command_effects::handle_actor_command_used;
use crate::engine::reducer::observation::render_story_text;
use crate::content::types::ContentPack;
use crate::engine::events::WorldEvent;
use crate::engine::hooks::apply_world_hook_effects;
use crate::engine::narrative::NarrativeLines;
use crate::engine::state::WorldState;
use serde_json::{Value, json};

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_actor_command_used_event(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    actor_name: &str,
    room_id: &str,
    command_id: &str,
    target_room_id: Option<&str>,
    target_actor_id: Option<&str>,
    target_actor_name: Option<&str>,
    context_label: Option<&str>,
    feature_id: Option<&str>,
    consumable_id: Option<&str>,
    freeform_text: Option<&str>,
    lines: &mut NarrativeLines,
    outbox: &mut Vec<WorldEvent>,
) {
    if let Some(new_lines) = handle_actor_command_used(
        state,
        content,
        actor_id,
        actor_name,
        room_id,
        command_id,
        target_room_id,
        target_actor_id,
        target_actor_name,
        context_label,
        feature_id,
        consumable_id,
        freeform_text,
        outbox,
    ) {
        lines.extend(new_lines.0);
    }
}

pub(crate) fn apply_content_event(
    state: &mut WorldState,
    content: &ContentPack,
    event_id: &str,
    payload: &std::collections::BTreeMap<String, String>,
    lines: &mut NarrativeLines,
) {
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
        input.insert("actor_stats".to_string(), json!(state.actor_stats));
        apply_world_hook_effects(state, content, &event.hook_id, Value::Object(input))
            .unwrap_or_else(|error| eprintln!("[cinder] hook warning (content_event): {error}"));
    }
    for signal in &event.signals {
        let rendered_signal = render_story_text(
            &content.render_template(signal, &template_values),
            state,
        );
        lines.extend_narration(advance_objective_for_signal(
            state,
            content,
            &rendered_signal,
        ));
    }
    if !event.open_menu.is_empty() {
        handle_menu_opened(state, content, &event.open_menu, lines);
    }
}