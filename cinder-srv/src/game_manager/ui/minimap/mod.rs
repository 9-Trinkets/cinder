//! Minimap projection, split into layers: a [`RevealPolicy`] decides visibility,
//! a [`RoomDecorator`] annotates visible rooms with entities and teleport
//! anchors, and [`MinimapProjector`] orchestrates them into a [`MinimapData`]
//! payload together with the de-duplicated room connections.

mod decorate;
mod visibility;

use cinder_core::content::types::{ContentPack, MapDefinition, RoomExitDefinition};
use cinder_core::engine::state::WorldState;
use std::collections::BTreeSet;

use self::decorate::RoomDecorator;
use self::visibility::{MapVisibility, RevealPolicy, VisitedAndConditionRevealPolicy};
use super::{MinimapConnection, MinimapData, MinimapRoom};

pub(super) fn build_minimap(
    state: &WorldState,
    content: &ContentPack,
    current_room_id: &str,
) -> Option<MinimapData> {
    MinimapProjector {
        reveal: VisitedAndConditionRevealPolicy,
    }
    .project(state, content, current_room_id)
}

/// Orchestrates the minimap layers for the map containing `current_room_id`.
struct MinimapProjector<P: RevealPolicy> {
    reveal: P,
}

impl<P: RevealPolicy> MinimapProjector<P> {
    fn project(
        &self,
        state: &WorldState,
        content: &ContentPack,
        current_room_id: &str,
    ) -> Option<MinimapData> {
        let map = content.map_for_room(current_room_id)?;
        let player_id = &content.settings.combat.player_actor_id;
        let visibility = self.reveal.visibility(map, state, content, current_room_id);
        let entity_tracking = entity_tracking_enabled(content, state);
        let decorator = RoomDecorator::new(content, state, player_id, entity_tracking);
        let rooms = project_rooms(map, &visibility, current_room_id, &decorator);
        let connections = project_connections(content, state, map, &visibility);
        let has_teleports = rooms.iter().any(|room| room.has_teleport);

        Some(MinimapData {
            id: map.id.clone(),
            label: map.label.clone(),
            fully_revealed: visibility.fully_revealed(),
            visited_count: visibility.visited_count(),
            total_count: visibility.total_count(),
            rooms,
            connections,
            entity_tracking,
            has_teleports,
        })
    }
}

/// Whether the entity overlay (allies/hostiles and teleport anchors) is active.
fn entity_tracking_enabled(content: &ContentPack, state: &WorldState) -> bool {
    let when = &content.settings.minimap_entity_tracking_when;
    !when.is_empty()
        && cinder_core::engine::conditions::all_hold(
            when,
            &cinder_core::engine::conditions::ConditionContext::new(
                content,
                state,
                &state.current_room_id,
            ),
        )
}

/// Projects the visible rooms of `map` into decorated [`MinimapRoom`]s.
fn project_rooms(
    map: &MapDefinition,
    visibility: &MapVisibility,
    current_room_id: &str,
    decorator: &RoomDecorator,
) -> Vec<MinimapRoom> {
    map.rooms
        .iter()
        .filter(|room| visibility.is_visible(&room.room_id))
        .filter_map(|map_room| decorator.annotate(map_room, current_room_id, visibility))
        .collect()
}

/// Projects the undirected connections between visible rooms, de-duplicating
/// each pair and hiding exits gated by an unsatisfied story variable.
fn project_connections(
    content: &ContentPack,
    state: &WorldState,
    map: &MapDefinition,
    visibility: &MapVisibility,
) -> Vec<MinimapConnection> {
    let mut seen = BTreeSet::new();
    let mut connections = Vec::new();
    for map_room in &map.rooms {
        if !visibility.is_visible(&map_room.room_id) {
            continue;
        }
        let Some(room) = content.room(&map_room.room_id) else {
            continue;
        };
        for exit in &room.exits {
            if !exit_is_revealed(content, state, &room.id, exit, visibility) {
                continue;
            }
            let pair = ordered_room_pair(&room.id, &exit.room_id);
            if seen.insert(pair) {
                connections.push(MinimapConnection {
                    from: pair.0.to_string(),
                    to: pair.1.to_string(),
                });
            }
        }
    }
    connections
}

fn exit_is_revealed(
    content: &ContentPack,
    state: &WorldState,
    from_room_id: &str,
    exit: &RoomExitDefinition,
    visibility: &MapVisibility,
) -> bool {
    visibility.is_visible(&exit.room_id) && content.exit_is_open(state, from_room_id, exit)
}

/// Orders a connection's two endpoints so each undirected pair is keyed once.
fn ordered_room_pair<'a>(a: &'a str, b: &'a str) -> (&'a str, &'a str) {
    if a < b { (a, b) } else { (b, a) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cinder_core::content::types::{Condition, MapDefinition, MapRoomDefinition};
    use cinder_core::engine::state::ActorStance;
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
            fog: true,
            reveal_when: vec![Condition::Defeated("casey".to_string())],
        }];
        content.settings.minimap_entity_tracking_when = vec![Condition::Any(vec![
            Condition::StoryVar("has_sensory_enhancer".to_string()),
            Condition::StoryVar("map_entity_tracking".to_string()),
        ])];
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
        content.rooms[0].exits[0].when = vec![cinder_core::content::types::Condition::StoryVar(
            "secret_open".to_string(),
        )];
        content.rooms[1].exits[0].when = vec![cinder_core::content::types::Condition::StoryVar(
            "secret_open".to_string(),
        )];
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
        content.maps[0].fog = false;
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
                when: vec![cinder_core::content::types::Condition::StoryVar(
                    "anchor_kitchen".to_string(),
                )],
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
