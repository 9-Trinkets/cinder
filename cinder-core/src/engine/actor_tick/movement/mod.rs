pub(crate) mod pathfinding;
pub(crate) mod strategies;

use super::{room_is_in_tick_scope, tick_scope_room_ids};
use crate::content::types::{ActorDefinition, ContentPack, MovementOrderDefinition};
use crate::engine::behavior::should_hold;
use crate::engine::combat_targets::room_has_living_guard;
use crate::engine::events::WorldEvent;
use crate::engine::orders::{OrderKind, parse_order};
use crate::engine::state::{ActorStance, GamePhase, WorldState};
use std::collections::BTreeSet;
use std::error::Error;
use std::sync::Arc;

use crate::content::types::ActorMovementRulesDefinition;
use crate::engine::actor_turn::movement::required_movement_target_room_id;

pub(crate) use pathfinding::next_room_toward;
pub(crate) use strategies::{
    DeployStrategy, ExitLabelStrategy, RandomAdjacentStrategy, StayStrategy, ToDestinationStrategy,
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

/// Creates a concrete `MovementStrategy` from a content `MovementOrderDefinition`.
/// A single place maps an order token to a strategy; a new movement behavior is a
/// new token in `engine::orders` plus a new arm here.
pub(crate) fn strategy_from_order(
    order: &MovementOrderDefinition,
) -> Option<Box<dyn MovementStrategy>> {
    let cadence = order.cadence_ticks;
    match parse_order(&order.token)? {
        OrderKind::Hold => Some(Box::new(StayStrategy)),
        OrderKind::Patrol => Some(Box::new(RandomAdjacentStrategy::new(cadence))),
        OrderKind::Hunt => Some(Box::new(TowardPlayerStrategy::new(cadence))),
        OrderKind::Follow => None,
        OrderKind::Deploy(target) => Some(Box::new(DeployStrategy::new(
            cadence,
            order_target(order, target),
        ))),
        OrderKind::Goto(target) => Some(Box::new(ToDestinationStrategy::new(
            cadence,
            order_target(order, target),
        ))),
        OrderKind::Exit(label) => {
            let label = if label.is_empty() {
                order.exit_label.clone()
            } else {
                label
            };
            Some(Box::new(ExitLabelStrategy::new(cadence, label)))
        }
    }
}

/// The target for a target-bearing order, preferring an inline token target and
/// falling back to the authored `target_room_id`.
fn order_target(order: &MovementOrderDefinition, inline: String) -> String {
    if inline.is_empty() {
        order.target_room_id.clone()
    } else {
        inline
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
        if state.followed_actor(&actor.id).is_some() {
            return false;
        }
        if state.actor_stat(&actor.id, &content.settings.combat.health_stat_id) <= 0 {
            return false;
        }
        let current_room_id = state.actor_room_id(&actor.id, &actor.room_id);
        if !room_is_in_tick_scope(scope_room_ids, current_room_id) {
            return false;
        }
        if is_hostile && room_has_living_guard(content, state, current_room_id) {
            return false;
        }
        if should_hold(content, state, &actor.id) {
            return false;
        }
        if state.actor_is_imprisoned(content, &actor.id) {
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

fn has_unjoined_followers(content: &ContentPack, state: &WorldState, actor_id: &str) -> bool {
    let Some(party) = state.party_of_actor(actor_id) else {
        return false;
    };
    if party.is_player_controlled() || party.leader.as_deref() != Some(actor_id) {
        return false;
    }
    let leader_room = state.actor_current_room_id(content, actor_id);
    if leader_room.is_empty() {
        return false;
    }
    let health_stat_id = if content.settings.combat.health_stat_id.is_empty() {
        "hp"
    } else {
        &content.settings.combat.health_stat_id
    };
    let leader_stance = state.stance(actor_id);
    let followers = state.followers_of(content, actor_id);
    followers.iter().any(|follower_id| {
        state.stance(follower_id) == leader_stance
            && !state.actor_is_offstage(content, follower_id)
            && !state.actor_is_defeated(follower_id, health_stat_id)
            && !state.actor_is_imprisoned(content, follower_id)
            && state.actor_current_room_id(content, follower_id) != leader_room
    })
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
        if state.followed_actor(actor_id).is_some() {
            return None;
        }
        if has_unjoined_followers(content, state, actor_id) {
            return Some(Box::new(StayStrategy));
        }
        if let Some(order) = state.party_order(content, actor_id) {
            let authored = content
                .movement
                .actors
                .get(actor_id)
                .and_then(|rules| rules.order.as_ref());
            return match parse_order(&order) {
                Some(OrderKind::Hold) => Some(Box::new(StayStrategy)),
                Some(OrderKind::Patrol) => authored
                    .and_then(strategy_from_order)
                    .or_else(|| Some(Box::new(RandomAdjacentStrategy::new(1)))),
                Some(OrderKind::Follow) => {
                    // Player-party followers are driven by the follow-player
                    // system. For AI parties, "follow" tracks the party leader.
                    let leader = state
                        .party_of_actor(actor_id)
                        .filter(|party| !party.is_player_controlled())
                        .and_then(|party| party.leader.clone());
                    let Some(leader) = leader.filter(|leader| leader != actor_id) else {
                        return None;
                    };
                    let leader_room = state.actor_current_room_id(content, &leader);
                    if leader_room.is_empty() {
                        return None;
                    }
                    Some(Box::new(ToDestinationStrategy::new(
                        1,
                        leader_room.to_string(),
                    )))
                }
                Some(OrderKind::Hunt) => {
                    let cadence = authored.map_or(1, |order| order.cadence_ticks);
                    Some(Box::new(TowardPlayerStrategy::new(cadence)))
                }
                Some(OrderKind::Deploy(target)) => {
                    let target = if target.is_empty() {
                        authored
                            .map(|order| order.target_room_id.clone())
                            .unwrap_or_default()
                    } else {
                        target
                    };
                    Some(Box::new(DeployStrategy::new(1, target)))
                }
                Some(OrderKind::Goto(target)) => {
                    let cadence = authored.map_or(1, |order| order.cadence_ticks);
                    let target = if target.is_empty() {
                        authored
                            .map(|order| order.target_room_id.clone())
                            .unwrap_or_default()
                    } else {
                        target
                    };
                    Some(Box::new(ToDestinationStrategy::new(cadence, target)))
                }
                Some(OrderKind::Exit(label)) => {
                    let cadence = authored.map_or(1, |order| order.cadence_ticks);
                    let label = if label.is_empty() {
                        authored
                            .map(|order| order.exit_label.clone())
                            .unwrap_or_default()
                    } else {
                        label
                    };
                    Some(Box::new(ExitLabelStrategy::new(cadence, label)))
                }
                _ => None,
            };
        }

        content
            .movement
            .actors
            .get(actor_id)
            .and_then(|rules| rules.order.as_ref())
            .or(content.movement.defaults.order.as_ref())
            .and_then(strategy_from_order)
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
            if cadence == 0 || !state.clock.current_time_minutes.is_multiple_of(cadence) {
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
fn order_destination(
    content: &ContentPack,
    state: &WorldState,
    actor: &ActorDefinition,
    current_room_id: &str,
    order: &MovementOrderDefinition,
) -> Option<String> {
    strategy_from_order(order)?.plan_destination(content, state, actor, current_room_id)
}

pub(crate) fn decide_movement(
    content: Arc<ContentPack>,
    state: &WorldState,
    actor: &ActorDefinition,
    rules: &ActorMovementRulesDefinition,
    current_room_id: &str,
    preferred_target_room_id: Option<&str>,
) -> Result<Vec<WorldEvent>, Box<dyn Error>> {
    let hold = should_hold(&content, state, &actor.id);
    let guard = state.stance(&actor.id) == ActorStance::Hostile
        && room_has_living_guard(&content, state, current_room_id);
    let followed = state.followed_actor(&actor.id).is_some();
    if hold {
        return Ok(vec![]);
    }
    if guard {
        return Ok(vec![]);
    }
    if followed {
        return Ok(vec![]);
    }
    let target_room_id = if let Some(order) = state.party_order(&content, &actor.id)
        && let Some(OrderKind::Deploy(target)) = parse_order(&order)
    {
        let target = target.trim();
        if !target.is_empty() && content.room(target).is_some() {
            Some(target.to_string())
        } else {
            None
        }
    } else {
        required_movement_target_room_id(state, rules, current_room_id)
            .or_else(|| {
                let content_rules = content.movement_rules(&actor.id);
                required_movement_target_room_id(state, &content_rules, current_room_id)
            })
            .or_else(|| preferred_target_room_id.map(str::to_string))
    };
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
    fn order_tokens_map_to_strategies_through_one_registry() {
        let plan = |token: &str, cadence: u32| {
            let order = MovementOrderDefinition {
                token: token.to_string(),
                cadence_ticks: cadence,
                ..Default::default()
            };
            strategy_from_order(&order).map(|strategy| strategy.cadence_ticks())
        };
        assert_eq!(plan("hold", 9), Some(0));
        assert_eq!(plan("guard", 9), Some(0));
        assert_eq!(plan("patrol", 3), Some(3));
        assert_eq!(plan("wander", 2), Some(2));
        assert_eq!(plan("advance", 2), Some(2));
        assert_eq!(plan("follow", 1), None);
        assert_eq!(plan("mystery", 1), None);
    }

    #[test]
    fn ai_party_follow_order_tracks_the_leader() {
        use crate::content::types::{PartyController, PartyDefinition};
        let mut content = minimal_test_pack();
        let follower = content.actors[0].id.clone();
        let mut leader = content.actors[1].clone();
        leader.id = "chief".to_string();
        leader.name = "Chief".to_string();
        leader.room_id = "kitchen".to_string();
        content.actors.push(leader);
        content.actors[0].room_id = "lounge".to_string();
        content.settings.parties = vec![PartyDefinition {
            id: "army".to_string(),
            controller: PartyController::Ai,
            leader: Some("chief".to_string()),
            members: vec!["chief".to_string(), follower.clone()],
            initial_orders: std::collections::BTreeMap::from([(
                follower.clone(),
                "follow".to_string(),
            )]),
            ..PartyDefinition::default()
        }];
        crate::engine::test_fixtures::rebuild_test_pack_indexes(&mut content);

        let state = WorldState::new(&content);
        let strategy = DefaultMovementStrategyResolver
            .resolve(&content, &state, &follower)
            .expect("follow resolves to a leader-tracking strategy");
        let actor = content.actor(&follower).unwrap();
        assert_eq!(
            strategy.plan_destination(&content, &state, actor, "lounge"),
            Some("kitchen".to_string())
        );
    }

    #[test]
    fn ai_party_advance_order_chases_the_player() {
        use crate::content::types::{PartyController, PartyDefinition};
        let mut content = minimal_test_pack();
        let rusher = content.actors[1].id.clone();
        content.actors[1].room_id = "lounge".to_string();
        content
            .movement
            .actors
            .entry(rusher.clone())
            .or_default()
            .order = Some(MovementOrderDefinition {
            token: "patrol".to_string(),
            cadence_ticks: 3,
            ..Default::default()
        });
        content.settings.parties = vec![PartyDefinition {
            id: "army".to_string(),
            controller: PartyController::Ai,
            members: vec![rusher.clone()],
            initial_orders: std::collections::BTreeMap::from([(
                rusher.clone(),
                "advance".to_string(),
            )]),
            ..PartyDefinition::default()
        }];
        crate::engine::test_fixtures::rebuild_test_pack_indexes(&mut content);

        let mut state = WorldState::new(&content);
        state.current_room_id = "kitchen".to_string();
        let strategy = DefaultMovementStrategyResolver
            .resolve(&content, &state, &rusher)
            .expect("advance resolves to a chase strategy");
        assert_eq!(strategy.cadence_ticks(), 3);
        let actor = content.actor(&rusher).unwrap();
        assert_eq!(
            strategy.plan_destination(&content, &state, actor, "lounge"),
            Some("kitchen".to_string())
        );
    }

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
            .order = Some(MovementOrderDefinition {
            token: "patrol".to_string(),
            cadence_ticks: 1,
            ..Default::default()
        });
        content
            .movement
            .actors
            .entry(roaming_id.clone())
            .or_default()
            .order = Some(MovementOrderDefinition {
            token: "patrol".to_string(),
            cadence_ticks: 1,
            ..Default::default()
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
            .combat
            .actor_stats
            .entry(engaged_id.clone())
            .or_default()
            .insert("hp".to_string(), 5);
        state
            .combat
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
        let order = MovementOrderDefinition {
            token: "exit".to_string(),
            cadence_ticks: 1,
            exit_label: "kitchen".to_string(),
            ..Default::default()
        };
        let state = WorldState::new(&content);
        let dest = order_destination(&content, &state, &content.actors[0], "lounge", &order);
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
    fn living_guard_blocks_hostile_target_rule_movement() {
        let mut content = minimal_test_pack();
        let hostile_id = content.actors[0].id.clone();
        let guard_id = content.actors[1].id.clone();
        content.actors[0].room_id = "lounge".to_string();
        content.actors[1].room_id = "lounge".to_string();
        let rules = ActorMovementRulesDefinition {
            target_rules: vec![ActorMovementTargetRuleDefinition {
                target_room_id: "kitchen".to_string(),
                target_behavior: Some(MovementTargetBehavior::Move),
                ..Default::default()
            }],
            ..Default::default()
        };

        let mut state = WorldState::new(&content);
        state.set_stance(&hostile_id, ActorStance::Hostile);
        state.set_stance(&guard_id, ActorStance::Allied);
        state.set_follows_player(&guard_id, false);
        state
            .relations
            .party_orders
            .insert(guard_id.clone(), "guard".to_string());
        state
            .combat
            .actor_stats
            .entry(hostile_id.clone())
            .or_default()
            .insert("hp".to_string(), 10);
        state
            .combat
            .actor_stats
            .entry(guard_id.clone())
            .or_default()
            .insert("hp".to_string(), 10);

        let blocked = decide_movement(
            Arc::new(content.clone()),
            &state,
            &content.actors[0],
            &rules,
            "lounge",
            None,
        )
        .expect("movement resolves");
        assert!(blocked.is_empty());

        state
            .combat
            .actor_stats
            .entry(guard_id)
            .or_default()
            .insert("hp".to_string(), 0);
        let resumed = decide_movement(
            Arc::new(content.clone()),
            &state,
            &content.actors[0],
            &rules,
            "lounge",
            None,
        )
        .expect("movement resolves");
        assert!(matches!(
            resumed.as_slice(),
            [WorldEvent::ActorMoved {
                actor_id,
                from_room_id,
                to_room_id,
            }] if actor_id == &hostile_id
                && from_room_id == "lounge"
                && to_room_id == "kitchen"
        ));
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
            .relations
            .party_orders
            .insert(ally_id.clone(), "patrol".to_string());
        state
            .combat
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
            .relations
            .party_orders
            .insert(ally_id.clone(), "guard".to_string());
        state
            .combat
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
    fn living_guard_blocks_hostile_movement_until_no_longer_defending_room() {
        let mut content = minimal_test_pack();
        let hostile_id = content.actors[0].id.clone();
        let guard_id = content.actors[1].id.clone();
        content.actors[0].room_id = "lounge".to_string();
        content.actors[1].room_id = "lounge".to_string();

        let mut state = WorldState::new(&content);
        state.set_stance(&hostile_id, ActorStance::Hostile);
        state.set_stance(&guard_id, ActorStance::Allied);
        state.set_follows_player(&guard_id, false);
        state
            .relations
            .party_orders
            .insert(guard_id.clone(), "guard".to_string());
        state
            .combat
            .actor_stats
            .entry(hostile_id.clone())
            .or_default()
            .insert("hp".to_string(), 10);
        state
            .combat
            .actor_stats
            .entry(guard_id.clone())
            .or_default()
            .insert("hp".to_string(), 10);

        let policy = DefaultMovementEligibilityPolicy;
        assert!(!policy.is_eligible(&content, &state, &content.actors[0], &None));

        state
            .combat
            .actor_stats
            .entry(guard_id.clone())
            .or_default()
            .insert("hp".to_string(), 0);
        assert!(policy.is_eligible(&content, &state, &content.actors[0], &None));

        state
            .combat
            .actor_stats
            .entry(guard_id.clone())
            .or_default()
            .insert("hp".to_string(), 10);
        state
            .actors
            .actor_room_overrides
            .insert(guard_id, "kitchen".to_string());
        assert!(policy.is_eligible(&content, &state, &content.actors[0], &None));
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
            .relations
            .party_orders
            .insert(ally_id.clone(), "follow".to_string());
        state
            .combat
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
        state.clock.current_time_minutes = 1;

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
