use cinder_core::content::types::ContentPack;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::{ActorStance, WorldState};

use super::{
    EquippedItem, InventoryItem, PanelOptionData, PartyMember, PartySkill, PlayerStatus, StatValue,
};
use std::collections::{BTreeMap, BTreeSet};

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
    let has_mp = content.stats.actor.contains_key("mp");
    let (mp, mp_max) = if has_mp {
        (
            Some(state.effective_actor_stat(content, player_id, "mp").max(0) as u32),
            Some(
                state
                    .effective_actor_stat_maximum(content, player_id, "mp")
                    .max(0) as u32,
            ),
        )
    } else {
        (None, None)
    };
    let mut stats = content
        .stats
        .actor
        .iter()
        .filter(|(stat_id, _)| *stat_id != health_stat && (stat_id.as_str() != "mp" || !has_mp))
        .map(|(stat_id, _)| StatValue {
            id: stat_id.clone(),
            value: state.effective_actor_stat(content, player_id, stat_id),
        })
        .collect::<Vec<_>>();
    stats.sort_by(|a, b| a.id.cmp(&b.id));
    let (level, xp, xp_max) = xp_progress(state, content);
    let attack = if !content.settings.combat.attack_stat_id.is_empty() {
        Some(state.actor_combat_attack(content, player_id))
    } else {
        None
    };
    let defense = if !content.settings.combat.defense_stat_id.is_empty() {
        Some(state.actor_combat_defense(content, player_id))
    } else {
        None
    };
    PlayerStatus {
        hp,
        hp_max,
        mp,
        mp_max,
        attack,
        defense,
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
                        usable: false,
                    }
                })
                .collect::<Vec<_>>();
            inventory.sort_by(|a, b| a.label.cmp(&b.label));
            let in_room = state.actor_is_in_room(content, &actor_id, &state.current_room_id);
            let mut skills_set = state
                .actor_skills
                .get(&actor_id)
                .cloned()
                .unwrap_or_default();
            if cinder_core::engine::reducer::combat::offscreen_comms::actor_can_use_comms(
                state, &actor_id,
            ) {
                skills_set.insert("comms".to_string());
            }
            let skills = skills_set
                .into_iter()
                .filter_map(|skill_id| {
                    content.skill(&skill_id).map(|skill| PartySkill {
                        id: skill.id.clone(),
                        label: skill.label.clone(),
                        kind: skill.kind,
                    })
                })
                .collect();
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
                skills,
                in_room,
            }
        })
        .collect::<Vec<_>>();
    members.sort_by(|a, b| a.label.cmp(&b.label).then_with(|| a.id.cmp(&b.id)));
    members
}

struct HuntTargetInfo {
    id: String,
    title: String,
    subtitle: Option<String>,
}

fn build_hunt_targets_for_floor(content: &ContentPack, state: &WorldState) -> Vec<HuntTargetInfo> {
    let floor_rooms = content.reachable_room_ids(&state.current_room_id);
    let player_id = &content.settings.combat.player_actor_id;
    let health_stat_id = if content.settings.combat.health_stat_id.is_empty() {
        "hp"
    } else {
        &content.settings.combat.health_stat_id
    };

    let mut seen_ids = BTreeSet::new();
    let mut targets = Vec::new();
    let mut civilian_count = 0usize;

    for actor in state.onstage_actors(content) {
        if actor.id == *player_id {
            continue;
        }
        if state.actor_is_defeated(&actor.id, health_stat_id) {
            continue;
        }
        if state.follows_player(&actor.id)
            || state
                .relationships
                .get(&actor.id)
                .is_some_and(|r| r.follows_player || r.stance == ActorStance::Allied)
        {
            continue;
        }

        let actor_room = state.actor_room_id(&actor.id, &actor.room_id);
        if !floor_rooms.contains(actor_room) {
            continue;
        }

        let in_player_room = actor_room == state.current_room_id;
        let visited_actor_room = state.actor_has_visited_room(player_id, actor_room);
        let visited_home_room =
            !actor.room_id.is_empty() && state.actor_has_visited_room(player_id, &actor.room_id);
        let studied_actor = state.actor_has_studied_actor(player_id, &actor.id);
        let studied_template = actor
            .id
            .split_once('-')
            .map(|(prefix, _)| prefix)
            .is_some_and(|prefix| state.actor_has_studied_actor(player_id, prefix));

        let encountered = in_player_room
            || visited_actor_room
            || visited_home_room
            || studied_actor
            || studied_template;
        if !encountered {
            continue;
        }

        if actor
            .tags
            .iter()
            .any(|t| t.eq_ignore_ascii_case("civilian"))
        {
            civilian_count += 1;
            continue;
        }

        let base_id = actor
            .id
            .split_once('-')
            .map(|(prefix, _)| prefix)
            .unwrap_or(&actor.id);

        if seen_ids.insert(base_id.to_string()) {
            let title = content
                .actor(base_id)
                .map(|t| t.name.clone())
                .unwrap_or_else(|| actor.name.clone());
            targets.push(HuntTargetInfo {
                id: base_id.to_string(),
                title,
                subtitle: None,
            });
        }
    }

    targets.sort_by(|a, b| a.title.cmp(&b.title));

    if civilian_count > 0 && seen_ids.insert("civilian".to_string()) {
        targets.push(HuntTargetInfo {
            id: "civilian".to_string(),
            title: "Civilians".to_string(),
            subtitle: Some("Courtyard refugees".to_string()),
        });
    }

    targets
}

pub(super) fn build_party_order_panels(
    content: &ContentPack,
    state: &WorldState,
    members: &[PartyMember],
) -> BTreeMap<String, Vec<PanelOptionData>> {
    let directives = content.settings.party.directives();
    let hunt_targets = build_hunt_targets_for_floor(content, state);
    let mut panels = BTreeMap::new();

    for member in members {
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
                let subtitle =
                    content.render_message(&format!("party.directive.{directive}.subtitle"), &[]);

                if directive == "hunt" {
                    let is_hunting = member.order == "hunt"
                        || member.order.starts_with("hunt ")
                        || member.order.starts_with("hunt:");
                    PanelOptionData {
                        id: "hunt".to_string(),
                        title,
                        subtitle,
                        command: None,
                        disabled: false,
                        selected: is_hunting,
                        group: None,
                        panel: Some(format!("party-order-hunt:{}", member.id)),
                    }
                } else {
                    let selected = member.order == *directive;
                    PanelOptionData {
                        id: directive.clone(),
                        title,
                        subtitle,
                        command: Some(format!("order {} {}", member.id, directive)),
                        disabled: selected,
                        selected,
                        group: None,
                        panel: None,
                    }
                }
            })
            .collect();
        panels.insert(member.order_panel.clone(), options);

        let hunt_panel_id = format!("party-order-hunt:{}", member.id);
        let hunt_options: Vec<PanelOptionData> = if hunt_targets.is_empty() {
            vec![PanelOptionData {
                id: "none".to_string(),
                title: "No targets encountered".to_string(),
                subtitle: Some("Explore the floor to encounter targets".to_string()),
                command: None,
                disabled: true,
                selected: false,
                group: None,
                panel: None,
            }]
        } else {
            hunt_targets
                .iter()
                .map(|target| {
                    let is_selected = member.order == format!("hunt {}", target.id)
                        || member.order == format!("hunt:{}", target.id);
                    PanelOptionData {
                        id: target.id.clone(),
                        title: target.title.clone(),
                        subtitle: target.subtitle.clone(),
                        command: Some(format!("order {} hunt {}", member.id, target.id)),
                        disabled: is_selected,
                        selected: is_selected,
                        group: None,
                        panel: None,
                    }
                })
                .collect()
        };
        panels.insert(hunt_panel_id, hunt_options);
    }

    panels
}

fn living_follower_ids(state: &WorldState, content: &ContentPack) -> Vec<String> {
    state
        .relationships
        .iter()
        .filter(|(actor_id, relationship)| {
            (relationship.follows_player || relationship.stance == ActorStance::Allied)
                && actor_id.as_str() != content.settings.combat.player_actor_id
                && !state.actor_is_offstage(content, actor_id)
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
    let mut slot_by_item: std::collections::BTreeMap<&str, Vec<&str>> =
        std::collections::BTreeMap::new();
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
            let usable = content
                .item(&id)
                .is_some_and(|item| !item.use_hook.is_empty());
            InventoryItem {
                label,
                count,
                id: Some(id),
                usable,
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
                usable: false,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cinder_core::content::types::{LevelDefinition, SkillDefinition, SkillKind};
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
        state
            .adjust_actor_stat(&content, "blair", "stamina", 4)
            .unwrap();
        state
            .adjust_actor_stat(&content, "blair", "stamina", -2)
            .unwrap();
        state.set_follows_player("blair", true);
        let runtime = CinderRuntime::new(content.clone(), false).unwrap();

        let members = build_party_members(&runtime, &state, &content);
        let blair = members
            .into_iter()
            .find(|member| member.id == "blair")
            .unwrap();

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
        let content = minimal_test_pack();
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
            skills: Vec::new(),
            in_room: true,
        }];

        let state = WorldState::new(&content);
        let panels = build_party_order_panels(&content, &state, &members);
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
    fn party_order_hunt_creates_subpanel_with_encountered_targets() {
        let mut content = minimal_test_pack();
        let player_id = content.settings.combat.player_actor_id.clone();
        content.actors.retain(|a| a.id == player_id);
        content.settings.party.directives = vec!["guard".to_string(), "hunt".to_string()];

        let mut enemy = minimal_test_pack().actors[0].clone();
        enemy.id = "wolf_raider".to_string();
        enemy.name = "Wolf Raider".to_string();
        enemy.room_id = "lounge".to_string();
        enemy.tags = Vec::new();

        let mut civ = minimal_test_pack().actors[0].clone();
        civ.id = "refugee_bob".to_string();
        civ.name = "Refugee Bob".to_string();
        civ.room_id = "lounge".to_string();
        civ.tags = vec!["civilian".to_string()];

        content.actors.push(enemy);
        content.actors.push(civ);
        cinder_core::engine::test_fixtures::rebuild_test_pack_indexes(&mut content);

        let mut state = WorldState::new(&content);
        state.current_room_id = "lounge".to_string();
        // Spawned raider instance
        let mut raider_inst = content.actor("wolf_raider").unwrap().clone();
        raider_inst.id = "wolf_raider-1".to_string();
        state
            .spawned_actors
            .insert(raider_inst.id.clone(), raider_inst);
        state.set_actor_room("wolf_raider-1", "lounge");
        state.set_actor_room("refugee_bob", "lounge");

        let members = vec![PartyMember {
            id: "sprite".to_string(),
            label: "Sprite".to_string(),
            level: 1,
            hp: 10,
            hp_max: 10,
            order: "hunt civilian".to_string(),
            order_panel: "party-order:sprite".to_string(),
            inventory: Vec::new(),
            equipped_items: Vec::new(),
            skills: Vec::new(),
            in_room: true,
        }];

        let panels = build_party_order_panels(&content, &state, &members);
        let main_options = &panels["party-order:sprite"];
        let hunt_opt = main_options.iter().find(|o| o.id == "hunt").unwrap();
        assert!(hunt_opt.selected);
        assert!(!hunt_opt.disabled);
        assert_eq!(hunt_opt.panel.as_deref(), Some("party-order-hunt:sprite"));
        assert_eq!(hunt_opt.command, None);

        let hunt_subpanel = &panels["party-order-hunt:sprite"];
        assert_eq!(hunt_subpanel.len(), 2);

        let raider_opt = hunt_subpanel
            .iter()
            .find(|o| o.id == "wolf_raider")
            .unwrap();
        assert_eq!(raider_opt.title, "Wolf Raider");
        assert_eq!(
            raider_opt.command.as_deref(),
            Some("order sprite hunt wolf_raider")
        );
        assert!(!raider_opt.selected);

        let civ_opt = hunt_subpanel.iter().find(|o| o.id == "civilian").unwrap();
        assert_eq!(civ_opt.title, "Civilians");
        assert_eq!(
            civ_opt.command.as_deref(),
            Some("order sprite hunt civilian")
        );
        assert!(civ_opt.selected);
        assert!(civ_opt.disabled);
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
        state.add_item_to_storage(
            "sigil",
            cinder_core::content::types::ItemStorageTarget::CurrentRoom,
            &room_id,
        );
        state.add_item_to_storage(
            "scroll",
            cinder_core::content::types::ItemStorageTarget::CurrentRoom,
            &room_id,
        );

        let items = build_current_room_items(&content, &state, &room_id);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id.as_deref(), Some("scroll"));
    }

    #[test]
    fn build_party_members_includes_inventory_and_equipment() {
        let mut content = minimal_test_pack();
        content.settings.equipment_slots =
            std::collections::BTreeSet::from(["weapon".to_string(), "ring".to_string()]);
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
                std::collections::BTreeMap::from([(
                    "weapon".to_string(),
                    "iron-sword".to_string(),
                )]),
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
        assert_eq!(
            companion.equipped_items[0].id.as_deref(),
            Some("iron-sword")
        );
    }

    #[test]
    fn build_party_members_resolves_owned_skills_from_content() {
        let mut content = minimal_test_pack();
        content.skills.skills.extend([
            SkillDefinition {
                id: "field-medicine".to_string(),
                label: "Field Medicine".to_string(),
                kind: Some(SkillKind::Heal),
                ..SkillDefinition::default()
            },
            SkillDefinition {
                id: "shield-wall".to_string(),
                label: "Shield Wall".to_string(),
                kind: Some(SkillKind::Defend),
                ..SkillDefinition::default()
            },
        ]);
        content.skill_index.insert(
            "field-medicine".to_string(),
            content.skills.skills.len() - 2,
        );
        content
            .skill_index
            .insert("shield-wall".to_string(), content.skills.skills.len() - 1);

        let follower_id = "companion-1";
        let mut state = WorldState::new(&content);
        state.set_follows_player(follower_id, true);
        state.actor_skills.insert(
            follower_id.to_string(),
            std::collections::BTreeSet::from([
                "field-medicine".to_string(),
                "missing-skill".to_string(),
                "shield-wall".to_string(),
            ]),
        );

        let runtime = CinderRuntime::new(content.clone(), false).unwrap();
        let member = build_party_members(&runtime, &state, &content)
            .into_iter()
            .find(|member| member.id == follower_id)
            .unwrap();

        assert_eq!(member.skills.len(), 2);
        assert_eq!(member.skills[0].id, "field-medicine");
        assert_eq!(member.skills[0].label, "Field Medicine");
        assert_eq!(member.skills[0].kind, Some(SkillKind::Heal));
        assert_eq!(member.skills[1].id, "shield-wall");
        assert_eq!(member.skills[1].kind, Some(SkillKind::Defend));
    }

    #[test]
    fn build_party_members_includes_comms_skill_for_awakened_member() {
        let mut content = minimal_test_pack();
        content.skills.skills.push(SkillDefinition {
            id: "comms".to_string(),
            label: "Comms".to_string(),
            kind: Some(SkillKind::Support),
            ..SkillDefinition::default()
        });
        content
            .skill_index
            .insert("comms".to_string(), content.skills.skills.len() - 1);

        let follower_id = "companion-1";
        let mut state = WorldState::new(&content);
        state.set_follows_player(follower_id, true);
        state.set_transformation_applied(follower_id, "awakening");

        let runtime = CinderRuntime::new(content.clone(), false).unwrap();
        let member = build_party_members(&runtime, &state, &content)
            .into_iter()
            .find(|member| member.id == follower_id)
            .unwrap();

        let comms_skill = member.skills.iter().find(|s| s.id == "comms");
        assert!(
            comms_skill.is_some(),
            "Awakened member must show comms skill in sidebar"
        );
        assert_eq!(comms_skill.unwrap().label, "Comms");
        assert_eq!(comms_skill.unwrap().kind, Some(SkillKind::Support));
    }
}
