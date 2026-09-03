use crate::engine::reducer::command_effects::trigger_surrounded_hooks;
use crate::content::types::{ContentPack, ItemStorageTarget};
use crate::engine::narrative::NarrativeLines;
use crate::engine::state::WorldState;
pub(crate) fn handle_item_acquired(
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
            trigger_surrounded_hooks(state, content, item_id, lines);
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
    let label = content
        .item(item_id)
        .map(|i| i.label.as_str())
        .unwrap_or(item_id);
    if state.remove_item_from_storage(item_id, ItemStorageTarget::CurrentRoom, &room_id) {
        state.add_item(item_id);
        if let Some(line) = content.render_message("item.taken", &[("label", label)]) {
            lines.narration(line);
        }
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
    let label = content
        .item(item_id)
        .map(|i| i.label.as_str())
        .unwrap_or(item_id);
    if state.remove_item(item_id) {
        state.add_item_to_storage(item_id, ItemStorageTarget::CurrentRoom, &room_id);
        if let Some(line) = content.render_message("item.dropped", &[("label", label)]) {
            lines.narration(line);
        }
        trigger_surrounded_hooks(state, content, item_id, lines);
    }
}

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

pub(crate) fn handle_item_consumed(
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