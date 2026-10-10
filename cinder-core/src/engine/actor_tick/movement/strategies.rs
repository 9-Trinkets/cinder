use super::{MovementStrategy, next_room_toward, path_distance};
use crate::content::types::{ActorDefinition, ContentPack};
use crate::engine::state::{ActorStance, WorldState};
use crate::engine::turn_policies::story_var_is_truthy;
use rand::Rng;

/// Stationary strategy for guarding orders.
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

/// Finds the target room for an actor on a hunt order.
/// Target can be an actor id, an actor name, an actor tag (e.g. "civilian"),
/// or a room id.
pub(crate) fn find_hunt_target_room_id(
    content: &ContentPack,
    state: &WorldState,
    hunter_id: &str,
    target: &str,
) -> Option<String> {
    let target = target.trim();
    if target.is_empty() {
        return None;
    }
    let hunter_stance = state.stance(hunter_id);
    let hunter_room = state.actor_current_room_id(content, hunter_id);
    let health_stat_id = if content.settings.combat.health_stat_id.is_empty() {
        "hp"
    } else {
        &content.settings.combat.health_stat_id
    };

    // 1. Look for living, onstage actors matching target by id, name, alias, or template prefix
    let matching_actors: Vec<_> = state
        .onstage_actors(content)
        .filter(|a| a.id != hunter_id)
        .filter(|a| !state.actor_is_defeated(&a.id, health_stat_id))
        .filter(|a| {
            a.id.eq_ignore_ascii_case(target)
                || a.name.eq_ignore_ascii_case(target)
                || a.aliases
                    .iter()
                    .any(|alias| alias.eq_ignore_ascii_case(target))
                || a.id
                    .split_once('-')
                    .map(|(t, _)| t)
                    .is_some_and(|t| t.eq_ignore_ascii_case(target))
        })
        .collect();

    // 2. Look for living, onstage actors matching target by tag (e.g. "civilian")
    let tagged_actors: Vec<_> = if matching_actors.is_empty() {
        state
            .onstage_actors(content)
            .filter(|a| a.id != hunter_id)
            .filter(|a| !state.actor_is_defeated(&a.id, health_stat_id))
            .filter(|a| {
                if hunter_stance == ActorStance::Allied
                    && state.stance(&a.id) == ActorStance::Allied
                {
                    return false;
                }
                if hunter_stance == ActorStance::Hostile
                    && state.stance(&a.id) == ActorStance::Hostile
                {
                    return false;
                }
                a.tags.iter().any(|t| t.eq_ignore_ascii_case(target))
            })
            .collect()
    } else {
        Vec::new()
    };

    let candidates = if !matching_actors.is_empty() {
        matching_actors
    } else {
        tagged_actors
    };

    if !candidates.is_empty() {
        let mut best_target_room: Option<(usize, String)> = None;
        let mut same_board_room: Option<String> = None;
        let board_room_ids = content.reachable_room_ids(hunter_room);

        for a in &candidates {
            let candidate_room = state.actor_current_room_id(content, &a.id);
            if let Some(dist) = path_distance(content, state, hunter_room, candidate_room) {
                if best_target_room
                    .as_ref()
                    .is_none_or(|(best_dist, _)| dist < *best_dist)
                {
                    best_target_room = Some((dist, candidate_room.to_string()));
                }
            } else if same_board_room.is_none() && board_room_ids.contains(candidate_room) {
                same_board_room = Some(candidate_room.to_string());
            }
        }

        if let Some((_, room)) = best_target_room {
            return Some(room);
        }
        if let Some(room) = same_board_room {
            return Some(room);
        }
        return Some(
            state
                .actor_current_room_id(content, &candidates[0].id)
                .to_string(),
        );
    }

    // 3. Check if target matches a room id
    if content.room(target).is_some() {
        return Some(target.to_string());
    }

    // 4. Fallback for hostile hunters targeting civilians if all civilians are dead: hunt player!
    if hunter_stance == ActorStance::Hostile && target.eq_ignore_ascii_case("civilian") {
        return Some(state.current_room_id.clone());
    }

    None
}

/// Pathfinds toward an actor, actor tag, or destination.
#[derive(Debug, Clone)]
pub(crate) struct HuntStrategy {
    pub cadence: u32,
    pub target: String,
}

impl HuntStrategy {
    pub(crate) fn new(cadence: u32, target: String) -> Self {
        Self { cadence, target }
    }
}

impl MovementStrategy for HuntStrategy {
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
        let target_room_id = find_hunt_target_room_id(content, state, &actor.id, &self.target)?;
        if target_room_id == current_room_id {
            return None;
        }
        next_room_toward(content, state, current_room_id, &target_room_id)
    }
}
