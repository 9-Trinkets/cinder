use super::{MovementStrategy, next_room_toward};
use crate::content::types::{ActorDefinition, ContentPack};
use crate::engine::state::WorldState;
use crate::engine::turn_policies::story_var_is_truthy;
use rand::Rng;

/// Stationary strategy for guarding, sentry, or hold orders.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct StayStrategy;

impl MovementStrategy for StayStrategy {
    fn cadence_ticks(&self) -> u32 {
        0
    }

    fn plan_destination(
        &self,
        _content: &ContentPack,
        _state: &WorldState,
        _actor: &ActorDefinition,
        _current_room_id: &str,
    ) -> Option<String> {
        None
    }
}

/// Randomly chooses an adjacent reachable room.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RandomAdjacentStrategy {
    pub cadence: u32,
}

impl RandomAdjacentStrategy {
    pub(crate) fn new(cadence: u32) -> Self {
        Self { cadence }
    }
}

impl MovementStrategy for RandomAdjacentStrategy {
    fn cadence_ticks(&self) -> u32 {
        self.cadence
    }

    fn plan_destination(
        &self,
        content: &ContentPack,
        state: &WorldState,
        _actor: &ActorDefinition,
        current_room_id: &str,
    ) -> Option<String> {
        let room = content.room(current_room_id)?;
        let neighbors: Vec<_> = room
            .exits
            .iter()
            .filter(|exit| {
                content.room_is_reachable(&exit.room_id)
                    && (exit.requires_story_var.is_empty()
                        || story_var_is_truthy(state, &exit.requires_story_var))
            })
            .map(|exit| exit.room_id.clone())
            .collect();
        if neighbors.is_empty() {
            return None;
        }
        let index = rand::thread_rng().gen_range(0..neighbors.len());
        Some(neighbors[index].clone())
    }
}

/// Pathfinds toward the player's current room.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TowardPlayerStrategy {
    pub cadence: u32,
}

impl TowardPlayerStrategy {
    pub(crate) fn new(cadence: u32) -> Self {
        Self { cadence }
    }
}

impl MovementStrategy for TowardPlayerStrategy {
    fn cadence_ticks(&self) -> u32 {
        self.cadence
    }

    fn plan_destination(
        &self,
        content: &ContentPack,
        state: &WorldState,
        _actor: &ActorDefinition,
        current_room_id: &str,
    ) -> Option<String> {
        next_room_toward(content, state, current_room_id, &state.current_room_id)
    }
}

/// Pathfinds toward a designated destination room.
#[derive(Debug, Clone)]
pub(crate) struct ToDestinationStrategy {
    pub cadence: u32,
    pub destination_room_id: String,
}

impl ToDestinationStrategy {
    pub(crate) fn new(cadence: u32, destination_room_id: String) -> Self {
        Self {
            cadence,
            destination_room_id,
        }
    }
}

impl MovementStrategy for ToDestinationStrategy {
    fn cadence_ticks(&self) -> u32 {
        self.cadence
    }

    fn plan_destination(
        &self,
        content: &ContentPack,
        state: &WorldState,
        actor: &ActorDefinition,
        current_room_id: &str,
    ) -> Option<String> {
        let destination = if self.destination_room_id.is_empty() {
            &actor.room_id
        } else {
            &self.destination_room_id
        };
        next_room_toward(content, state, current_room_id, destination)
    }
}

/// Follows a named exit label or alias from the current room.
#[derive(Debug, Clone)]
pub(crate) struct ExitLabelStrategy {
    pub cadence: u32,
    pub exit_label: String,
}

impl ExitLabelStrategy {
    pub(crate) fn new(cadence: u32, exit_label: String) -> Self {
        Self {
            cadence,
            exit_label,
        }
    }
}

impl MovementStrategy for ExitLabelStrategy {
    fn cadence_ticks(&self) -> u32 {
        self.cadence
    }

    fn plan_destination(
        &self,
        content: &ContentPack,
        state: &WorldState,
        _actor: &ActorDefinition,
        current_room_id: &str,
    ) -> Option<String> {
        let room = content.room(current_room_id)?;
        let target_exit = room.exits.iter().find(|exit| {
            (exit.label.eq_ignore_ascii_case(&self.exit_label)
                || exit
                    .aliases
                    .iter()
                    .any(|alias| alias.eq_ignore_ascii_case(&self.exit_label)))
                && (exit.requires_story_var.is_empty()
                    || story_var_is_truthy(state, &exit.requires_story_var))
        })?;
        if content.room_is_reachable(&target_exit.room_id) {
            Some(target_exit.room_id.clone())
        } else {
            None
        }
    }
}
