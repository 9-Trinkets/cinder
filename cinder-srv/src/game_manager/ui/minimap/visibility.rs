//! Visibility layer: decides which rooms a minimap reveals and which the player
//! has visited.

use cinder_core::content::types::{ContentPack, MapDefinition, MapRevealCondition};
use cinder_core::engine::state::WorldState;
use std::collections::BTreeSet;

/// The set of rooms a minimap should show, and the subset the player has been
/// in. Built once per projection so downstream layers query it instead of
/// re-deriving reveal rules.
pub(super) struct MapVisibility {
    fully_revealed: bool,
    total_rooms: usize,
    visited: BTreeSet<String>,
    visible: BTreeSet<String>,
}

impl MapVisibility {
    pub(super) fn fully_revealed(&self) -> bool {
        self.fully_revealed
    }

    pub(super) fn visited_count(&self) -> usize {
        self.visited.len()
    }

    /// The total room count, but only once the map is fully revealed so the
    /// count itself does not leak hidden rooms.
    pub(super) fn total_count(&self) -> Option<usize> {
        self.fully_revealed.then_some(self.total_rooms)
    }

    pub(super) fn is_visible(&self, room_id: &str) -> bool {
        self.visible.contains(room_id)
    }

    pub(super) fn is_visited(&self, room_id: &str) -> bool {
        self.visited.contains(room_id)
    }
}

/// Decides which rooms a map reveals for a given world state. Injected into
/// [`super::MinimapProjector`] so reveal rules can be swapped or faked in tests.
pub(super) trait RevealPolicy {
    fn visibility(
        &self,
        map: &MapDefinition,
        state: &WorldState,
        content: &ContentPack,
        current_room_id: &str,
    ) -> MapVisibility;
}

/// Default reveal policy: a room shows once visited (or when it is the current
/// room), and the whole map opens when every room is visited or a declared
/// reveal condition is met.
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct VisitedAndConditionRevealPolicy;

impl RevealPolicy for VisitedAndConditionRevealPolicy {
    fn visibility(
        &self,
        map: &MapDefinition,
        state: &WorldState,
        content: &ContentPack,
        current_room_id: &str,
    ) -> MapVisibility {
        let player_id = &content.settings.combat.player_actor_id;
        let visited = map
            .rooms
            .iter()
            .filter(|room| {
                room.room_id == current_room_id
                    || state.actor_has_visited_room(player_id, &room.room_id)
            })
            .map(|room| room.room_id.clone())
            .collect::<BTreeSet<_>>();
        let fully_revealed = visited.len() == map.rooms.len()
            || map
                .reveal_conditions
                .iter()
                .any(|condition| reveal_condition_met(state, content, condition));
        let visible = if fully_revealed {
            map.rooms.iter().map(|room| room.room_id.clone()).collect()
        } else {
            visited.clone()
        };
        MapVisibility {
            fully_revealed,
            total_rooms: map.rooms.len(),
            visited,
            visible,
        }
    }
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

/// Whether a story variable is set to a truthy (non-empty, non-"false", non-"0")
/// value.
pub(super) fn story_var_is_truthy(state: &WorldState, key: &str) -> bool {
    cinder_core::engine::conditions::story_var_is_truthy(state, key)
}
