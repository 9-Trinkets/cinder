use crate::content::types::{ActorDefinition, ContentPack, RoomDefinition};
use crate::engine::dialogue::DialogueRequest;
use crate::engine::hooks::{actor_state_notes, pair_state_note};
use crate::engine::state::{
    ConversationMemoryKind, ConversationMemoryLine, WorldState, display_actor_name,
    remap_story_actor_id, render_dynamic_story_text, resolved_actor_prompt_context,
};
use std::collections::BTreeMap;
const ROOM_RECENT_MEMORY_LIMIT: usize = 8;

pub(crate) fn build_grounded_dialogue_request(
    content: &ContentPack,
    state: &WorldState,
    actor_id: &str,
    current_room_id: &str,
    other_person_message: Option<String>,
) -> Result<DialogueRequest, String> {
    build_grounded_dialogue_request_for_exchange(
        content,
        state,
        actor_id,
        current_room_id,
        &viewer_participant_id(content),
        &content.opening.title,
        other_person_message,
    )
}

pub(crate) fn build_grounded_dialogue_request_for_exchange(
    content: &ContentPack,
    state: &WorldState,
    actor_id: &str,
    current_room_id: &str,
    other_person_id: &str,
    other_person_name: &str,
    other_person_message: Option<String>,
) -> Result<DialogueRequest, String> {
    let actor_id = remap_story_actor_id(state, actor_id);
    let actor = content
        .actor(actor_id)
        .ok_or_else(|| format!("missing actor '{actor_id}'"))?;
    let room = content
        .room(current_room_id)
        .ok_or_else(|| format!("missing room '{current_room_id}'"))?;
    let recent_memory = recent_exchange_memory(
        state,
        actor_id,
        other_person_id,
        other_person_message.as_deref(),
    );
    let current_time_note = content.render_template(
        &content.system_text.prompt_time_note,
        &[("current_time", state.current_time_label().as_str())],
    );
    let setting_notes = build_setting_notes(content, state, actor, room, &current_time_note);
    let current_beat_notes = build_current_beat_notes(
        content,
        room,
        other_person_id,
        other_person_name,
        other_person_message.as_deref(),
        &current_objective_beat_notes(content, state),
    );
    let prompt_context = resolved_actor_prompt_context(content, state, actor);
    let actor_name = display_actor_name(state, actor);
    let mut response_notes = prompt_context.response_notes.clone();
    response_notes.push(content.render_template(
        &content.system_text.prompt_address_other_person_note,
        &[("other_person_name", other_person_name)],
    ));
    let mut subtext_notes = prompt_context.subtext_notes.clone();
    if other_person_id == viewer_participant_id(content) {
        subtext_notes.extend(content.opening.prompt_context.subtext_notes.clone());
    }
    if let Some(note) = pair_state_note(
        content,
        state,
        actor_id,
        other_person_id,
        other_person_name,
        &BTreeMap::new(),
    ) {
        subtext_notes.push(note);
    }
    subtext_notes.extend(actor_state_notes(content, state, actor_id));
    Ok(DialogueRequest {
        actor_id: actor.id.clone(),
        actor_name,
        current_room_id: current_room_id.to_string(),
        other_person_id: other_person_id.to_string(),
        other_person_name: other_person_name.to_string(),
        locale: content.locale.clone(),
        system_text: content.system_text.clone(),
        character_notes: prompt_context.character_notes,
        setting_notes,
        current_beat_notes,
        subtext_notes,
        behavior_examples: prompt_context.behavior_examples,
        response_notes,
        other_person_message,
        recent_memory_summary: state
            .conversation_summary(actor_id, other_person_id)
            .map(str::to_string),
        recent_memory,
        include_conversation_summary_section: true,
        include_latest_line_section: true,
    })
}

pub(crate) fn recent_room_memory(
    content: &ContentPack,
    state: &WorldState,
    actor_id: &str,
    current_room_id: &str,
) -> Vec<ConversationMemoryLine> {
    let mut room_memory_lines = content
        .actors
        .iter()
        .filter(|other_actor| {
            other_actor.id != actor_id
                && state.actor_room_id(&other_actor.id, &other_actor.room_id) == current_room_id
        })
        .flat_map(|other_actor| state.conversation_history(actor_id, &other_actor.id).iter())
        .cloned()
        .collect::<Vec<_>>();
    room_memory_lines.sort_by_key(|line| (line.event_sequence, line.turn_number));
    if room_memory_lines.len() > ROOM_RECENT_MEMORY_LIMIT {
        room_memory_lines.drain(..room_memory_lines.len() - ROOM_RECENT_MEMORY_LIMIT);
    }
    room_memory_lines
}

pub(crate) fn build_grounded_dialogue_request_for_room(
    content: &ContentPack,
    state: &WorldState,
    actor_id: &str,
    current_room_id: &str,
    audience_label: &str,
) -> Result<DialogueRequest, String> {
    let actor_id = remap_story_actor_id(state, actor_id);
    let actor = content
        .actor(actor_id)
        .ok_or_else(|| format!("missing actor '{actor_id}'"))?;
    let room = content
        .room(current_room_id)
        .ok_or_else(|| format!("missing room '{current_room_id}'"))?;
    let current_time_note = content.render_template(
        &content.system_text.prompt_time_note,
        &[("current_time", state.current_time_label().as_str())],
    );
    let setting_notes = build_setting_notes(content, state, actor, room, &current_time_note);
    let current_beat_notes = current_objective_beat_notes(content, state);
    let prompt_context = resolved_actor_prompt_context(content, state, actor);
    let actor_name = display_actor_name(state, actor);
    let mut response_notes = prompt_context.response_notes.clone();
    response_notes.push(content.render_template(
        &content.system_text.prompt_address_other_person_note,
        &[("other_person_name", audience_label)],
    ));
    let mut subtext_notes = prompt_context.subtext_notes.clone();
    subtext_notes.extend(actor_state_notes(content, state, actor_id));
    Ok(DialogueRequest {
        actor_id: actor.id.clone(),
        actor_name,
        current_room_id: current_room_id.to_string(),
        other_person_id: format!("room:{current_room_id}"),
        other_person_name: audience_label.to_string(),
        locale: content.locale.clone(),
        system_text: content.system_text.clone(),
        character_notes: prompt_context.character_notes,
        setting_notes,
        current_beat_notes,
        subtext_notes,
        behavior_examples: prompt_context.behavior_examples,
        response_notes,
        other_person_message: None,
        recent_memory_summary: None,
        recent_memory: recent_room_memory(content, state, actor_id, current_room_id),
        include_conversation_summary_section: false,
        include_latest_line_section: false,
    })
}

pub(crate) fn recent_exchange_memory(
    state: &WorldState,
    actor_id: &str,
    other_person_id: &str,
    other_person_message: Option<&str>,
) -> Vec<crate::engine::state::ConversationMemoryLine> {
    state
        .conversation_history(actor_id, other_person_id)
        .iter()
        .rev()
        .filter(|line| {
            !other_person_message
                .is_some_and(|message| line.speaker_id == other_person_id && line.text == message)
        })
        .take(6)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
}

pub(crate) fn latest_other_person_message(
    state: &WorldState,
    actor_id: &str,
    other_person_id: &str,
) -> Option<String> {
    state
        .conversation_history(actor_id, other_person_id)
        .iter()
        .rev()
        .find(|line| {
            line.speaker_id == other_person_id && line.kind == ConversationMemoryKind::Speech
        })
        .map(|line| line.text.clone())
}

pub(crate) fn viewer_participant_id(content: &ContentPack) -> String {
    format!("viewer:{}", content.opening.id)
}

pub(crate) fn current_objective_beat_notes(
    content: &ContentPack,
    state: &WorldState,
) -> Vec<String> {
    state
        .active_objective_stage_ids
        .iter()
        .filter_map(|stage_id| {
            content
                .beats
                .stages
                .iter()
                .find(|stage| stage.id == *stage_id)
        })
        .map(|stage| render_story_text(&stage.beat_note, state))
        .filter(|note| !note.is_empty())
        .collect()
}

pub(crate) fn render_story_text(template: &str, state: &WorldState) -> String {
    render_dynamic_story_text(template, state)
}

pub(crate) fn build_setting_notes(
    content: &ContentPack,
    state: &WorldState,
    actor: &ActorDefinition,
    room: &RoomDefinition,
    current_time_note: &str,
) -> Vec<String> {
    let mut notes = Vec::new();
    notes.push(content.render_template(
        &content.system_text.prompt_current_room_note,
        &[("room_title", room.title.as_str())],
    ));
    notes.push(current_time_note.to_string());
    notes.extend(content.opening.prompt_context.setting_notes.iter().cloned());
    notes.push(room.summary.clone());

    if !room.features.is_empty() {
        let features = room
            .features
            .iter()
            .map(|feature| feature.label.as_str())
            .collect::<Vec<_>>();
        let features = natural_join(&features);
        notes.push(content.render_template(
            &content.system_text.prompt_visible_features_note,
            &[("features", features.as_str())],
        ));
    }

    let other_people = content
        .actors
        .iter()
        .filter(|other| {
            state.actor_room_id(&other.id, &other.room_id) == room.id && other.id != actor.id
        })
        .map(|other| display_actor_name(state, other))
        .collect::<Vec<_>>();
    if !other_people.is_empty() {
        let people_refs = other_people.iter().map(String::as_str).collect::<Vec<_>>();
        let people = natural_join(&people_refs);
        notes.push(content.render_template(
            &content.system_text.prompt_people_here_note,
            &[("people", people.as_str())],
        ));
    }

    let exits = room
        .exits
        .iter()
        .filter_map(|exit| content.room(&exit.room_id))
        .map(|reachable_room| reachable_room.title.as_str())
        .collect::<Vec<_>>();
    if !exits.is_empty() {
        let exits = natural_join(&exits);
        notes.push(content.render_template(
            &content.system_text.prompt_exits_note,
            &[("exits", exits.as_str())],
        ));
    }
    notes.extend(
        state
            .actor_recent_observation_notes(&actor.id)
            .iter()
            .cloned(),
    );
    notes
}

pub(crate) fn build_current_beat_notes(
    content: &ContentPack,
    room: &RoomDefinition,
    other_person_id: &str,
    other_person_name: &str,
    player_message: Option<&str>,
    objective_notes: &[String],
) -> Vec<String> {
    let mut notes = Vec::new();
    if player_message.is_some() {
        notes.push(content.render_template(
            &content.system_text.prompt_current_speaker_note,
            &[
                ("other_person_name", other_person_name),
                ("room_title", room.title.as_str()),
            ],
        ));
    } else if other_person_id != viewer_participant_id(content) {
        notes.push(content.render_template(
            &content.system_text.prompt_shared_room_note,
            &[
                ("other_person_name", other_person_name),
                ("room_title", room.title.as_str()),
            ],
        ));
    }
    notes.extend(objective_notes.iter().cloned());
    let referenced_features = collect_referenced_features(room, player_message);
    if !referenced_features.is_empty() {
        let features = referenced_features
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        let features = natural_join(&features);
        notes.push(content.render_template(
            &content.system_text.prompt_latest_words_note,
            &[("features", features.as_str())],
        ));
    }
    notes
}

fn collect_referenced_features(room: &RoomDefinition, player_message: Option<&str>) -> Vec<String> {
    let Some(message) = player_message else {
        return Vec::new();
    };
    let lower = message.to_ascii_lowercase();
    room.features
        .iter()
        .filter(|feature| {
            lower.contains(&feature.label.to_ascii_lowercase())
                || lower.contains(&feature.id.to_ascii_lowercase())
                || feature
                    .aliases
                    .iter()
                    .any(|alias| lower.contains(&alias.to_ascii_lowercase()))
        })
        .map(|feature| feature.label.clone())
        .collect()
}

fn natural_join(items: &[&str]) -> String {
    match items {
        [] => String::new(),
        [only] => (*only).to_string(),
        [first, second] => format!("{first} and {second}"),
        _ => {
            let mut parts = items[..items.len() - 1].join(", ");
            parts.push_str(", and ");
            parts.push_str(items[items.len() - 1]);
            parts
        }
    }
}
