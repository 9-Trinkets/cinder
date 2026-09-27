use crate::content::types::{ContentPack, ItemStorageTarget, PackMessageVoice};
use crate::engine::narrative::NarrativeLines;
use crate::engine::reducer::beat_advance::advance_objective_for_signal;
use crate::engine::reducer::command_effects::{trigger_placement_hooks, trigger_surrounded_hooks};
use crate::engine::state::WorldState;

use super::feedback::{push_message, push_rendered_message};

pub(crate) fn handle_item_acquired(
    state: &mut WorldState,
    content: &ContentPack,
    item_id: &str,
    storage: ItemStorageTarget,
    lines: &mut NarrativeLines,
) {
    let room_id = state.current_room_id.clone();
    if storage == ItemStorageTarget::CurrentRoom
        && content.item(item_id).is_some_and(|item| item.trace_mark)
        && state.has_item_in_storage(item_id, storage, &room_id)
    {
        return;
    }
    let label = content.item_label(item_id);
    state.add_item_to_storage(item_id, storage, &room_id);
    match storage {
        ItemStorageTarget::PlayerInventory => {
            if let Some((line, voice)) =
                render_acquired_line(content, item_id, "item.acquired_inventory", label)
            {
                push_rendered_message(lines, content, line, voice);
            }
            lines.extend_narration(advance_objective_for_signal(
                state,
                content,
                &format!("item_acquired:{item_id}"),
            ));
            lines.extend_narration(advance_objective_for_signal(
                state,
                content,
                "item_acquired",
            ));
        }
        ItemStorageTarget::CurrentRoom => {
            if let Some((line, voice)) =
                render_acquired_line(content, item_id, "item.acquired_room", label)
            {
                push_rendered_message(lines, content, line, voice);
            }
            if item_id == "teleport-sigil"
                && room_id != "teleport_platform"
                && let Some(evicted_room) = state.add_chalk_anchor(&room_id)
            {
                state.remove_item_from_storage(
                    "teleport-sigil",
                    ItemStorageTarget::CurrentRoom,
                    &evicted_room,
                );
                let evicted_title = content
                    .room(&evicted_room)
                    .map(|r| r.title.as_str())
                    .unwrap_or(&evicted_room);
                lines.narration(format!(
                    "The chalk anchor in {evicted_title} fades away as you draw a new one."
                ));
            }
            // An item appearing in a room can complete a surround or trigger placement.
            trigger_surrounded_hooks(state, content, item_id, &room_id, lines);
            trigger_placement_hooks(
                state,
                content,
                item_id,
                &content.settings.combat.player_actor_id,
                &room_id,
                lines,
            );
            if item_id == "teleport-sigil" && room_id == "teleport_platform" {
                state.remove_item_from_storage(
                    "teleport-sigil",
                    ItemStorageTarget::CurrentRoom,
                    &room_id,
                );
            }
        }
    }
}

/// Moves a loose item from the current room into the player's inventory in
/// response to the generic `take <item>` command.
pub(crate) fn handle_player_took_item(
    state: &mut WorldState,
    content: &ContentPack,
    item_id: &str,
    lines: &mut NarrativeLines,
) {
    let room_id = state.current_room_id.clone();
    let label = content.item_label(item_id);
    if content
        .item(item_id)
        .is_some_and(|item| !item.is_takeable())
    {
        push_message(lines, content, "item.takedenied", &[("label", label)]);
        return;
    }
    if state.remove_item_from_storage(item_id, ItemStorageTarget::CurrentRoom, &room_id) {
        state.add_item(item_id);
        push_message(lines, content, "item.taken", &[("label", label)]);
        lines.extend_narration(advance_objective_for_signal(
            state,
            content,
            &format!("item_acquired:{item_id}"),
        ));
        lines.extend_narration(advance_objective_for_signal(
            state,
            content,
            "item_acquired",
        ));
    }
}

/// Moves an inventory item into the current room in response to the generic
/// `drop <item>` command.
pub(crate) fn handle_player_dropped_item(
    state: &mut WorldState,
    content: &ContentPack,
    item_id: &str,
    lines: &mut NarrativeLines,
) {
    let room_id = state.current_room_id.clone();
    let label = content.item_label(item_id);
    if content
        .item(item_id)
        .is_some_and(|item| !item.is_takeable())
    {
        push_message(lines, content, "item.takedenied", &[("label", label)]);
        return;
    }
    if state.remove_item(item_id) {
        state.add_item_to_storage(item_id, ItemStorageTarget::CurrentRoom, &room_id);
        push_message(lines, content, "item.dropped", &[("label", label)]);
        trigger_surrounded_hooks(state, content, item_id, &room_id, lines);
        trigger_placement_hooks(
            state,
            content,
            item_id,
            &content.settings.combat.player_actor_id,
            &room_id,
            lines,
        );
    }
}

/// Consumes an item from player inventory in response to generic `use` / `eat` command,
/// fires its use_hook, narrating the effect and emitting signals.
pub(crate) fn handle_player_used_item(
    state: &mut WorldState,
    content: &ContentPack,
    item_id: &str,
    lines: &mut NarrativeLines,
) {
    let Some(item) = content.item(item_id) else {
        return;
    };
    if item.use_hook.is_empty() || !state.remove_item(item_id) {
        return;
    }
    let player_id = content.settings.combat.player_actor_id.clone();
    if let Err(error) = crate::engine::hooks::apply_narrating_world_hook_effects(
        state,
        content,
        &item.use_hook,
        serde_json::json!({
            "actor_id": player_id,
            "actor_name": super::super::command_effects::actor_display_name(state, content, &player_id),
            "item_id": item.id,
            "item_label": item.label,
        }),
        lines,
    ) {
        eprintln!("[cinder] hook warning ({}): {error}", item.use_hook);
    }
    let specific_key = format!("item.{item_id}.used");
    let line = if content.message(&specific_key).is_some() {
        content.render_message(&specific_key, &[("item", item.label.as_str())])
    } else {
        content.render_message("item.used", &[("item", item.label.as_str())])
    };
    if let Some(line) = line {
        lines.narration(line);
    }
    lines.extend_narration(advance_objective_for_signal(
        state,
        content,
        &format!("item_used:{item_id}"),
    ));
    lines.extend_narration(advance_objective_for_signal(
        state,
        content,
        &format!("item_consumed:{item_id}"),
    ));
    lines.extend_narration(advance_objective_for_signal(
        state,
        content,
        "item_consumed",
    ));
}

/// `item.<id>.<generic key>` to override the message; an empty override
/// suppresses the line entirely. Returns the rendered text together with the
/// voice of whichever key (specific or generic) supplies it.
fn render_acquired_line(
    content: &ContentPack,
    item_id: &str,
    generic_key: &str,
    label: &str,
) -> Option<(String, PackMessageVoice)> {
    let specific_key = format!(
        "item.{item_id}.{}",
        generic_key.strip_prefix("item.").unwrap_or(generic_key)
    );
    if content.message(&specific_key).is_some() {
        let text = content.render_message(&specific_key, &[("label", label)])?;
        if text.trim().is_empty() {
            return None;
        }
        return Some((text, content.message_voice(&specific_key)));
    }
    Some((
        content.render_message(generic_key, &[("label", label)])?,
        content.message_voice(generic_key),
    ))
}

pub(crate) fn handle_item_consumed(
    state: &mut WorldState,
    content: &ContentPack,
    item_id: &str,
    storage: ItemStorageTarget,
    consumer_id: Option<&str>,
    consumer_name: Option<&str>,
    lines: &mut NarrativeLines,
) {
    let label = content.item_label(item_id);
    let room_id = state.current_room_id.clone();
    if state.remove_item_from_storage(item_id, storage, &room_id) {
        if consumer_id == Some(content.settings.combat.player_actor_id.as_str()) {
            push_message(lines, content, "item.consumed_player", &[("label", label)]);
        } else if let Some(consumer_name) = consumer_name {
            push_message(
                lines,
                content,
                "item.consumed_actor",
                &[("consumer_name", consumer_name), ("label", label)],
            );
        } else {
            push_message(lines, content, "item.consumed_use", &[("label", label)]);
        }
    }
}

pub(crate) fn handle_item_observed(
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
