use cinder_core::content::types::ContentPack;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::WorldState;

use super::{EquippedItem, InventoryItem, PartyMember, PlayerStatus, StatValue};

/// The player's progress toward the next level. XP/level are per-actor now:
/// `actor_xp` holds progress toward the *next* level (reset past each
/// threshold on level-up), so `xp_max` is the threshold at the player's own
/// current level. A missing threshold means the actor is at max level
/// (xp_max == 0).
pub(super) fn xp_progress(
    state: &WorldState,
    content: &ContentPack,
) -> (u32, u32, u32) {
    let player_id = &content.settings.combat.player_actor_id;
    let level = state.actor_level(player_id);
    let xp = state.actor_xp(player_id);
    let xp_max = content
        .xp_required_for_next_level(player_id, level)
        .unwrap_or(0);
    (level, xp, xp_max)
}

pub(super) fn build_player_status(
    state: &WorldState,
    content: &ContentPack,
) -> PlayerStatus {
    let player_id = &content.settings.combat.player_actor_id;
    let health_stat = &content.settings.combat.health_stat_id;
    let hp = state
        .effective_actor_stat(content, player_id, health_stat)
        .max(0) as u32;
    let hp_max = content
        .stats
        .actor
        .get(health_stat)
        .and_then(|stat| stat.max)
        .map(|max| max.max(0) as u32)
        .unwrap_or(hp);
    let mut stats = content
        .stats
        .actor
        .iter()
        .filter(|(stat_id, _)| *stat_id != health_stat)
        .map(|(stat_id, _)| StatValue {
            id: stat_id.clone(),
            value: state.effective_actor_stat(content, player_id, stat_id),
        })
        .collect::<Vec<_>>();
    stats.sort_by(|a, b| a.id.cmp(&b.id));
    let (level, xp, xp_max) = xp_progress(state, content);
    PlayerStatus {
        hp,
        hp_max,
        stats,
        level,
        xp,
        xp_max,
    }
}

pub(super) fn build_party_members(
    runtime: &CinderRuntime,
    state: &WorldState,
    content: &ContentPack,
) -> Vec<PartyMember> {
    let mut members: Vec<PartyMember> = Vec::new();
    for actor_id in living_follower_ids(state, content) {
        let label = runtime
            .actor_display_name(&actor_id)
            .ok()
            .flatten()
            .unwrap_or_else(|| actor_id.clone());
        let level = state.actor_level(&actor_id);
        if let Some(member) = members.iter_mut().find(|m| m.label == label) {
            member.count += 1;
        } else {
            members.push(PartyMember {
                label,
                count: 1,
                level,
            });
        }
    }
    members.sort_by(|a, b| a.label.cmp(&b.label));
    members
}

fn living_follower_ids(state: &WorldState, content: &ContentPack) -> Vec<String> {
    state
        .relationships
        .iter()
        .filter(|(actor_id, relationship)| {
            relationship.follows_player
                && actor_id.as_str() != content.settings.combat.player_actor_id
                && !content.actor_is_offstage(actor_id)
                && !state.actor_is_defeated(actor_id, &content.settings.combat.health_stat_id)
        })
        .map(|(actor_id, _)| actor_id.clone())
        .collect()
}

pub(super) fn build_equipped_items(
    state: &WorldState,
    content: &ContentPack,
) -> Vec<EquippedItem> {
    let mut items: Vec<EquippedItem> = state
        .equipment
        .iter()
        .map(|(slot, item_id)| {
            let label = content
                .item(item_id)
                .map(|item| item.label.clone())
                .unwrap_or_else(|| item_id.clone());
            EquippedItem {
                slot: slot.clone(),
                label,
            }
        })
        .collect();
    items.sort_by(|left, right| left.slot.cmp(&right.slot));
    items
}

pub(super) fn build_inventory(
    runtime: &CinderRuntime,
    content: &ContentPack,
) -> Vec<InventoryItem> {
    let mut inventory = runtime
        .inventory_items()
        .unwrap_or_default()
        .into_iter()
        .map(|(id, count)| {
            let label = content
                .item(&id)
                .map(|item| item.label.clone())
                .unwrap_or_else(|| id.clone());
            InventoryItem {
                label,
                count,
                id: None,
            }
        })
        .collect::<Vec<_>>();
    inventory.sort_by(|left, right| left.label.cmp(&right.label));
    inventory
}

pub(super) fn build_current_room_items(
    content: &ContentPack,
    state: &WorldState,
    current_room_id: &str,
) -> Vec<InventoryItem> {
    state
        .loose_room_items(current_room_id)
        .into_iter()
        .map(|(item_id, count)| {
            let label = content
                .item(&item_id)
                .map(|item| item.label.clone())
                .unwrap_or_else(|| item_id.clone());
            InventoryItem {
                label,
                count,
                id: Some(item_id.clone()),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cinder_core::engine::test_fixtures::minimal_test_pack;

    #[test]
    fn defeated_followers_are_excluded_from_the_party() {
        let mut content = minimal_test_pack();
        content.settings.combat.health_stat_id = "stamina".to_string();
        let living_id = content.actors[0].id.clone();
        let defeated_id = content.actors[1].id.clone();
        let mut state = WorldState::new(&content);
        state.set_follows_player(&living_id, true);
        state.set_follows_player(&defeated_id, true);
        state
            .adjust_actor_stat(&defeated_id, "stamina", -100)
            .unwrap();

        assert_eq!(living_follower_ids(&state, &content), vec![living_id]);
    }
}
