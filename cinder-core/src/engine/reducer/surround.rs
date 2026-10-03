use crate::content::types::{ContentPack, ItemStorageTarget, SurroundRule};
use crate::engine::hook_ids;
use crate::engine::hooks::{apply_narrating_world_hook_effects, evaluate_hook_effects};
use crate::engine::narrative::NarrativeLines;
use crate::engine::state::{ActorStance, WorldState};
use serde_json::json;

use std::collections::HashSet;

use super::handlers::push_message;
use crate::engine::reducer::combat::VEC_EMPTY_TAGS;

/// Whether the pack's surround gate lets an encircled actor convert. With no
/// rule (`SurroundRule::None`) every candidate converts and the pack gates via
/// its own hook conditions. With `Resistance` the player must out-score the
/// target against the configured stat: `player_stat + player_level >=
/// target_stat + 2*target_level`. The player stat is the *effective* value so
/// equipped bonuses count.
fn surround_rule_passes(state: &WorldState, content: &ContentPack, target_actor_id: &str) -> bool {
    let SurroundRule::Resistance { stat } = &content.settings.surround_rule else {
        return true;
    };
    let player_id = &content.settings.combat.player_actor_id;
    let player_stat = state.effective_actor_stat(content, player_id, stat).max(0) as u32;
    let player_level = state.actor_level(player_id);
    let target_stat = state.actor_stat(target_actor_id, stat).max(0) as u32;
    let target_level = state.actor_level(target_actor_id);
    player_stat + player_level >= target_stat + 2 * target_level
}

/// Evaluates encirclement for a candidate room. If all neighboring rooms contain
/// the triggering `item_id`, every living, non-allied mob in `room_id` is evaluated
/// individually against the surround rule. On success with single-use tokens
/// (`consumed_on_surround_conversion`), or on refusal, all encircling tokens in
/// the neighboring rooms are consumed.
fn try_surround_room(
    state: &mut WorldState,
    content: &ContentPack,
    item_id: &str,
    source_room_id: &str,
    room_id: &str,
    lines: &mut NarrativeLines,
) -> bool {
    let neighbors = content.adjacent_room_ids(room_id);
    if neighbors.is_empty() {
        return false;
    }
    if !neighbors
        .iter()
        .all(|n| state.has_item_in_storage(item_id, ItemStorageTarget::CurrentRoom, n))
    {
        return false;
    }

    let player_id = &content.settings.combat.player_actor_id;
    let candidate_actors: Vec<(String, String)> = state
        .onstage_actors(content)
        .filter(|actor| {
            if actor.id == *player_id {
                return false;
            }
            if state.actor_room_id(&actor.id, &actor.room_id) != room_id {
                return false;
            }
            if state.actor_is_defeated(&actor.id, &content.settings.combat.health_stat_id) {
                return false;
            }
            let relationship = state.relationship(&actor.id);
            if relationship.stance == ActorStance::Allied || relationship.follows_player {
                return false;
            }
            true
        })
        .map(|actor| (actor.id.clone(), actor.room_id.clone()))
        .collect();

    let mut converted_count = 0;
    let mut refused_count = 0;

    for (actor_id, actor_home_room_id) in candidate_actors {
        let actor_name = state
            .actor_display_name(content, &actor_id)
            .unwrap_or(&actor_id)
            .to_string();
        let input = json!({
            "actor_id": actor_id,
            "actor_name": actor_name,
            "room_id": room_id,
            "item_id": item_id,
            "tags": content
                .actor(&actor_id)
                .map(|actor| &actor.tags)
                .unwrap_or(&VEC_EMPTY_TAGS),
            "story_vars": state.story_vars.to_map(),
        });
        let effects = match evaluate_hook_effects::<serde_json::Value>(
            content,
            hook_ids::ACTOR_SURROUNDED,
            input.clone(),
        ) {
            Ok(effects) => effects,
            Err(error) => {
                eprintln!("[cinder] hook warning (actor.surrounded): {error}");
                continue;
            }
        };
        if effects.is_empty() {
            continue;
        }

        if !surround_rule_passes(state, content, &actor_id) {
            push_message(lines, content, "surround.refused", &[]);
            refused_count += 1;
            continue;
        }

        apply_narrating_world_hook_effects(
            state,
            content,
            hook_ids::ACTOR_SURROUNDED,
            input,
            lines,
        )
        .unwrap_or_else(|error| eprintln!("[cinder] hook warning (actor.surrounded): {error}"));

        let relationship = state.relationship(&actor_id);
        let converted = relationship.stance == ActorStance::Allied || relationship.follows_player;
        if converted {
            state.initialize_party_order(content, &actor_id);
            let party_room_id = state.current_room_id.clone();
            if state.actor_room_id(&actor_id, &actor_home_room_id) != party_room_id {
                state.mark_actor_room_visited(&actor_id, &party_room_id);
                state
                    .actor_room_overrides
                    .insert(actor_id.clone(), party_room_id);
            }
            converted_count += 1;
        }
    }

    let consumed_on_conversion = content
        .item(item_id)
        .is_some_and(|item| item.consumed_on_surround_conversion);

    if (converted_count > 0 && consumed_on_conversion) || refused_count > 0 {
        // Successful conversion with single-use tokens, or a refusal, spends the ring:
        // encircling items fade from all neighboring rooms.
        for neighbor in &neighbors {
            state.remove_items_from_room(neighbor, item_id);
        }
        state.remove_items_from_room(source_room_id, item_id);
        return true;
    }

    false
}

/// Fires the content-authored `actor.surrounded` hook for each living,
/// non-allied actor in any room whose neighboring rooms all contain the triggering item.
/// `source_room_id` is where the triggering item was just placed; a conversion
/// caused by a single-use token (an item with `consumed_on_surround_conversion`)
/// spends all encircling tokens from the neighboring rooms.
pub(super) fn trigger_surrounded_hooks(
    state: &mut WorldState,
    content: &ContentPack,
    item_id: &str,
    source_room_id: &str,
    lines: &mut NarrativeLines,
) {
    let player_id = &content.settings.combat.player_actor_id;
    let onstage_actors: Vec<(String, String)> = state
        .onstage_actors(content)
        .map(|actor| (actor.id.clone(), actor.room_id.clone()))
        .collect();

    let mut candidate_rooms = Vec::new();
    let mut seen_rooms = HashSet::new();
    for (actor_id, actor_home_room_id) in &onstage_actors {
        if actor_id == player_id {
            continue;
        }
        let relationship = state.relationship(actor_id);
        if relationship.stance == ActorStance::Allied || relationship.follows_player {
            continue;
        }
        if state.actor_is_defeated(actor_id, &content.settings.combat.health_stat_id) {
            continue;
        }
        let room_id = state.actor_room_id(actor_id, actor_home_room_id);
        if seen_rooms.insert(room_id.to_string()) {
            candidate_rooms.push(room_id.to_string());
        }
    }

    for room_id in candidate_rooms {
        let stop = try_surround_room(state, content, item_id, source_room_id, &room_id, lines);
        if stop {
            break;
        }
    }
}

/// Fires the content-authored placement hook when an item is placed in a room
/// (e.g. via `trace` or `drop`), and consumes it if `consumed_on_placement` is true.
pub(super) fn trigger_placement_hooks(
    state: &mut WorldState,
    content: &ContentPack,
    item_id: &str,
    placer_actor_id: &str,
    room_id: &str,
    lines: &mut NarrativeLines,
) {
    let Some(item) = content.item(item_id) else {
        return;
    };
    if !item.placement_hook.is_empty() {
        let input = json!({
            "item_id": item_id,
            "actor_id": placer_actor_id,
            "room_id": room_id,
        });
        if let Err(error) =
            apply_narrating_world_hook_effects(state, content, &item.placement_hook, input, lines)
        {
            eprintln!(
                "[cinder] hook warning (placement_hook: {}): {error}",
                item.placement_hook
            );
        }
    }
    if item.consumed_on_placement {
        state.remove_items_from_room(room_id, item_id);
    }
}
