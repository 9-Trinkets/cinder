use cinder_core::content::types::ContentPack;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::{ActorStance, WorldState};

use super::{EquippedItem, InventoryItem, PanelOptionData, PartyMember, PlayerStatus, StatValue};
use std::collections::BTreeMap;

/// The player's progress toward the next level. XP/level are per-actor now:
/// `actor_xp` holds progress toward the *next* level (reset past each
/// threshold on level-up), so `xp_max` is the threshold at the player's own
/// current level. A missing threshold means the actor is at max level
/// (xp_max == 0).
pub(super) fn xp_progress(state: &WorldState, content: &ContentPack) -> (u32, u32, u32) {
    let player_id = &content.settings.combat.player_actor_id;
    let level = state.actor_level(player_id);
    let xp = state.actor_xp(player_id);
    let xp_max = content
        .xp_required_for_next_level(player_id, level)
        .unwrap_or(0);
    (level, xp, xp_max)
}

pub(super) fn build_player_status(state: &WorldState, content: &ContentPack) -> PlayerStatus {
    let player_id = &content.settings.combat.player_actor_id;
    let health_stat = &content.settings.combat.health_stat_id;
    let hp = state
        .effective_actor_stat(content, player_id, health_stat)
        .max(0) as u32;
    let hp_max = state
        .effective_actor_stat_maximum(content, player_id, health_stat)
        .max(0) as u32;
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
    let follower_ids = living_follower_ids(state, content);
    let mut label_totals = BTreeMap::<String, usize>::new();
    for actor_id in &follower_ids {
        let label = runtime
            .actor_display_name(actor_id)
            .ok()
            .flatten()
            .unwrap_or_else(|| actor_id.clone());
        *label_totals.entry(label).or_default() += 1;
    }
    let mut label_indexes = BTreeMap::<String, usize>::new();
    let health_stat = &content.settings.combat.health_stat_id;
    let mut members = follower_ids
        .into_iter()
        .map(|actor_id| {
            let base_label = runtime
                .actor_display_name(&actor_id)
                .ok()
                .flatten()
                .unwrap_or_else(|| actor_id.clone());
            let index = label_indexes.entry(base_label.clone()).or_default();
            *index += 1;
            let label = if label_totals.get(&base_label).copied().unwrap_or(0) > 1 {
                format!("{base_label} {index}")
            } else {
                base_label
            };
            let hp = state.actor_stat(&actor_id, health_stat).max(0) as u32;
            let hp_max = state
                .actor_stat_maximum(content, &actor_id, health_stat)
                .max(1) as u32;
            let order = state.party_order(content, &actor_id).unwrap_or_default();
            let equipped_items =
                build_equipped_items_from_map(state.actor_equipment(&actor_id), content);
            let mut inventory = state
                .actor_inventory(&actor_id)
                .into_iter()
                .map(|(id, count)| {
                    let label = content.item_label(&id).to_string();
                    InventoryItem {
                        label,
                        count,
                        id: Some(id),
                    }
                })
                .collect::<Vec<_>>();
            inventory.sort_by(|a, b| a.label.cmp(&b.label));
            let in_room = state.actor_is_in_room(content, &actor_id, &state.current_room_id);
            PartyMember {
                id: actor_id.clone(),
                label,
                level: state.actor_level(&actor_id),
                hp,
                hp_max,
                order,
                order_panel: format!("party-order:{actor_id}"),
                inventory,
                equipped_items,
                in_room,
            }
        })
        .collect::<Vec<_>>();
    members.sort_by(|a, b| a.label.cmp(&b.label).then_with(|| a.id.cmp(&b.id)));
    members
}

pub(super) fn build_party_order_panels(
    content: &ContentPack,
    members: &[PartyMember],
) -> BTreeMap<String, Vec<PanelOptionData>> {
    let directives = content.settings.party.directives();
    members
        .iter()
        .map(|member| {
            let options = directives
                .iter()
                .map(|directive| {
                    let title = content
                        .render_message(&format!("party.directive.{directive}.title"), &[])
                        .unwrap_or_else(|| {
                            let mut chars = directive.chars();
                            match chars.next() {
                                Some(first) => {
                                    first.to_uppercase().collect::<String>() + chars.as_str()
                                }
                                None => directive.clone(),
                            }
                        });
                    let subtitle = content
                        .render_message(&format!("party.directive.{directive}.subtitle"), &[]);
                    let selected = member.order == *directive;
                    PanelOptionData {
                        id: directive.clone(),
                        title,
                        subtitle,
                        command: Some(format!("order {} {}", member.id, directive)),
                        disabled: selected,
                        selected,
                    }
                })
                .collect();
            (member.order_panel.clone(), options)
        })
        .collect()
}

fn living_follower_ids(state: &WorldState, content: &ContentPack) -> Vec<String> {
    state
        .relationships
        .iter()
        .filter(|(actor_id, relationship)| {
            (relationship.follows_player || relationship.stance == ActorStance::Allied)
                && actor_id.as_str() != content.settings.combat.player_actor_id
                && !content.actor_is_offstage(actor_id)
                && !state.actor_is_defeated(actor_id, &content.settings.combat.health_stat_id)
        })
        .map(|(actor_id, _)| actor_id.clone())
        .collect()
}

pub(super) fn build_equipped_items_from_map(
    equipment: &std::collections::BTreeMap<String, String>,
    content: &ContentPack,
) -> Vec<EquippedItem> {
    // A multi-slot item (e.g. a two-hand weapon) is listed once, with its
    // slots joined in the pack's declared order.
    let declared = &content.settings.equipment_slots;
    let slot_rank = |slot: &str| {
        declared
            .iter()
            .position(|declared_slot| declared_slot == slot)
            .unwrap_or(usize::MAX)
    };
    let mut slot_by_item: std::collections::BTreeMap<&str, Vec<&str>> = std::collections::BTreeMap::new();
    for (slot, item_id) in equipment {
        slot_by_item.entry(item_id).or_default().push(slot);
    }
    let mut items: Vec<EquippedItem> = slot_by_item
        .into_iter()
        .map(|(item_id, mut slots)| {
            slots.sort_by_key(|slot| slot_rank(slot));
            let label = content.item_label(item_id).to_string();
            EquippedItem {
                slot: slots.join("+"),
                label,
                id: Some(item_id.to_string()),
            }
        })
        .collect();
    items.sort_by_key(|item| {
        item.slot
            .split('+')
            .map(slot_rank)
            .min()
            .unwrap_or(usize::MAX)
    });
    items
}

pub(super) fn build_equipped_items(state: &WorldState, content: &ContentPack) -> Vec<EquippedItem> {
    build_equipped_items_from_map(&state.equipment, content)
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
            let label = content.item_label(&id).to_string();
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
        .filter(|(item_id, _)| content.item(item_id).is_none_or(|item| item.is_takeable()))
        .map(|(item_id, count)| {
            let label = content.item_label(&item_id).to_string();
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
    use cinder_core::content::types::LevelDefinition;
    use cinder_core::engine::test_fixtures::minimal_test_pack;

    #[test]
    fn member_hp_max_includes_level_up_growth() {
        let mut content = minimal_test_pack();
        content.settings.combat.health_stat_id = "stamina".to_string();
        content.levels.default = vec![LevelDefinition {
            exp_required: 0,
            stat_changes: std::collections::BTreeMap::from([("stamina".to_string(), 4)]),
            ..LevelDefinition::default()
        }];
        let mut state = WorldState::new(&content);
        state.actor_level.insert("blair".to_string(), 2);
        state.adjust_actor_stat(&content, "blair", "stamina", 4).unwrap();
        state.adjust_actor_stat(&content, "blair", "stamina", -2).unwrap();
        state.set_follows_player("blair", true);
        let runtime = CinderRuntime::new(content.clone(), false).unwrap();

        let members = build_party_members(&runtime, &state, &content);
        let blair = members.into_iter().find(|member| member.id == "blair").unwrap();

        assert_eq!(blair.level, 2);
        assert_eq!(blair.hp, 8);
        assert_eq!(blair.hp_max, 10);
    }

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
            .adjust_actor_stat(&content, &defeated_id, "stamina", -100)
            .unwrap();

        assert_eq!(living_follower_ids(&state, &content), vec![living_id]);
    }

    #[test]
    fn allied_actors_not_following_player_are_included_in_living_followers() {
        let mut content = minimal_test_pack();
        let ally_id = content.actors[0].id.clone();
        let mut state = WorldState::new(&content);
        state.set_stance(&ally_id, ActorStance::Allied);
        state.set_follows_player(&ally_id, false);

        assert_eq!(living_follower_ids(&state, &content), vec![ally_id]);
    }

    #[test]
    fn party_order_panels_mark_the_current_order_and_use_actor_ids() {
        let mut content = minimal_test_pack();
        content.settings.party.initial_orders = std::collections::BTreeMap::from([
            ("guide-a".to_string(), "follow".to_string()),
            ("guide-b".to_string(), "guard".to_string()),
        ]);
        let members = vec![PartyMember {
            id: "dark-golem-2".to_string(),
            label: "dark golem 2".to_string(),
            level: 1,
            hp: 8,
            hp_max: 8,
            order: "guard".to_string(),
            order_panel: "party-order:dark-golem-2".to_string(),
            inventory: Vec::new(),
            equipped_items: Vec::new(),
            in_room: true,
        }];

        let panels = build_party_order_panels(&content, &members);
        let options = &panels["party-order:dark-golem-2"];

        assert_eq!(options[0].id, "follow");
        assert_eq!(
            options[0].command.as_deref(),
            Some("order dark-golem-2 follow")
        );
        assert_eq!(options[1].id, "guard");
        assert!(options[1].selected);
        assert!(options[1].disabled);
    }

    #[test]
    fn build_current_room_items_excludes_trace_marks() {
        let mut content = minimal_test_pack();
        content.items.extend([
            cinder_core::content::types::ItemDefinition {
                id: "sigil".to_string(),
                label: "sigil".to_string(),
                trace_mark: true,
                ..Default::default()
            },
            cinder_core::content::types::ItemDefinition {
                id: "scroll".to_string(),
                label: "scroll".to_string(),
                ..Default::default()
            },
        ]);
        let mut state = WorldState::new(&content);
        let room_id = state.current_room_id.clone();
        state.add_item_to_storage("sigil", cinder_core::content::types::ItemStorageTarget::CurrentRoom, &room_id);
        state.add_item_to_storage("scroll", cinder_core::content::types::ItemStorageTarget::CurrentRoom, &room_id);

        let items = build_current_room_items(&content, &state, &room_id);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id.as_deref(), Some("scroll"));
    }

    #[test]
    fn build_party_members_includes_inventory_and_equipment() {
        let mut content = minimal_test_pack();
        content.settings.equipment_slots = std::collections::BTreeSet::from(["weapon".to_string(), "ring".to_string()]);
        content.items.extend([
            cinder_core::content::types::ItemDefinition {
                id: "iron-sword".to_string(),
                label: "iron sword".to_string(),
                equip_slots: vec!["weapon".to_string()],
                ..Default::default()
            },
            cinder_core::content::types::ItemDefinition {
                id: "herb".to_string(),
                label: "green herb".to_string(),
                ..Default::default()
            },
        ]);
        let mut state = WorldState::new(&content);
        let follower_id = "companion-1";
        state.set_follows_player(follower_id, true);
        state.actor_add_item(follower_id, "herb");
        state.actor_add_item(follower_id, "herb");
        if let Some(equip) = state.actor_equipment.get_mut(follower_id) {
            equip.insert("weapon".to_string(), "iron-sword".to_string());
        } else {
            state.actor_equipment.insert(
                follower_id.to_string(),
                std::collections::BTreeMap::from([("weapon".to_string(), "iron-sword".to_string())]),
            );
        }

        let runtime = CinderRuntime::new(content.clone(), false).unwrap();
        let members = build_party_members(&runtime, &state, &content);
        let companion = members.into_iter().find(|m| m.id == follower_id).unwrap();
        assert_eq!(companion.inventory.len(), 1);
        assert_eq!(companion.inventory[0].label, "green herb");
        assert_eq!(companion.inventory[0].count, 2);
        assert_eq!(companion.inventory[0].id.as_deref(), Some("herb"));
        assert_eq!(companion.equipped_items.len(), 1);
        assert_eq!(companion.equipped_items[0].slot, "weapon");
        assert_eq!(companion.equipped_items[0].label, "iron sword");
        assert_eq!(companion.equipped_items[0].id.as_deref(), Some("iron-sword"));
    }
}
