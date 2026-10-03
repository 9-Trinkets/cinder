pub(crate) mod pathfinding;
pub(crate) mod strategies;

use super::{room_is_in_tick_scope, tick_scope_room_ids};
use crate::content::types::{ActorDefinition, ContentPack, WanderDefinition, WanderMode};
use crate::engine::behavior::should_hold;
use crate::engine::events::WorldEvent;
use crate::engine::state::{ActorStance, GamePhase, WorldState};
use std::collections::BTreeSet;
use std::error::Error;
use std::sync::Arc;

use crate::content::types::ActorMovementRulesDefinition;
use crate::engine::actor_turn::movement::required_movement_target_room_id;

pub(crate) use pathfinding::next_room_toward;
pub(crate) use strategies::{
    ExitLabelStrategy, RandomAdjacentStrategy, StayStrategy, ToDestinationStrategy,
    TowardPlayerStrategy,
};

/// Strategy defining movement destination selection and cadence for an actor.
pub(crate) trait MovementStrategy: Send + Sync {
    /// Number of minutes/ticks between movements. 0 means stationary.
    fn cadence_ticks(&self) -> u32;

    /// Plans the next room destination from `current_room_id`.
    fn plan_destination(
        &self,
        content: &ContentPack,
        state: &WorldState,
        actor: &ActorDefinition,
        current_room_id: &str,
    ) -> Option<String>;
}

/// Creates a concrete `MovementStrategy` from a content `WanderDefinition`.
pub(crate) fn strategy_from_wander(wander: &WanderDefinition) -> Box<dyn MovementStrategy> {
    match wander.mode {
        WanderMode::Stay => Box::new(StayStrategy),
        WanderMode::RandomAdjacent => Box::new(RandomAdjacentStrategy::new(wander.cadence_ticks)),
        WanderMode::TowardPlayer => Box::new(TowardPlayerStrategy::new(wander.cadence_ticks)),
        WanderMode::To => Box::new(ToDestinationStrategy::new(
            wander.cadence_ticks,
            wander.room_id.clone(),
        )),
        WanderMode::ExitLabel => Box::new(ExitLabelStrategy::new(
            wander.cadence_ticks,
            wander.exit_label.clone(),
        )),
    }
}

/// Policy governing whether an actor is eligible to wander on this tick.
pub(crate) trait MovementEligibilityPolicy {
    fn is_eligible(
        &self,
        content: &ContentPack,
        state: &WorldState,
        actor: &ActorDefinition,
        scope_room_ids: &Option<BTreeSet<String>>,
    ) -> bool;
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct DefaultMovementEligibilityPolicy;

impl MovementEligibilityPolicy for DefaultMovementEligibilityPolicy {
    fn is_eligible(
        &self,
        content: &ContentPack,
        state: &WorldState,
        actor: &ActorDefinition,
        scope_room_ids: &Option<BTreeSet<String>>,
    ) -> bool {
        if content.is_player_actor(&actor.id) {
            return false;
        }
        let is_hostile = state.stance(&actor.id) == ActorStance::Hostile;
        let is_autonomous_ally =
            state.stance(&actor.id) == ActorStance::Allied && !state.follows_player(&actor.id);
        if !is_hostile && !is_autonomous_ally {
            return false;
        }
        if state.actor_stat(&actor.id, &content.settings.combat.health_stat_id) <= 0 {
            return false;
        }
        let current_room_id = state.actor_room_id(&actor.id, &actor.room_id);
        if !room_is_in_tick_scope(scope_room_ids, current_room_id) {
            return false;
        }
        if should_hold(content, state, &actor.id) {
            return false;
        }
        true
    }
}

/// Resolves the movement strategy for a specific actor.
pub(crate) trait MovementStrategyResolver {
    fn resolve(
        &self,
        content: &ContentPack,
        state: &WorldState,
        actor_id: &str,
    ) -> Option<Box<dyn MovementStrategy>>;
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct DefaultMovementStrategyResolver;

impl MovementStrategyResolver for DefaultMovementStrategyResolver {
    fn resolve(
        &self,
        content: &ContentPack,
        state: &WorldState,
        actor_id: &str,
    ) -> Option<Box<dyn MovementStrategy>> {
        if let Some(order) = state.party_order(content, actor_id) {
            return match order.to_ascii_lowercase().as_str() {
                "guard" | "sentry" | "hold" => Some(Box::new(StayStrategy)),
                "patrol" => {
                    let authored = content
                        .movement
                        .actors
                        .get(actor_id)
                        .and_then(|rules| rules.wander.as_ref());
                    if let Some(wander) = authored {
                        Some(strategy_from_wander(wander))
                    } else {
                        Some(Box::new(RandomAdjacentStrategy::new(1)))
                    }
                }
                "follow" => None,
                _ => None,
            };
        }

        content
            .movement
            .actors
            .get(actor_id)
            .and_then(|rules| rules.wander.as_ref())
            .or(content.movement.defaults.wander.as_ref())
            .map(strategy_from_wander)
    }
}

/// Plans wander moves using default eligibility policy and strategy resolver.
pub(crate) fn plan_wander_moves(content: &ContentPack, state: &WorldState) -> Vec<WorldEvent> {
    plan_wander_moves_with_dependencies(
        content,
        state,
        &DefaultMovementEligibilityPolicy,
        &DefaultMovementStrategyResolver,
    )
}

/// Plans wander moves with injected eligibility policy and strategy resolver.
pub(crate) fn plan_wander_moves_with_dependencies(
    content: &ContentPack,
    state: &WorldState,
    eligibility: &impl MovementEligibilityPolicy,
    resolver: &impl MovementStrategyResolver,
) -> Vec<WorldEvent> {
    if state.phase != GamePhase::Active {
        return Vec::new();
    }
    let scope_room_ids = tick_scope_room_ids(content, state);

    state
        .onstage_actors(content)
        .filter(|actor| eligibility.is_eligible(content, state, actor, &scope_room_ids))
        .filter_map(|actor| {
            let strategy = resolver.resolve(content, state, &actor.id)?;
            let cadence = strategy.cadence_ticks();
            if cadence == 0 || !state.current_time_minutes.is_multiple_of(cadence) {
                return None;
            }
            let current_room_id = state.actor_room_id(&actor.id, &actor.room_id);
            let to_room_id = strategy.plan_destination(content, state, actor, current_room_id)?;
            if to_room_id == current_room_id {
                return None;
            }
            Some(WorldEvent::ActorMoved {
                actor_id: actor.id.clone(),
                from_room_id: current_room_id.to_string(),
                to_room_id,
            })
        })
        .collect()
}

#[cfg(test)]
fn wander_destination(
    content: &ContentPack,
    state: &WorldState,
    actor: &ActorDefinition,
    current_room_id: &str,
    wander: &WanderDefinition,
) -> Option<String> {
    strategy_from_wander(wander).plan_destination(content, state, actor, current_room_id)
}

pub(crate) fn decide_movement(
    content: Arc<ContentPack>,
    state: &WorldState,
    actor: &ActorDefinition,
    rules: &ActorMovementRulesDefinition,
    current_room_id: &str,
    preferred_target_room_id: Option<&str>,
) -> Result<Vec<WorldEvent>, Box<dyn Error>> {
    if should_hold(&content, state, &actor.id) {
        return Ok(vec![]);
    }
    let target_room_id = required_movement_target_room_id(state, rules, current_room_id)
        .or_else(|| preferred_target_room_id.map(str::to_string));
    let Some(target_room_id) = target_room_id else {
        return Ok(vec![]);
    };
    if target_room_id == current_room_id {
        return Ok(vec![]);
    }
    Ok(
        next_room_toward(&content, state, current_room_id, &target_room_id)
            .map(|next_room_id| {
                vec![WorldEvent::ActorMoved {
                    actor_id: actor.id.clone(),
                    from_room_id: current_room_id.to_string(),
                    to_room_id: next_room_id,
                }]
            })
            .unwrap_or_default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::types::{ActorMovementTargetRuleDefinition, MovementTargetBehavior};
    use crate::engine::test_fixtures::minimal_test_pack;

    #[test]
    fn engaged_hostile_does_not_wander_away_from_player_room() {
        let mut content = minimal_test_pack();
        assert!(content.actors.len() >= 2, "fixture needs two actors");
        content.actors[0].room_id = "lounge".to_string();
        content.actors[1].room_id = "kitchen".to_string();
        let engaged_id = content.actors[0].id.clone();
        let roaming_id = content.actors[1].id.clone();
        content
            .movement
            .actors
            .entry(engaged_id.clone())
            .or_default()
            .wander = Some(WanderDefinition {
            mode: WanderMode::RandomAdjacent,
            cadence_ticks: 1,
            room_id: String::new(),
            exit_label: String::new(),
        });
        content
            .movement
            .actors
            .entry(roaming_id.clone())
            .or_default()
            .wander = Some(WanderDefinition {
            mode: WanderMode::RandomAdjacent,
            cadence_ticks: 1,
            room_id: String::new(),
            exit_label: String::new(),
        });
        content.behavior.defaults.hold = Some(serde_json::json!({
            "rule": "effect_table",
            "rule_config": {
                "cases_path": "rules",
                "next_on_match": "continue",
                "next_on_default": "continue",
                "default_payload_template": { "effects": [] }
            },
            "input_overlay": {
                "rules": [{
                    "conditions": [
                        { "path": "actor.stance", "operator": "equal", "value": "hostile" },
                        { "path": "actor.alive", "operator": "equal", "value": true },
                        { "path": "world.in_player_room", "operator": "equal", "value": true }
                    ],
                    "payload_template": { "kind": "hold" }
                }]
            }
        }));
        let mut state = WorldState::new(&content);
        state.current_room_id = "lounge".to_string();
        state.set_stance(&engaged_id, ActorStance::Hostile);
        state.set_stance(&roaming_id, ActorStance::Hostile);
        state
            .actor_stats
            .entry(engaged_id.clone())
            .or_default()
            .insert("hp".to_string(), 5);
        state
            .actor_stats
            .entry(roaming_id.clone())
            .or_default()
            .insert("hp".to_string(), 5);

        let events = plan_wander_moves(&content, &state);

        assert!(!events.iter().any(|event| matches!(
            event,
            WorldEvent::ActorMoved { actor_id, .. } if actor_id == &engaged_id
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            WorldEvent::ActorMoved { actor_id, .. } if actor_id == &roaming_id
        )));
    }

    #[test]
    fn exit_label_wander_follows_named_exit() {
        let content = minimal_test_pack();
        let wander = WanderDefinition {
            mode: WanderMode::ExitLabel,
            cadence_ticks: 1,
            room_id: String::new(),
            exit_label: "kitchen".to_string(),
        };

        let state = WorldState::new(&content);
        let dest = wander_destination(&content, &state, &content.actors[0], "lounge", &wander);
        assert_eq!(dest.as_deref(), Some("kitchen"));
    }

    #[test]
    fn target_rule_stops_when_forbidden_story_var_is_true() {
        let content = minimal_test_pack();
        let mut state = WorldState::new(&content);
        let rules = ActorMovementRulesDefinition {
            target_rules: vec![ActorMovementTargetRuleDefinition {
                target_room_id: "kitchen".to_string(),
                forbidden_story_var: "lane_stopped".to_string(),
                target_behavior: Some(MovementTargetBehavior::Move),
                ..Default::default()
            }],
            ..Default::default()
        };

        assert_eq!(
            required_movement_target_room_id(&state, &rules, "lounge").as_deref(),
            Some("kitchen")
        );

        state.story_vars.set_unchecked("lane_stopped", "true");
        assert_eq!(
            required_movement_target_room_id(&state, &rules, "lounge"),
            None
        );
    }

    #[test]
    fn pathfinding_does_not_cross_locked_story_exit() {
        let mut content = minimal_test_pack();
        let state = WorldState::new(&content);
        content
            .rooms
            .iter_mut()
            .find(|room| room.id == "lounge")
            .expect("lounge exists")
            .exits
            .iter_mut()
            .find(|exit| exit.room_id == "kitchen")
            .expect("kitchen exit exists")
            .requires_story_var = "gate_open".to_string();

        assert_eq!(
            next_room_toward(&content, &state, "lounge", "kitchen"),
            None
        );

        let mut open_state = state;
        open_state.story_vars.set_unchecked("gate_open", "true");
        assert_eq!(
            next_room_toward(&content, &open_state, "lounge", "kitchen").as_deref(),
            Some("kitchen")
        );
    }

    #[test]
    fn patrolling_allied_member_wanders_via_movement_engine() {
        let mut content = minimal_test_pack();
        let ally_id = content.actors[0].id.clone();
        content.actors[0].room_id = "lounge".to_string();

        let mut state = WorldState::new(&content);
        state.current_room_id = "kitchen".to_string();
        state.set_stance(&ally_id, ActorStance::Allied);
        state.set_follows_player(&ally_id, false);
        state
            .party_orders
            .insert(ally_id.clone(), "patrol".to_string());
        state
            .actor_stats
            .entry(ally_id.clone())
            .or_default()
            .insert("hp".to_string(), 10);

        let events = plan_wander_moves(&content, &state);
        assert!(events.iter().any(|event| matches!(
            event,
            WorldEvent::ActorMoved { actor_id, from_room_id, .. }
                if actor_id == &ally_id && from_room_id == "lounge"
        )));
    }

    #[test]
    fn guarding_allied_member_stays_put() {
        let mut content = minimal_test_pack();
        let ally_id = content.actors[0].id.clone();
        content.actors[0].room_id = "lounge".to_string();

        let mut state = WorldState::new(&content);
        state.current_room_id = "kitchen".to_string();
        state.set_stance(&ally_id, ActorStance::Allied);
        state.set_follows_player(&ally_id, false);
        state
            .party_orders
            .insert(ally_id.clone(), "guard".to_string());
        state
            .actor_stats
            .entry(ally_id.clone())
            .or_default()
            .insert("hp".to_string(), 10);

        let events = plan_wander_moves(&content, &state);
        assert!(!events.iter().any(|event| matches!(
            event,
            WorldEvent::ActorMoved { actor_id, .. } if actor_id == &ally_id
        )));
    }

    #[test]
    fn following_allied_member_does_not_wander_independently() {
        let mut content = minimal_test_pack();
        let ally_id = content.actors[0].id.clone();
        content.actors[0].room_id = "lounge".to_string();

        let mut state = WorldState::new(&content);
        state.current_room_id = "kitchen".to_string();
        state.set_stance(&ally_id, ActorStance::Allied);
        state.set_follows_player(&ally_id, true);
        state
            .party_orders
            .insert(ally_id.clone(), "follow".to_string());
        state
            .actor_stats
            .entry(ally_id.clone())
            .or_default()
            .insert("hp".to_string(), 10);

        let events = plan_wander_moves(&content, &state);
        assert!(!events.iter().any(|event| matches!(
            event,
            WorldEvent::ActorMoved { actor_id, .. } if actor_id == &ally_id
        )));
    }

    #[test]
    fn plan_wander_moves_with_injected_dependencies() {
        struct AlwaysEligiblePolicy;
        impl MovementEligibilityPolicy for AlwaysEligiblePolicy {
            fn is_eligible(
                &self,
                _content: &ContentPack,
                _state: &WorldState,
                _actor: &ActorDefinition,
                _scope_room_ids: &Option<BTreeSet<String>>,
            ) -> bool {
                true
            }
        }

        struct FixedDestinationStrategy {
            target: String,
        }
        impl MovementStrategy for FixedDestinationStrategy {
            fn cadence_ticks(&self) -> u32 {
                1
            }
            fn plan_destination(
                &self,
                _content: &ContentPack,
                _state: &WorldState,
                _actor: &ActorDefinition,
                _current_room_id: &str,
            ) -> Option<String> {
                Some(self.target.clone())
            }
        }

        struct InjectedResolver;
        impl MovementStrategyResolver for InjectedResolver {
            fn resolve(
                &self,
                _content: &ContentPack,
                _state: &WorldState,
                _actor_id: &str,
            ) -> Option<Box<dyn MovementStrategy>> {
                Some(Box::new(FixedDestinationStrategy {
                    target: "secret_vault".to_string(),
                }))
            }
        }

        let content = minimal_test_pack();
        let mut state = WorldState::new(&content);
        state.current_time_minutes = 1;

        let events = plan_wander_moves_with_dependencies(
            &content,
            &state,
            &AlwaysEligiblePolicy,
            &InjectedResolver,
        );

        assert!(!events.is_empty());
        assert_eq!(
            events[0],
            WorldEvent::ActorMoved {
                actor_id: content.actors[0].id.clone(),
                from_room_id: content.actors[0].room_id.clone(),
                to_room_id: "secret_vault".to_string(),
            }
        );
    }
}
