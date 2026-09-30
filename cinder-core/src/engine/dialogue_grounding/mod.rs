use crate::content::types::{ActorDefinition, ContentPack, RoomDefinition};
use crate::engine::dialogue::DialogueRequest;
use crate::engine::hooks::{actor_state_notes, pair_state_note};
use crate::engine::state::{
    ConversationMemoryKind, ConversationMemoryLine, WorldState, display_actor_name,
    remap_story_actor_id, render_dynamic_story_text, resolved_actor_prompt_context,
};
use crate::engine::turn_policies::actor_objective_guidance_notes;
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
    let objective_notes = current_stage_and_objective_notes(content, state, actor_id);
    let current_beat_notes = build_current_beat_notes(
        content,
        room,
        other_person_id,
        other_person_name,
        other_person_message.as_deref(),
        &objective_notes,
    );
    let prompt_context = resolved_actor_prompt_context(content, state, actor);
    let actor_name = display_actor_name(state, actor);
    let mut response_notes = prompt_context.response_notes.clone();
    response_notes.push(content.render_template(
        &content.system_text.prompt_address_other_person_note,
        &[("other_person_name", other_person_name)],
    ));
    let actor_items: Vec<(String, u32)> = state
        .actor_inventory(actor_id)
        .into_iter()
        .filter(|(_, count)| *count > 0)
        .collect();
    if !actor_items.is_empty() {
        let items_str = actor_items
            .iter()
            .map(|(item_id, count)| {
                let label = content
                    .item(item_id)
                    .map(|i| i.label.as_str())
                    .unwrap_or(item_id.as_str());
                if *count > 1 {
                    format!("{label} (id: '{item_id}', qty: {count})")
                } else {
                    format!("{label} (id: '{item_id}')")
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        response_notes.push(format!(
            "You are carrying: {items_str}. If you choose to give an item to {other_person_name} as part of your response, append '[GIVE: <item_id>]' (e.g. '[GIVE: date-flatbread]') to your speech. Only give items you are currently carrying."
        ));
    }
    let mut subtext_notes = prompt_context.subtext_notes.clone();
    if other_person_id == viewer_participant_id(content) {
        subtext_notes.extend(content.opening.prompt_context.subtext_notes.clone());
    }
    if let Some(note) =
        pair_state_note(content, state, actor_id, other_person_id, other_person_name)
    {
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
                && state.actor_is_in_room(content, &other_actor.id, current_room_id)
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
    let current_beat_notes = current_stage_and_objective_notes(content, state, actor_id);
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
    actor_id: Option<&str>,
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
        .filter(|stage| {
            if stage.target_actor_story_var.is_empty() {
                return true;
            }
            let Some(actor_id) = actor_id else {
                return true;
            };
            state
                .story_vars
                .get(&stage.target_actor_story_var)
                .map(|ids| ids.split(',').any(|id| id.trim() == actor_id))
                .unwrap_or(false)
        })
        .map(|stage| render_story_text(&stage.beat_note, state))
        .filter(|note| !note.is_empty())
        .collect()
}

fn current_stage_and_objective_notes(
    content: &ContentPack,
    state: &WorldState,
    actor_id: &str,
) -> Vec<String> {
    let mut notes = current_objective_beat_notes(content, state, Some(actor_id));
    notes.extend(actor_objective_guidance_notes(content, state, actor_id));
    notes
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
            state.actor_is_in_room(content, &other.id, &room.id) && other.id != actor.id
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
        .map(|exit| exit.label.as_str())
        .collect::<Vec<_>>();
    if !exits.is_empty() {
        let exits = exits.join(", ");
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

pub(crate) fn extract_gift_tags(raw: &str) -> (String, Vec<String>) {
    let mut gifts = Vec::new();
    let mut clean = String::with_capacity(raw.len());
    let mut cursor = 0;

    while let Some(start_offset) = raw[cursor..].find('[') {
        let open_idx = cursor + start_offset;
        clean.push_str(&raw[cursor..open_idx]);

        if let Some(close_offset) = raw[open_idx..].find(']') {
            let close_idx = open_idx + close_offset;
            let bracketed = raw[open_idx + 1..close_idx].trim();
            if bracketed.len() >= 5 && bracketed[..5].eq_ignore_ascii_case("give:") {
                let item_id = bracketed[5..]
                    .trim()
                    .trim_matches(|c| c == '\'' || c == '\"');
                if !item_id.is_empty() {
                    gifts.push(item_id.to_string());
                }
                cursor = close_idx + 1;
            } else {
                clean.push('[');
                cursor = open_idx + 1;
            }
        } else {
            clean.push_str(&raw[open_idx..]);
            cursor = raw.len();
            break;
        }
    }
    clean.push_str(&raw[cursor..]);

    let mut normalized = String::with_capacity(clean.len());
    let mut last_was_space = false;
    for c in clean.chars() {
        if c.is_whitespace() {
            if !last_was_space && !normalized.is_empty() {
                normalized.push(' ');
                last_was_space = true;
            }
        } else {
            if last_was_space
                && (c == '.' || c == ',' || c == '!' || c == '?' || c == ';' || c == ':')
                && normalized.ends_with(' ')
            {
                normalized.pop();
            }
            normalized.push(c);
            last_was_space = false;
        }
    }
    if normalized.ends_with(' ') {
        normalized.pop();
    }

    let final_text = if normalized.is_empty() && !gifts.is_empty() {
        "Here, take this.".to_string()
    } else {
        normalized
    };

    (final_text, gifts)
}

/// Resolves a `[GIVE: ...]` tag to an item the giver can actually hand over.
///
/// Only items in the giver's inventory qualify. Equipped items are excluded:
/// with several items sharing a name (two "leaf spear"s, say), matching on
/// content order alone would hand over the equipped one, which either fails
/// silently or strips the giver's gear. Returns `None` when the giver holds
/// nothing by that name, so no phantom transfer is emitted.
pub(crate) fn resolve_gift_item_id(
    content: &ContentPack,
    state: &WorldState,
    giver_id: &str,
    raw_id: &str,
) -> Option<String> {
    let raw = raw_id.trim();
    let with_dashes = raw.replace(' ', "-");
    let with_spaces = raw.replace('-', " ");
    let held = |item_id: &str| {
        if content.is_player_actor(giver_id) {
            state.has_item(item_id)
        } else {
            state.actor_has_item(giver_id, item_id)
        }
    };
    // An exact id names one item, so it wins over any label match.
    let by_id = content.items.iter().find(|item| {
        held(&item.id)
            && (item.id.eq_ignore_ascii_case(raw) || item.id.eq_ignore_ascii_case(&with_dashes))
    });
    by_id
        .or_else(|| {
            content.items.iter().find(|item| {
                held(&item.id)
                    && (item.label.eq_ignore_ascii_case(raw)
                        || item.label.eq_ignore_ascii_case(&with_spaces))
            })
        })
        .map(|item| item.id.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_gift_tag_at_end_of_speech() {
        let raw =
            "Here, take this flatbread with roasted dates; you'll need it. [GIVE: date-flatbread]";
        let (clean, gifts) = extract_gift_tags(raw);
        assert_eq!(
            clean,
            "Here, take this flatbread with roasted dates; you'll need it."
        );
        assert_eq!(gifts, vec!["date-flatbread"]);
    }

    #[test]
    fn extracts_gift_tag_at_start_of_speech() {
        let raw = "[GIVE: date-flatbread] Here, take this.";
        let (clean, gifts) = extract_gift_tags(raw);
        assert_eq!(clean, "Here, take this.");
        assert_eq!(gifts, vec!["date-flatbread"]);
    }

    #[test]
    fn extracts_gift_tag_in_middle_with_proper_spacing() {
        let raw = "Take this [GIVE: date-flatbread], friend.";
        let (clean, gifts) = extract_gift_tags(raw);
        assert_eq!(clean, "Take this, friend.");
        assert_eq!(gifts, vec!["date-flatbread"]);
    }

    #[test]
    fn extracts_multiple_gift_tags_and_strips_quotes() {
        let raw = "Take both! [GIVE: 'date-flatbread'] [give: \"copper-pipe\"]";
        let (clean, gifts) = extract_gift_tags(raw);
        assert_eq!(clean, "Take both!");
        assert_eq!(gifts, vec!["date-flatbread", "copper-pipe"]);
    }

    #[test]
    fn preserves_non_gift_brackets() {
        let raw = "[smiles warmly] Take this! [GIVE: date-flatbread]";
        let (clean, gifts) = extract_gift_tags(raw);
        assert_eq!(clean, "[smiles warmly] Take this!");
        assert_eq!(gifts, vec!["date-flatbread"]);
    }

    #[test]
    fn provides_fallback_for_empty_speech_with_gift() {
        let raw = "[GIVE: date-flatbread]";
        let (clean, gifts) = extract_gift_tags(raw);
        assert_eq!(clean, "Here, take this.");
        assert_eq!(gifts, vec!["date-flatbread"]);
    }

    /// Builds a pack holding two items that share the label "leaf spear", so
    /// name resolution can only pick correctly by consulting the giver.
    fn colliding_spear_pack() -> ContentPack {
        let mut pack = crate::engine::test_fixtures::minimal_test_pack();
        for id in ["spear-a", "spear-b"] {
            pack.items.push(crate::content::types::ItemDefinition {
                id: id.to_string(),
                label: "leaf spear".to_string(),
                description: "A leaf-shaped spear.".to_string(),
                ..crate::content::types::ItemDefinition::default()
            });
        }
        crate::engine::test_fixtures::rebuild_test_pack_indexes(&mut pack);
        pack
    }

    #[test]
    fn gift_resolution_skips_an_equipped_item_that_shares_a_name() {
        let pack = colliding_spear_pack();
        let mut state = crate::engine::state::WorldState::new(&pack);
        // The giver holds spear-b but has spear-a equipped, so a name-only
        // lookup would hand over the equipped weapon.
        state.actor_add_item("gifter", "spear-b");
        state
            .actor_equipment
            .entry("gifter".to_string())
            .or_default()
            .insert("weapon".to_string(), "spear-a".to_string());

        assert_eq!(
            resolve_gift_item_id(&pack, &state, "gifter", "leaf spear"),
            Some("spear-b".to_string()),
            "a gift by name must resolve to the held item, not the equipped one"
        );
    }

    #[test]
    fn gift_resolution_prefers_an_exact_id_over_a_shared_label() {
        let pack = colliding_spear_pack();
        let mut state = crate::engine::state::WorldState::new(&pack);
        state.actor_add_item("gifter", "spear-a");
        state.actor_add_item("gifter", "spear-b");

        assert_eq!(
            resolve_gift_item_id(&pack, &state, "gifter", "spear-a"),
            Some("spear-a".to_string()),
            "an exact id names one item and must win over the shared label"
        );
    }

    #[test]
    fn gift_resolution_returns_none_when_the_giver_holds_no_match() {
        let pack = colliding_spear_pack();
        let state = crate::engine::state::WorldState::new(&pack);

        assert_eq!(
            resolve_gift_item_id(&pack, &state, "gifter", "leaf spear"),
            None,
            "a name the giver does not hold must not resolve to a phantom transfer"
        );
    }

    #[test]
    fn leaves_plain_dialogue_untouched() {
        let raw = "I have nothing for you, wanderer.";
        let (clean, gifts) = extract_gift_tags(raw);
        assert_eq!(clean, "I have nothing for you, wanderer.");
        assert!(gifts.is_empty());
    }
}
