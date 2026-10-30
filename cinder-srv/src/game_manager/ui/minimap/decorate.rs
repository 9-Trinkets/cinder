//! Decoration layer: turns a visible map room into a [`MinimapRoom`], adding the
//! live allies/hostiles the player can perceive and any armed teleport anchor.

use cinder_core::content::types::{ContentPack, MapRoomDefinition};
use cinder_core::engine::state::{ActorStance, WorldState};

use super::visibility::MapVisibility;
use crate::game_manager::ui::MinimapRoom;

/// Annotates map rooms with entity and teleport information. All the context
/// needed to decorate a room is captured once so each room costs only two args.
pub(super) struct RoomDecorator<'a> {
    content: &'a ContentPack,
    state: &'a WorldState,
    player_id: &'a str,
    entity_tracking: bool,
}

impl<'a> RoomDecorator<'a> {
    pub(super) fn new(
        content: &'a ContentPack,
        state: &'a WorldState,
        player_id: &'a str,
        entity_tracking: bool,
    ) -> Self {
        Self {
            content,
            state,
            player_id,
            entity_tracking,
        }
    }

    pub(super) fn annotate(
        &self,
        map_room: &MapRoomDefinition,
        current_room_id: &str,
        visibility: &MapVisibility,
    ) -> Option<MinimapRoom> {
        let room = self.content.room(&map_room.room_id)?;
        let (ally_names, hostile_names) = self.entity_names(&map_room.room_id);
        let visited = visibility.is_visited(&room.id);
        let has_stairs = (visited || visibility.fully_revealed())
            && room_has_stairs(self.content, self.state, map_room);
        Some(MinimapRoom {
            id: room.id.clone(),
            label: room.title.clone(),
            x: map_room.x,
            y: map_room.y,
            current: room.id == current_room_id,
            visited,
            has_teleport: self.entity_tracking
                && is_teleport_anchor(self.content, self.state, &map_room.room_id),
            has_stairs,
            ally_count: ally_names.len(),
            hostile_count: hostile_names.len(),
            ally_names,
            hostile_names,
        })
    }

    /// Names of the onstage allies and hostiles in `room_id`. Empty unless
    /// sensory tracking (the map entity overlay) is active.
    fn entity_names(&self, room_id: &str) -> (Vec<String>, Vec<String>) {
        if !self.entity_tracking {
            return (Vec::new(), Vec::new());
        }
        let mut allies = Vec::new();
        let mut hostiles = Vec::new();
        for actor in self.state.onstage_actors(self.content) {
            if actor.id == self.player_id
                || self
                    .state
                    .actor_is_defeated(&actor.id, &self.content.settings.combat.health_stat_id)
                || !self
                    .state
                    .actor_is_in_room(self.content, &actor.id, room_id)
            {
                continue;
            }
            let name = self
                .content
                .actor(&actor.id)
                .map(|a| a.name.clone())
                .unwrap_or_else(|| actor.id.clone());
            match self.state.stance(&actor.id) {
                ActorStance::Allied => allies.push(name),
                ActorStance::Hostile => hostiles.push(name),
                _ => {}
            }
        }
        (allies, hostiles)
    }
}

/// Whether a room holds an armed permanent anchor or a chalk-marked anchor.
fn is_teleport_anchor(content: &ContentPack, state: &WorldState, room_id: &str) -> bool {
    let permanent =
        content.teleports.permanent_anchors.iter().any(|anchor| {
            anchor.room_id == room_id && content.teleport_anchor_is_armed(state, anchor)
        });
    let chalk = state.chalk_anchors.iter().any(|anchor| anchor == room_id);
    permanent || chalk
}

/// Whether a room contains stairs (inter-floor connection or explicit stair exit/feature).
fn room_has_stairs(
    content: &ContentPack,
    state: &WorldState,
    map_room: &MapRoomDefinition,
) -> bool {
    if let Some(explicit) = map_room.has_stairs {
        return explicit;
    }
    let Some(room) = content.room(&map_room.room_id) else {
        return false;
    };
    let has_stair_feature = room.features.iter().any(|f| {
        f.id.to_ascii_lowercase().contains("stair")
            || f.label.to_ascii_lowercase().contains("stair")
    });
    room.exits.iter().any(|exit| {
        if !content.exit_is_open(state, &room.id, exit) {
            return false;
        }
        if exit.label.to_ascii_lowercase().contains("stair")
            || exit
                .menu_label
                .as_deref()
                .map_or(false, |m| m.to_ascii_lowercase().contains("stair"))
            || exit.aliases.iter().any(|a| {
                let alias = a.trim().to_ascii_lowercase();
                alias == "up"
                    || alias == "down"
                    || alias.contains("stair")
                    || alias.contains("step")
            })
        {
            return true;
        }
        if has_stair_feature {
            let from_map = content.map_for_room(&room.id).map(|m| &m.id);
            let to_map = content.map_for_room(&exit.room_id).map(|m| &m.id);
            if from_map.is_some() && to_map.is_some() && from_map != to_map {
                return true;
            }
        }
        false
    })
}
