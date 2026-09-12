use crate::content::types::{ContentPack, ItemStorageTarget, SurroundRule};
use crate::engine::hook_ids;
use crate::engine::hooks::apply_narrating_world_hook_effects;
use crate::engine::narrative::NarrativeLines;
use crate::engine::state::{ActorStance, WorldState};
use serde_json::json;

use super::combat::actor_display_name;
use super::handlers::push_message;

/// Whether the pack's surround gate lets an encircled actor convert. With no
/// rule (`SurroundRule::None`) every candidate converts and the pack gates via
/// its own hook conditions. With `Resistance` the player must out-score the
/// target against the configured stat: `player_stat + player_level >=
/// target_stat + 2*target_level`. The player stat is the *effective* value so
/// equipped bonuses count.
fn surround_rule_passes(
    state: &WorldState,
    content: &ContentPack,
    target_actor_id: &str,
) -> bool {
    let SurroundRule::Resistance { stat } = &content.settings.surround_rule else {
        return true;
    };
    let player_id = &content.settings.combat.player_actor_id;
    let player_stat = state
        .effective_actor_stat(content, player_id, stat)
        .max(0) as u32;
    let player_level = state.actor_level(player_id);
    let target_stat = state.actor_stat(target_actor_id, stat).max(0) as u32;
    let target_level = state.actor_level(target_actor_id);
    player_stat + player_level >= target_stat + 2 * target_level
}

/// Fires the content-authored `actor.surrounded` hook for each living,
/// non-allied actor whose neighboring rooms all contain the triggering item.
/// `source_room_id` is where the triggering item was just placed; a conversion
/// caused by a single-use token (an item with `consumed_on_surround_conversion`)
/// spends that token from the room.
pub(super) fn trigger_surrounded_hooks(
    state: &mut WorldState,
    content: &ContentPack,
    item_id: &str,
    source_room_id: &str,
    lines: &mut NarrativeLines,
) {
    let player_id = &content.settings.combat.player_actor_id;
    for actor in content.onstage_actors() {
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
        if !surround_rule_passes(state, content, &actor.id) {
            push_message(lines, content, "surround.refused", &[]);
            // A refused conversion spends the ring: the encircling items fade
            // from this target's neighbors, so the ring must be rebuilt before
            // the attempt can repeat.
            for neighbor in &neighbors {
                state.remove_items_from_room(neighbor, item_id);
            }
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
        let relationship = state.relationship(&actor.id);
        let converted = relationship.stance == ActorStance::Allied || relationship.follows_player;
        if converted {
            // The closed ring draws the convert into the room the ring was
            // drawn in, so a charmed mob joins the party immediately instead
            // of staying in the room where it was encircled.
            let party_room_id = state.current_room_id.clone();
            if state.actor_room_id(&actor.id, &actor.room_id) != party_room_id {
                state.mark_actor_room_visited(&actor.id, &party_room_id);
                state
                    .actor_room_overrides
                    .insert(actor.id.clone(), party_room_id);
            }
            if content
                .item(item_id)
                .is_some_and(|item| item.consumed_on_surround_conversion)
            {
                // The conversion spent the single-use token: it fades from the room
                // it was just placed in and this placement converts nothing else.
                state.remove_items_from_room(source_room_id, item_id);
                break;
            }
        }
    }
}