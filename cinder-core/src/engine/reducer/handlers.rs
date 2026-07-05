use super::beat_advance::{advance_objective_for_signal, time_reached_signals};
use super::command_effects::{
    ActorMoveTransitionContext, apply_actor_move_transition, apply_speech_stamina_cost,
    handle_actor_command_used,
};
use super::observation::{
    render_actor_speech_line, render_feature_consumables_line, render_room_observation,
    render_story_text,
};
use super::tick::{
    advance_actor_stats_on_tick, advance_house_progress_objectives, advance_stat_threshold_objectives,
    increment_shared_room_safety,
};
use crate::content::types::ContentPack;
use crate::engine::commands::{player_command_help_text, player_command_suggestions};
use crate::engine::events::ObservationMode;
use crate::engine::hook_ids;
use crate::engine::hooks::apply_world_hook_effects;
use crate::engine::state::{ConversationMemoryKind, ConversationMemoryLine, WorldState};
use serde_json::{Value, json};

pub(super) fn handle_turn_started(
    state: &mut WorldState,
    content: &ContentPack,
    turn_number: u32,
    advances_time: bool,
    lines: &mut Vec<String>,
) {
    state.turn_number = turn_number;
    if advances_time {
        increment_shared_room_safety(state, content);
        state.clear_stale_pending_replies();
        let previous_time_minutes = state.current_time_minutes;
        state.current_time_minutes += content.settings.tick_minutes_per_turn;
        advance_actor_stats_on_tick(
            state,
            content,
            previous_time_minutes,
            state.current_time_minutes,
        );
        for signal in time_reached_signals(previous_time_minutes, state.current_time_minutes) {
            lines.extend(advance_objective_for_signal(state, content, &signal));
        }
        lines.extend(advance_house_progress_objectives(state, content));
        lines.extend(advance_stat_threshold_objectives(state, content));
    }
}

pub(super) fn handle_current_room_observed(
    state: &mut WorldState,
    content: &ContentPack,
    room_id: &str,
    mode: ObservationMode,
    lines: &mut Vec<String>,
) {
    if let Some(observation) = render_room_observation(content, state, room_id, mode) {
        lines.push(observation);
    } else {
        lines.push(content.presentation.error_text.room_missing.clone());
    }
}

pub(super) fn handle_feature_observed(
    state: &mut WorldState,
    content: &ContentPack,
    room_id: &str,
    feature_id: &str,
    lines: &mut Vec<String>,
) {
    if let Some(feature) = content.room(room_id).and_then(|room| {
        room.features
            .iter()
            .find(|feature| feature.id == feature_id)
    }) {
        lines.push(feature.inspect_text.clone());
        if let Some(consumables_line) =
            render_feature_consumables_line(content, state, room_id, feature_id)
        {
            lines.push(consumables_line);
        }
    } else {
        lines.push(content.presentation.error_text.room_missing.clone());
    }
}

pub(super) fn handle_actor_observed(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    lines: &mut Vec<String>,
) {
    if let Some(actor) = content.actor(actor_id) {
        lines.push(render_story_text(&actor.inspect_text, state));
    } else {
        lines.push(content.presentation.error_text.actor_unknown.clone());
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_actor_spoke(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    actor_name: &str,
    other_person_id: &str,
    other_person_name: &str,
    other_person_message: &Option<String>,
    room_id: &str,
    text: &str,
    lines: &mut Vec<String>,
) {
    let history = state.conversation_history(actor_id, other_person_id);
    let needs_other_person_line = other_person_message.as_ref().is_some_and(|message| {
        history
            .last()
            .is_none_or(|line| line.speaker_id != other_person_id || line.text != *message)
    });
    if needs_other_person_line && let Some(message) = other_person_message {
        state.push_conversation_line(
            actor_id,
            other_person_id,
            ConversationMemoryLine {
                turn_number: state.turn_number,
                event_sequence: 0,
                speaker_id: other_person_id.to_string(),
                speaker_name: other_person_name.to_string(),
                kind: ConversationMemoryKind::Speech,
                target_label: Some(actor_name.to_string()),
                text: message.clone(),
            },
        );
    }
    state.push_conversation_line(
        actor_id,
        other_person_id,
        ConversationMemoryLine {
            turn_number: state.turn_number,
            event_sequence: 0,
            speaker_id: actor_id.to_string(),
            speaker_name: actor_name.to_string(),
            kind: ConversationMemoryKind::Speech,
            target_label: Some(other_person_name.to_string()),
            text: text.to_string(),
        },
    );
    apply_world_hook_effects(
        state,
        content,
        hook_ids::SPEECH,
        json!({
            "event_kind": "speech",
            "actor_id": actor_id,
            "participant_a_id": actor_id,
            "participant_b_id": other_person_id,
        }),
    )
    .unwrap_or_else(|error| eprintln!("[cinder] hook warning (speech): {error}"));
    apply_speech_stamina_cost(state, content, actor_id, other_person_id);
    if state
        .pending_reply(actor_id, other_person_id)
        .is_some_and(|pending| {
            pending.speaker_id == other_person_id && pending.listener_id == actor_id
        })
    {
        state.clear_pending_reply(actor_id, other_person_id);
    }
    state.set_pending_reply(actor_id, other_person_id, room_id, state.turn_number);
    if state.current_room_id == room_id {
        lines.push(render_actor_speech_line(
            content,
            actor_name,
            Some(other_person_name),
            text,
        ));
    }
    lines.extend(advance_house_progress_objectives(state, content));
}

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_actor_spoke_to_room(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    actor_name: &str,
    audience_actor_ids: &[String],
    room_id: &str,
    text: &str,
    lines: &mut Vec<String>,
) {
    for audience_actor_id in audience_actor_ids.iter() {
        state.push_conversation_line(
            actor_id,
            audience_actor_id,
            ConversationMemoryLine {
                turn_number: state.turn_number,
                event_sequence: 0,
                speaker_id: actor_id.to_string(),
                speaker_name: actor_name.to_string(),
                kind: ConversationMemoryKind::Speech,
                target_label: Some("room".to_string()),
                text: text.to_string(),
            },
        );
        apply_world_hook_effects(
            state,
            content,
            hook_ids::SPEECH,
            json!({
                "event_kind": "speech",
                "actor_id": actor_id,
                "participant_a_id": actor_id,
                "participant_b_id": audience_actor_id,
            }),
        )
        .unwrap_or_else(|error| eprintln!("[cinder] hook warning (speech): {error}"));
    }
    if state.current_room_id == room_id {
        lines.push(render_actor_speech_line(content, actor_name, None, text));
    }
    lines.extend(advance_house_progress_objectives(state, content));
}

pub(super) fn handle_pair_stat_adjusted(
    state: &mut WorldState,
    participant_a_id: &str,
    participant_b_id: &str,
    stat: &str,
    delta: i32,
) {
    let _ = state.adjust_pair_stat(participant_a_id, participant_b_id, stat, delta);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_actor_command_used_event(
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
    lines: &mut Vec<String>,
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
    ) {
        lines.extend(new_lines);
    }
}

pub(super) fn handle_actor_observed_room(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    actor_name: &str,
    room_id: &str,
    lines: &mut Vec<String>,
) {
    if let Some(room) = content.room(room_id) {
        state.mark_actor_observed_room(actor_id, room_id);
        state.push_actor_observation_note(actor_id, room.inspect_text.clone());
        if state.current_room_id == room_id {
            lines.push(format!(
                "{actor_name} pauses to take in the {} more carefully.",
                room.title
            ));
        }
    } else {
        lines.push(content.presentation.error_text.room_missing.clone());
    }
}

pub(super) fn handle_actor_observed_feature(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    actor_name: &str,
    room_id: &str,
    feature_id: &str,
    lines: &mut Vec<String>,
) {
    if let Some((room, feature)) = content.room(room_id).and_then(|room| {
        room.features
            .iter()
            .find(|feature| feature.id == feature_id)
            .map(|feature| (room, feature))
    }) {
        state.mark_actor_feature_seen(actor_id, room_id, feature_id);
        state.push_actor_observation_note(actor_id, feature.inspect_text.clone());
        if state.current_room_id == room_id {
            lines.push(format!("{actor_name} studies the {}.", feature.label));
        }
        let _ = room;
    } else {
        lines.push(content.presentation.error_text.room_missing.clone());
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_actor_observed_actor(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    actor_name: &str,
    room_id: &str,
    target_actor_id: &str,
    target_actor_name: &str,
    lines: &mut Vec<String>,
) {
    if let Some(target_actor) = content.actor(target_actor_id) {
        state.mark_actor_studied_actor(actor_id, target_actor_id);
        state.push_actor_observation_note(actor_id, target_actor.inspect_text.clone());
        if state.current_room_id == room_id {
            lines.push(format!(
                "{actor_name} studies {target_actor_name} more closely."
            ));
        }
    } else {
        lines.push(content.presentation.error_text.actor_unknown.clone());
    }
}

pub(super) fn handle_actor_relocated(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    to_room_id: &str,
    lines: &mut Vec<String>,
) {
    state.mark_actor_room_visited(actor_id, to_room_id);
    state
        .actor_room_overrides
        .insert(actor_id.to_string(), to_room_id.to_string());
    lines.extend(advance_house_progress_objectives(state, content));
}

pub(super) fn handle_actor_moved(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    from_room_id: &str,
    to_room_id: &str,
    lines: &mut Vec<String>,
) {
    apply_actor_move_transition(
        state,
        content,
        ActorMoveTransitionContext {
            actor_id,
            actor_name: None,
            from_room_id,
            to_room_id,
            command_text: None,
        },
        lines,
    );
}

pub(super) fn handle_player_moved(
    state: &mut WorldState,
    content: &ContentPack,
    to_room_id: &str,
    lines: &mut Vec<String>,
) {
    let from_room_id = state.current_room_id.clone();
    lines.extend(advance_objective_for_signal(
        state,
        content,
        &format!("room_left:{from_room_id}"),
    ));
    state.current_room_id = to_room_id.to_string();
    lines.extend(advance_objective_for_signal(
        state,
        content,
        &format!("room_entered:{to_room_id}"),
    ));
}

pub(super) fn handle_menu_opened(
    state: &mut WorldState,
    content: &ContentPack,
    menu_id: &str,
    lines: &mut Vec<String>,
) {
    state.active_menu_id = Some(menu_id.to_string());
    lines.extend(advance_objective_for_signal(
        state,
        content,
        &format!("menu_opened:{menu_id}"),
    ));
}

pub(super) fn handle_menu_choice_made(
    state: &mut WorldState,
    content: &ContentPack,
    menu_id: &str,
    option_id: &str,
    title: &str,
    lines: &mut Vec<String>,
) {
    state.active_menu_id = None;
    if let Some(menu) = content.menu(menu_id) {
        state
            .story_vars
            .insert("selection_title".to_string(), title.to_string());
        if !menu.selection_var_key.is_empty() {
            state
                .story_vars
                .insert(menu.selection_var_key.clone(), title.to_string());
        }
        if !menu.selection_id_var_key.is_empty() {
            state
                .story_vars
                .insert(menu.selection_id_var_key.clone(), option_id.to_string());
        }
        lines.push(super::observation::render_story_text(
            &menu.selection_confirmation,
            state,
        ));
    }
    apply_world_hook_effects(
        state,
        content,
        &format!("menu.{menu_id}.selected"),
        json!({
            "menu_id": menu_id,
            "option_id": option_id,
            "title": title,
        }),
    )
    .unwrap_or_else(|error| eprintln!("[cinder] hook warning (menu.selected): {error}"));
    lines.extend(advance_objective_for_signal(
        state,
        content,
        &format!("menu_selected:{menu_id}"),
    ));
}

pub(super) fn handle_narrative_line(text: &str, lines: &mut Vec<String>) {
    lines.push(text.to_string());
}

pub(super) fn handle_action_rejected(message: &str, lines: &mut Vec<String>) {
    lines.push(message.to_string());
}

pub(super) fn handle_help_shown(
    _state: &mut WorldState,
    content: &ContentPack,
    lines: &mut Vec<String>,
) {
    let available_commands = player_command_help_text(content);
    lines.push(content.render_template(
        &content.opening.help_text,
        &[("available_commands", available_commands.as_str())],
    ));
}

pub(super) fn handle_unknown_input(
    content: &ContentPack,
    raw_input: &str,
    lines: &mut Vec<String>,
) {
    let available_commands = player_command_suggestions(content);
    lines.push(content.render_template(
        &content.presentation.error_text.unknown_input,
        &[
            ("raw_input", raw_input),
            ("available_commands", available_commands.as_str()),
        ],
    ));
}

pub(super) fn handle_session_ended(
    state: &mut WorldState,
    content: &ContentPack,
    lines: &mut Vec<String>,
) {
    state.game_over = true;
    lines.push(content.presentation.presentation_text.session_ended.clone());
}

pub(super) fn apply_content_event(
    state: &mut WorldState,
    content: &ContentPack,
    event_id: &str,
    payload: &std::collections::BTreeMap<String, String>,
    lines: &mut Vec<String>,
) {
    let event = content
        .content_event(event_id)
        .unwrap_or_else(|| panic!("missing content event definition '{event_id}'"));
    let template_values: Vec<_> = payload
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    if !event.event_text.is_empty() {
        lines.push(content.render_template(&event.event_text, &template_values));
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
        let rendered_signal = content.render_template(signal, &template_values);
        lines.extend(advance_objective_for_signal(
            state,
            content,
            &rendered_signal,
        ));
    }
    if !event.open_menu.is_empty() {
        handle_menu_opened(state, content, &event.open_menu, lines);
    }
}

pub(super) fn handle_item_acquired(
    state: &mut WorldState,
    content: &ContentPack,
    item_id: &str,
    lines: &mut Vec<String>,
) {
    let label = content
        .item(item_id)
        .map(|i| i.label.as_str())
        .unwrap_or(item_id);
    state.add_item(item_id);
    lines.push(format!("You have {label} ready."));
}

pub(super) fn handle_item_consumed(
    state: &mut WorldState,
    content: &ContentPack,
    item_id: &str,
    consumer_id: &str,
    consumer_name: &str,
    lines: &mut Vec<String>,
) {
    let label = content
        .item(item_id)
        .map(|i| i.label.as_str())
        .unwrap_or(item_id);
    if state.remove_item(item_id) {
        if consumer_id == "player" {
            lines.push(format!("You drink the {label}."));
        } else {
            lines.push(format!("{consumer_name} accepts the {label}."));
        }
    }
}

pub(super) fn handle_item_observed(
    _state: &mut WorldState,
    content: &ContentPack,
    item_id: &str,
    lines: &mut Vec<String>,
) {
    if let Some(item) = content.item(item_id) {
        lines.push(item.description.clone());
    } else {
        lines.push(content.presentation.error_text.feature_unknown.clone());
    }
}
