use crate::content::types::{ContentPack, ItemStorageTarget};
use crate::engine::hook_ids;
use crate::engine::hooks::apply_narrating_world_hook_effects;
use crate::engine::narrative::NarrativeLines;
use crate::engine::state::{ActorStance, WorldState};
use serde_json::json;

use super::combat::actor_display_name;

/// Fires the content-authored `actor.surrounded` hook for each living,
/// non-allied actor whose neighboring rooms all contain the triggering item.
pub(super) fn trigger_surrounded_hooks(
    state: &mut WorldState,
    content: &ContentPack,
    item_id: &str,
    lines: &mut NarrativeLines,
) {
    let player_id = &content.settings.combat.player_actor_id;
    for actor in &content.actors {
        if actor.id == *player_id {
            continue;
        }
        let relationship = state.relationship(&actor.id);
        if relationship.stance == ActorStance::Allied || relationship.follows_player {
            continue;
        }
        if state.actor_is_defeated(&actor.id, &content.settings.combat.health_stat_id) {
            continue;
        }
        let room_id = state.actor_room_id(&actor.id, &actor.room_id).to_string();
        let neighbors = content.adjacent_room_ids(&room_id);
        if neighbors.is_empty() {
            continue;
        }
        if !neighbors
            .iter()
            .all(|n| state.has_item_in_storage(item_id, ItemStorageTarget::CurrentRoom, n))
        {
            continue;
        }
        let actor_name = actor_display_name(content, &actor.id);
        apply_narrating_world_hook_effects(
            state,
            content,
            hook_ids::ACTOR_SURROUNDED,
            json!({
                "actor_id": actor.id,
                "actor_name": actor_name,
                "room_id": room_id,
                "item_id": item_id,
            }),
            lines,
        )
        .unwrap_or_else(|error| eprintln!("[cinder] hook warning (actor.surrounded): {error}"));
    }
}
