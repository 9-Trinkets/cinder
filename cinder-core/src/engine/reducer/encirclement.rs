use crate::content::types::{CharmRule, ContentPack, ItemStorageTarget};
use crate::engine::hook_ids;
use crate::engine::hooks::apply_narrating_world_hook_effects;
use crate::engine::narrative::NarrativeLines;
use crate::engine::state::{ActorStance, WorldState};
use serde_json::json;

use super::combat::actor_display_name;
use super::handlers::push_feedback_line;

/// Cold text narrated when the pack's charm rule refuses an encircled actor,
/// keeping the refusal diegetic instead of silently failing.
fn charm_refused_line() -> String {
    "THE RING DOES NOT HOLD.".to_string()
}

/// Whether the pack's charm rule lets an encircled actor convert. With no
/// rule (`CharmRule::None`) every candidate converts and the pack gates via
/// its own hook conditions. With `IntAndLevel` the player must out-score the
/// target: `player_int + player_level >= target_int + 2*target_level`. The
/// player int is the *effective* value so equipped bonuses (e.g. a ring) count.
fn charm_rule_passes(
    state: &WorldState,
    content: &ContentPack,
    target_actor_id: &str,
) -> bool {
    let CharmRule::IntAndLevel = content.settings.charm_rule else {
        return true;
    };
    let player_id = &content.settings.combat.player_actor_id;
    let player_int = state
        .effective_actor_stat(content, player_id, "intelligence")
        .max(0) as u32;
    let player_level = state.actor_level(player_id);
    let target_int = state.actor_stat(target_actor_id, "intelligence").max(0) as u32;
    let target_level = state.actor_level(target_actor_id);
    player_int + player_level >= target_int + 2 * target_level
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
        if !charm_rule_passes(state, content, &actor.id) {
            push_feedback_line(lines, content, charm_refused_line(), |lines, text| {
                lines.system(text);
            });
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
        if converted
            && content
                .item(item_id)
                .is_some_and(|item| item.consumed_on_surround_conversion)
        {
            // The conversion spent the single-use token (e.g. a charm sigil):
            // it fades from the room it was just placed in and this placement
            // converts nothing else.
            state.remove_items_from_room(source_room_id, item_id);
            break;
        }
    }
}
