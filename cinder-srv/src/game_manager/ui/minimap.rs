use cinder_core::content::types::{ContentPack, MapRevealCondition};
use cinder_core::engine::state::{ActorStance, WorldState};
use std::collections::BTreeSet;

use super::{MinimapConnection, MinimapData, MinimapRoom};

pub(super) fn build_minimap(
    state: &WorldState,
    content: &ContentPack,
    current_room_id: &str,
) -> Option<MinimapData> {
    let map = content.map_for_room(current_room_id)?;
    let player_id = &content.settings.combat.player_actor_id;
    let visited_room_ids = map
        .rooms
        .iter()
        .filter(|room| {
            room.room_id == current_room_id
                || state.actor_has_visited_room(player_id, &room.room_id)
        })
        .map(|room| room.room_id.as_str())
        .collect::<BTreeSet<_>>();
    let fully_revealed = visited_room_ids.len() == map.rooms.len()
        || map
            .reveal_conditions
            .iter()
            .any(|condition| reveal_condition_met(state, content, condition));
    let visible_room_ids = if fully_revealed {
        map.rooms
            .iter()
            .map(|room| room.room_id.as_str())
            .collect::<BTreeSet<_>>()
    } else {
        visited_room_ids.clone()
    };

    let entity_tracking = story_var_is_truthy(state, "has_sensory_enhancer")
        || story_var_is_truthy(state, "map_entity_tracking");

    let is_teleport_anchor = |room_id: &str| -> bool {
        let permanent = content.teleports.permanent_anchors.iter().any(|anchor| {
            anchor.room_id == room_id
                && (anchor.armed_by.is_empty() || story_var_is_truthy(state, &anchor.armed_by))
        });
        let chalk = state.chalk_anchors.iter().any(|r| r == room_id);
        permanent || chalk
    };

    let rooms: Vec<MinimapRoom> = map
        .rooms
        .iter()
        .filter(|room| visible_room_ids.contains(room.room_id.as_str()))
        .filter_map(|map_room| {
            let room = content.room(&map_room.room_id)?;
            let has_teleport = entity_tracking && is_teleport_anchor(&map_room.room_id);
            let (ally_names, hostile_names) = if entity_tracking {
                let mut allies = Vec::new();
                let mut hostiles = Vec::new();
                for actor in state.onstage_actors(content) {
                    if actor.id == *player_id {
                        continue;
                    }
                    if state.actor_is_defeated(&actor.id, &content.settings.combat.health_stat_id) {
                        continue;
                    }
                    if state.actor_is_in_room(content, &actor.id, &map_room.room_id) {
                        let name = content
                            .actor(&actor.id)
                            .map(|a| a.name.clone())
                            .unwrap_or_else(|| actor.id.clone());
                        match state.stance(&actor.id) {
                            ActorStance::Allied => allies.push(name),
                            ActorStance::Hostile => hostiles.push(name),
                            _ => {}
                        }
                    }
                }
                (allies, hostiles)
            } else {
                (Vec::new(), Vec::new())
            };
            Some(MinimapRoom {
                id: room.id.clone(),
                label: room.title.clone(),
                x: map_room.x,
                y: map_room.y,
                current: room.id == current_room_id,
                visited: visited_room_ids.contains(room.id.as_str()),
                has_teleport,
                ally_count: ally_names.len(),
                hostile_count: hostile_names.len(),
                ally_names,
                hostile_names,
            })
        })
        .collect();

    let mut seen_connections = BTreeSet::new();
    let mut connections = Vec::new();
    for map_room in &map.rooms {
        if !visible_room_ids.contains(map_room.room_id.as_str()) {
            continue;
        }
        let Some(room) = content.room(&map_room.room_id) else {
            continue;
        };
        for exit in &room.exits {
            if !visible_room_ids.contains(exit.room_id.as_str())
                || (!exit.requires_story_var.is_empty()
                    && !story_var_is_truthy(state, &exit.requires_story_var))
            {
                continue;
            }
            let pair = if room.id < exit.room_id {
                (room.id.as_str(), exit.room_id.as_str())
            } else {
                (exit.room_id.as_str(), room.id.as_str())
            };
            if seen_connections.insert(pair) {
                connections.push(MinimapConnection {
                    from: pair.0.to_string(),
                    to: pair.1.to_string(),
                });
            }
        }
    }

    let has_teleports = rooms.iter().any(|r| r.has_teleport);

    Some(MinimapData {
        id: map.id.clone(),
        label: map.label.clone(),
        fully_revealed,
        visited_count: visited_room_ids.len(),
        total_count: fully_revealed.then_some(map.rooms.len()),
        rooms,
        connections,
        entity_tracking,
        has_teleports,
    })
}

fn reveal_condition_met(
    state: &WorldState,
    content: &ContentPack,
    condition: &MapRevealCondition,
) -> bool {
    match condition {
        MapRevealCondition::Always => true,
        MapRevealCondition::ActorDefeated { actor_id } => {
            state.actor_is_defeated(actor_id, &content.settings.combat.health_stat_id)
        }
        MapRevealCondition::StoryVarTruthy { key } => story_var_is_truthy(state, key),
    }
}

fn story_var_is_truthy(state: &WorldState, key: &str) -> bool {
    state.story_vars.get(key).is_some_and(|value| {
        !matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "" | "false" | "0"
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cinder_core::content::types::{MapDefinition, MapRevealCondition, MapRoomDefinition};
    use cinder_core::engine::test_fixtures::minimal_test_pack;

    fn mapped_pack() -> ContentPack {
        let mut content = minimal_test_pack();
        content.settings.combat.player_actor_id = "player".to_string();
        content.settings.combat.health_stat_id = "stamina".to_string();
        content.maps = vec![MapDefinition {
            id: "test-floor".to_string(),
            label: "Test Floor".to_string(),
            rooms: vec![
                MapRoomDefinition {
                    room_id: "lounge".to_string(),
                    x: 0.0,
                    y: 0.0,
                },
                MapRoomDefinition {
                    room_id: "kitchen".to_string(),
                    x: 1.0,
                    y: 0.0,
                },
            ],
            reveal_conditions: vec![MapRevealCondition::ActorDefeated {
                actor_id: "casey".to_string(),
            }],
        }];
        content
    }

    #[test]
    fn minimap_hides_unvisited_rooms_and_connections() {
        let content = mapped_pack();
        let state = WorldState::new(&content);

        let minimap = build_minimap(&state, &content, "lounge").unwrap();

        assert!(!minimap.fully_revealed);
        assert_eq!(minimap.rooms.len(), 1);
        assert_eq!(minimap.rooms[0].id, "lounge");
        assert!(minimap.connections.is_empty());
    }

    #[test]
    fn defeating_reveal_actor_shows_the_complete_map() {
        let content = mapped_pack();
        let mut state = WorldState::new(&content);
        state
            .adjust_actor_stat(&content, "casey", "stamina", -100)
            .unwrap();

        let minimap = build_minimap(&state, &content, "lounge").unwrap();

        assert!(minimap.fully_revealed);
        assert_eq!(minimap.rooms.len(), 2);
        assert_eq!(minimap.connections.len(), 1);
        assert!(
            !minimap
                .rooms
                .iter()
                .find(|room| room.id == "kitchen")
                .unwrap()
                .visited
        );
    }

    #[test]
    fn visiting_every_room_reveals_the_complete_map() {
        let content = mapped_pack();
        let mut state = WorldState::new(&content);
        state.mark_actor_room_visited("player", "kitchen");

        let minimap = build_minimap(&state, &content, "kitchen").unwrap();

        assert!(minimap.fully_revealed);
        assert_eq!(minimap.visited_count, 2);
    }

    #[test]
    fn full_reveal_does_not_expose_a_gated_connection() {
        let mut content = mapped_pack();
        content.rooms[0].exits[0].requires_story_var = "secret_open".to_string();
        content.rooms[1].exits[0].requires_story_var = "secret_open".to_string();
        let mut state = WorldState::new(&content);
        state.mark_actor_room_visited("player", "kitchen");

        let hidden = build_minimap(&state, &content, "kitchen").unwrap();
        assert!(hidden.fully_revealed);
        assert!(hidden.connections.is_empty());

        state.story_vars.set_unchecked("secret_open", "true");
        let revealed = build_minimap(&state, &content, "kitchen").unwrap();
        assert_eq!(revealed.connections.len(), 1);
    }

    #[test]
    fn always_reveal_condition_reveals_map_from_start() {
        let mut content = mapped_pack();
        content.maps[0].reveal_conditions = vec![MapRevealCondition::Always];
        let state = WorldState::new(&content);

        let minimap = build_minimap(&state, &content, "lounge").unwrap();

        assert!(minimap.fully_revealed);
        assert_eq!(minimap.rooms.len(), 2);
        assert_eq!(minimap.connections.len(), 1);
    }

    #[test]
    fn minimap_shows_teleport_anchor_when_armed() {
        use cinder_core::content::types::TeleportAnchorDefinition;

        let mut content = mapped_pack();
        content
            .teleports
            .permanent_anchors
            .push(TeleportAnchorDefinition {
                room_id: "kitchen".to_string(),
                armed_by: "anchor_kitchen".to_string(),
                title: "Kitchen Anchor".to_string(),
                aliases: vec!["kitchen".to_string()],
            });

        let mut state = WorldState::new(&content);
        state.mark_actor_room_visited("player", "kitchen");

        // Before arming
        let minimap = build_minimap(&state, &content, "kitchen").unwrap();
        let kitchen = minimap.rooms.iter().find(|r| r.id == "kitchen").unwrap();
        assert!(!kitchen.has_teleport);
        assert!(!minimap.has_teleports);

        // After arming, but without sensory enhancer -> still false
        state.story_vars.set_unchecked("anchor_kitchen", "true");
        let minimap = build_minimap(&state, &content, "kitchen").unwrap();
        let kitchen = minimap.rooms.iter().find(|r| r.id == "kitchen").unwrap();
        assert!(!kitchen.has_teleport);
        assert!(!minimap.has_teleports);

        // With sensory enhancer active -> teleport portal revealed
        state
            .story_vars
            .set_unchecked("has_sensory_enhancer", "true");
        let minimap = build_minimap(&state, &content, "kitchen").unwrap();
        let kitchen = minimap.rooms.iter().find(|r| r.id == "kitchen").unwrap();
        assert!(kitchen.has_teleport);
        assert!(minimap.has_teleports);
    }

    #[test]
    fn minimap_tracks_live_entities_when_sensory_enhancer_active() {
        let content = mapped_pack();
        let mut state = WorldState::new(&content);
        state.mark_actor_room_visited("player", "kitchen");

        // Place an ally in lounge and a hostile in lounge
        state.set_actor_stance(&content, "blair", ActorStance::Allied, false);
        state.set_actor_stance(&content, "casey", ActorStance::Hostile, false);

        // Without sensory enhancer -> no entity tracking
        let minimap = build_minimap(&state, &content, "lounge").unwrap();
        assert!(!minimap.entity_tracking);
        assert_eq!(minimap.rooms[0].ally_count, 0);
        assert_eq!(minimap.rooms[0].hostile_count, 0);

        // With sensory enhancer active -> entity tracking live
        state
            .story_vars
            .set_unchecked("has_sensory_enhancer", "true");
        let minimap = build_minimap(&state, &content, "lounge").unwrap();
        assert!(minimap.entity_tracking);
        let lounge = minimap.rooms.iter().find(|r| r.id == "lounge").unwrap();
        assert_eq!(lounge.ally_count, 1);
        assert_eq!(lounge.ally_names, vec!["Blair".to_string()]);
        assert_eq!(lounge.hostile_count, 1);
        assert_eq!(lounge.hostile_names, vec!["Casey".to_string()]);
    }
}
