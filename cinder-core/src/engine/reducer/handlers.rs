use super::beat_advance::{advance_objective_for_signal, time_reached_signals};
use super::command_effects::{
    ActorMoveTransitionContext, actor_display_name, apply_actor_move_transition,
    defeat_player_if_dead, handle_actor_command_used,
};
use super::observation::{
    render_actor_speech_line, render_feature_consumables_line, render_room_observation,
    render_story_text,
};
use super::tick::{
    advance_actor_stats_on_tick, advance_house_progress_objectives,
    advance_stat_threshold_objectives, increment_shared_room_safety,
};
use crate::content::types::{ContentPack, ItemStorageTarget};
use crate::engine::commands::{player_command_help_text, player_command_suggestions};
use crate::engine::events::{ObservationMode, WorldEvent};
use crate::engine::hook_ids;
use crate::engine::hooks::apply_world_hook_effects;
use crate::engine::narrative::NarrativeLines;
use crate::engine::state::{
    ActorStance, ConversationMemoryKind, ConversationMemoryLine, GamePhase, WorldState,
};
use crate::engine::turn_policies::{
    BundleSpeechEvent, mark_actor_bundle_progress_for_speech_event,
};
use serde_json::{Value, json};

pub(super) fn handle_turn_started(
    state: &mut WorldState,
    content: &ContentPack,
    turn_number: u32,
    _raw_input: &str,
    advances_time: bool,
    lines: &mut NarrativeLines,
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
            lines.extend_narration(advance_objective_for_signal(state, content, &signal));
        }
        lines.extend_narration(advance_house_progress_objectives(state, content));
        lines.extend_narration(advance_stat_threshold_objectives(state, content));
    }
    // Strike policy is external (tick workflows, rules or LLM); the reducer
    // only resolves declared HostileStrike events.
}

/// Generic mechanics for a declared hostile strike: validates eligibility,
/// applies stat-based damage, emits the pack-authored narration, and reschedules
/// the actor's cooldown. Strike *policy* (who strikes, when) lives in tick
/// behaviors, never here.
pub(super) fn handle_hostile_strike(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    lines: &mut NarrativeLines,
) {
    let combat = &content.settings.combat;
    if state.phase != GamePhase::Active
        || state.stance(actor_id) != ActorStance::Hostile
        || state.actor_stat(actor_id, &combat.health_stat_id) <= 0
    {
        return;
    }
    if state.current_time_minutes < *state.next_hostile_strike_at.get(actor_id).unwrap_or(&0) {
        return;
    }
    let default_room_id = content
        .actor(actor_id)
        .map(|actor| actor.room_id.clone())
        .unwrap_or_default();
    if state.actor_room_id(actor_id, &default_room_id) != state.current_room_id {
        return;
    }
    let damage = (state.actor_stat(actor_id, &combat.attack_stat_id)
        - state.effective_actor_stat(content, &combat.player_actor_id, &combat.defense_stat_id))
    .max(combat.minimum_damage);
    state
        .adjust_actor_stat(&combat.player_actor_id, &combat.health_stat_id, -damage)
        .unwrap_or_else(|error| eprintln!("[cinder] combat stat error: {error}"));
    let remaining =
        state.effective_actor_stat(content, &combat.player_actor_id, &combat.health_stat_id);
    let actor_name = actor_display_name(content, actor_id);
    if let Some(line) = content.render_message(
        "combat.hostile_strike",
        &[
            ("actor", actor_name.as_str()),
            ("damage", damage.to_string().as_str()),
            ("remaining", remaining.to_string().as_str()),
        ],
    ) {
        lines.narration(line);
    }
    let interval = content
        .actor(actor_id)
        .map(|actor| actor.attack_interval_minutes(combat.default_attack_interval_minutes))
        .unwrap_or(combat.default_attack_interval_minutes);
    state
        .next_hostile_strike_at
        .insert(actor_id.to_string(), state.current_time_minutes + interval);
    defeat_player_if_dead(state, content, lines);
}

pub(super) fn handle_current_room_observed(
    state: &mut WorldState,
    content: &ContentPack,
    room_id: &str,
    mode: ObservationMode,
    lines: &mut NarrativeLines,
) {
    if let Some(observation) = render_room_observation(content, state, room_id, mode) {
        lines.extend(observation.0);
    } else {
        lines.error(content.presentation.error_text.room_missing.clone());
    }
}

pub(super) fn handle_feature_observed(
    state: &mut WorldState,
    content: &ContentPack,
    room_id: &str,
    feature_id: &str,
    lines: &mut NarrativeLines,
) {
    if let Some(feature) = content.room(room_id).and_then(|room| {
        room.features
            .iter()
            .find(|feature| feature.id == feature_id)
    }) {
        lines.narration(feature.inspect_text.clone());
        if let Some(consumables_line) =
            render_feature_consumables_line(content, state, room_id, feature_id)
        {
            lines.narration(consumables_line);
        }
    } else {
        lines.narration(content.presentation.error_text.room_missing.clone());
    }
}

pub(super) fn handle_actor_observed(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    lines: &mut NarrativeLines,
) {
    if let Some(actor) = content.actor(actor_id) {
        lines.narration(render_story_text(&actor.inspect_text, state));
    } else {
        lines.narration(content.presentation.error_text.actor_unknown.clone());
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
    lines: &mut NarrativeLines,
) {
    mark_actor_bundle_progress_for_speech_event(
        content,
        state,
        actor_id,
        BundleSpeechEvent::ToActor,
    );
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
        lines.narration(render_actor_speech_line(
            content,
            actor_name,
            Some(other_person_name),
            text,
        ));
    }
    lines.extend_narration(advance_house_progress_objectives(state, content));
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
    lines: &mut NarrativeLines,
) {
    mark_actor_bundle_progress_for_speech_event(
        content,
        state,
        actor_id,
        BundleSpeechEvent::ToRoom,
    );
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
        lines.narration(render_actor_speech_line(content, actor_name, None, text));
    }
    lines.extend_narration(advance_house_progress_objectives(state, content));
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

pub(super) fn handle_actor_observed_room(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    actor_name: &str,
    room_id: &str,
    lines: &mut NarrativeLines,
) {
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

pub(super) fn handle_actor_observed_feature(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    actor_name: &str,
    room_id: &str,
    feature_id: &str,
    lines: &mut NarrativeLines,
) {
    if let Some((room, feature)) = content.room(room_id).and_then(|room| {
        room.features
            .iter()
            .find(|feature| feature.id == feature_id)
            .map(|feature| (room, feature))
    }) {
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
        let _ = room;
    } else {
        lines.narration(content.presentation.error_text.room_missing.clone());
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
    lines: &mut NarrativeLines,
) {
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

pub(super) fn handle_actor_relocated(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    to_room_id: &str,
    lines: &mut NarrativeLines,
) {
    state.mark_actor_room_visited(actor_id, to_room_id);
    state
        .actor_room_overrides
        .insert(actor_id.to_string(), to_room_id.to_string());
    lines.extend_narration(advance_house_progress_objectives(state, content));
}

pub(super) fn handle_actor_moved(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    from_room_id: &str,
    to_room_id: &str,
    lines: &mut NarrativeLines,
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
    lines: &mut NarrativeLines,
) {
    let from_room_id = state.current_room_id.clone();
    lines.extend_narration(advance_objective_for_signal(
        state,
        content,
        &format!("room_left:{from_room_id}"),
    ));
    state.current_room_id = to_room_id.to_string();
    lines.extend_narration(advance_objective_for_signal(
        state,
        content,
        &format!("room_entered:{to_room_id}"),
    ));
    let follower_actor_ids: Vec<String> = state
        .relationships
        .iter()
        .filter(|(_, relationship)| relationship.follows_player)
        .map(|(actor_id, _)| actor_id.clone())
        .collect();
    for follower_id in follower_actor_ids {
        if follower_id == content.settings.combat.player_actor_id {
            continue;
        }
        if state.actor_stat(&follower_id, &content.settings.combat.health_stat_id) <= 0 {
            continue;
        }
        let default_room_id = content
            .actor(&follower_id)
            .map(|actor| actor.room_id.clone())
            .unwrap_or_default();
        let already_here = state.actor_room_id(&follower_id, &default_room_id) == to_room_id;
        if !already_here {
            state
                .actor_room_overrides
                .insert(follower_id.clone(), to_room_id.to_string());
            if let Some(line) = content.render_message(
                "follow.actor_follows",
                &[("actor", actor_display_name(content, &follower_id).as_str())],
            ) {
                lines.narration(line);
            }
        }
    }
}

pub(super) fn handle_menu_opened(
    state: &mut WorldState,
    content: &ContentPack,
    menu_id: &str,
    lines: &mut NarrativeLines,
) {
    state.active_menu_id = Some(menu_id.to_string());
    state.pending_menu_selections.clear();
    if let Some(menu) = content.menu(menu_id) {
        lines.extend_narration(
            menu.opening_narrative_lines
                .iter()
                .map(|line| render_story_text(line, state)),
        );
    }
    lines.extend_narration(advance_objective_for_signal(
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
    lines: &mut NarrativeLines,
) {
    state.active_menu_id = None;
    if let Some(menu) = content.menu(menu_id) {
        let is_multi_select = menu.max_selections > 0;
        if is_multi_select && option_id == "done" {
            let selected_ids: Vec<String> = state.pending_menu_selections.clone();
            let selected_titles: Vec<String> = selected_ids
                .iter()
                .filter_map(|id| {
                    menu.options
                        .iter()
                        .find(|opt| opt.id == *id)
                        .map(|opt| opt.title.clone())
                })
                .collect();
            let joined_titles = selected_titles.join(", ");
            let joined_ids = selected_ids.join(", ");
            state
                .story_vars
                .set_unchecked("selection_title", &joined_titles);
            if !menu.multi_selection_var_keys.is_empty() {
                for (i, var_key) in menu.multi_selection_var_keys.iter().enumerate() {
                    let value = selected_titles.get(i).cloned().unwrap_or_default();
                    state.story_vars.set_unchecked(var_key, &value);
                }
            }
            if !menu.multi_selection_room_var_keys.is_empty() {
                let selected_rooms: Vec<String> = selected_ids
                    .iter()
                    .filter_map(|id| {
                        menu.options
                            .iter()
                            .find(|opt| opt.id == *id)
                            .filter(|opt| !opt.room_id.is_empty())
                            .map(|opt| opt.room_id.clone())
                    })
                    .collect();
                for (i, var_key) in menu.multi_selection_room_var_keys.iter().enumerate() {
                    let value = selected_rooms.get(i).cloned().unwrap_or_default();
                    state.story_vars.set_unchecked(var_key, &value);
                }
            }
            if !menu.multi_selection_host_var_keys.is_empty() {
                let selected_hosts: Vec<String> = selected_ids
                    .iter()
                    .filter_map(|id| {
                        menu.options
                            .iter()
                            .find(|opt| opt.id == *id)
                            .filter(|opt| !opt.host_actor_id.is_empty())
                            .map(|opt| opt.host_actor_id.clone())
                    })
                    .collect();
                for (i, var_key) in menu.multi_selection_host_var_keys.iter().enumerate() {
                    let value = selected_hosts.get(i).cloned().unwrap_or_default();
                    state.story_vars.set_unchecked(var_key, &value);
                }
            }
            if menu.multi_selection_var_keys.is_empty() && !menu.selection_var_key.is_empty() {
                state
                    .story_vars
                    .set_unchecked(&menu.selection_var_key, &joined_titles);
            }
            if !menu.selection_id_var_key.is_empty() {
                state
                    .story_vars
                    .set_unchecked(&menu.selection_id_var_key, &joined_ids);
            }
            lines.narration(super::observation::render_story_text(
                &menu.selection_confirmation,
                state,
            ));
        } else {
            state.story_vars.set_unchecked("selection_title", title);
            if !menu.selection_var_key.is_empty() {
                state
                    .story_vars
                    .set_unchecked(&menu.selection_var_key, title);
            }
            if !menu.selection_id_var_key.is_empty() {
                state
                    .story_vars
                    .set_unchecked(&menu.selection_id_var_key, option_id);
            }
            lines.narration(super::observation::render_story_text(
                &menu.selection_confirmation,
                state,
            ));
        }
        state.pending_menu_selections.clear();
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
    lines.extend_narration(advance_objective_for_signal(
        state,
        content,
        &format!("menu_selected:{menu_id}"),
    ));
}

pub(super) fn handle_menu_selection_toggled(
    state: &mut WorldState,
    content: &ContentPack,
    menu_id: &str,
    option_id: &str,
    selected: bool,
) {
    if let Some(menu) = content.menu(menu_id) {
        let max = menu.max_selections;
        if selected {
            if max > 0 && state.pending_menu_selections.len() >= max {
                return;
            }
            if !state
                .pending_menu_selections
                .contains(&option_id.to_string())
            {
                state.pending_menu_selections.push(option_id.to_string());
            }
        } else {
            state.pending_menu_selections.retain(|id| id != option_id);
        }
    }
}

pub(super) fn handle_narrative_line(text: &str, lines: &mut NarrativeLines) {
    lines.narration(text.to_string());
}

pub(super) fn handle_action_rejected(message: &str, lines: &mut NarrativeLines) {
    if !message.is_empty() {
        lines.error(message.to_string());
    }
}

pub(super) fn handle_help_shown(
    _state: &mut WorldState,
    content: &ContentPack,
    lines: &mut NarrativeLines,
) {
    let available_commands = player_command_help_text(content);
    lines.narration(content.render_template(
        &content.opening.help_text,
        &[("available_commands", available_commands.as_str())],
    ));
}

pub(super) fn handle_unknown_input(
    content: &ContentPack,
    raw_input: &str,
    lines: &mut NarrativeLines,
) {
    let available_commands = player_command_suggestions(content);
    lines.narration(content.render_template(
        &content.presentation.error_text.unknown_input,
        &[
            ("raw_input", raw_input),
            ("available_commands", available_commands.as_str()),
        ],
    ));
}

pub(super) fn handle_act_ended(
    state: &mut WorldState,
    _content: &ContentPack,
    _lines: &mut NarrativeLines,
) {
    state.phase = crate::engine::state::GamePhase::ActEnded;
}

pub(super) fn apply_content_event(
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
        lines.narration(super::observation::render_story_text(
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
        let rendered_signal = super::observation::render_story_text(
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

pub(super) fn handle_item_acquired(
    state: &mut WorldState,
    content: &ContentPack,
    item_id: &str,
    storage: ItemStorageTarget,
    lines: &mut NarrativeLines,
) {
    let label = content
        .item(item_id)
        .map(|i| i.label.as_str())
        .unwrap_or(item_id);
    let room_id = state.current_room_id.clone();
    state.add_item_to_storage(item_id, storage, &room_id);
    match storage {
        ItemStorageTarget::PlayerInventory => {
            if let Some(line) =
                render_acquired_line(content, item_id, "item.acquired_inventory", label)
            {
                lines.narration(line);
            }
        }
        ItemStorageTarget::CurrentRoom => {
            if let Some(line) = render_acquired_line(content, item_id, "item.acquired_room", label)
            {
                lines.narration(line);
            }
            // An item appearing in a room can complete an encirclement.
            super::command_effects::run_encirclement_rules(state, content, item_id, lines);
        }
    }
}

/// Renders the "item acquired" narration for a single item. A pack may define
/// `item.<id>.<generic key>` to override the message; an empty override
/// suppresses the line entirely.
fn render_acquired_line(
    content: &ContentPack,
    item_id: &str,
    generic_key: &str,
    label: &str,
) -> Option<String> {
    let specific_key = format!(
        "item.{item_id}.{}",
        generic_key.strip_prefix("item.").unwrap_or(generic_key)
    );
    if let Some(text) = content.message(&specific_key) {
        if text.is_empty() {
            return None;
        }
        return Some(content.render_template(text, &[("label", label)]));
    }
    content.render_message(generic_key, &[("label", label)])
}

pub(super) fn handle_item_consumed(
    state: &mut WorldState,
    content: &ContentPack,
    item_id: &str,
    storage: ItemStorageTarget,
    consumer_id: Option<&str>,
    consumer_name: Option<&str>,
    lines: &mut NarrativeLines,
) {
    let label = content
        .item(item_id)
        .map(|i| i.label.as_str())
        .unwrap_or(item_id);
    let room_id = state.current_room_id.clone();
    if state.remove_item_from_storage(item_id, storage, &room_id) {
        if consumer_id == Some(content.settings.combat.player_actor_id.as_str()) {
            if let Some(line) = content.render_message("item.consumed_player", &[("label", label)])
            {
                lines.narration(line);
            }
        } else if let Some(consumer_name) = consumer_name {
            if let Some(line) = content.render_message(
                "item.consumed_actor",
                &[("consumer_name", consumer_name), ("label", label)],
            ) {
                lines.narration(line);
            }
        } else if let Some(line) = content.render_message("item.consumed_use", &[("label", label)])
        {
            lines.narration(line);
        }
    }
}

pub(super) fn handle_item_observed(
    _state: &mut WorldState,
    content: &ContentPack,
    item_id: &str,
    lines: &mut NarrativeLines,
) {
    if let Some(item) = content.item(item_id) {
        lines.narration(item.description.clone());
    } else {
        lines.narration(content.presentation.error_text.feature_unknown.clone());
    }
}
