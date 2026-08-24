use crate::content::types::{
    ActorDefinition, ActorMovementRulesDefinition, ActorMovementTargetRuleDefinition, ContentPack,
    MovementTargetBehavior,
};
use crate::engine::actor_tick::decide_movement;
use crate::engine::events::WorldEvent;
use crate::engine::hooks::{pair_state_note, room_candidate_score};
use crate::engine::state::WorldState;
use serde_json::json;
use std::collections::{BTreeSet, VecDeque};
use std::error::Error;
use std::sync::Arc;

use super::symbolic_planner::evaluate_symbolic_boolean_rule;

/// Movement for non-autonomous packs (autonomous_actor_dialogue=false), which skip the
/// full affordance/dialogue pipeline in build_actor_turn and only ever move NPCs.
pub fn run_actor_turn(
    content: Arc<ContentPack>,
    state: &WorldState,
    actor: &ActorDefinition,
    rules: &ActorMovementRulesDefinition,
) -> Result<Vec<WorldEvent>, Box<dyn Error>> {
    if is_actor_movement_locked(
        content.as_ref(),
        state,
        &actor.id,
        &MovementSuppressionContext::default(),
    )? {
        return Ok(Vec::new());
    }
    let current_room_id = state.actor_room_id(&actor.id, &actor.room_id).to_string();
    decide_movement(content, state, actor, rules, &current_room_id, None)
}

/// The reactive-suppression inputs movement.json's `suppress_when` rule can reference,
/// mirroring the affordance availability a turn is already computing for other purposes.
#[derive(Debug, Clone, Default)]
pub(crate) struct MovementSuppressionContext {
    pub(crate) rest_available: bool,
    pub(crate) eat_option_count: usize,
    pub(crate) drink_option_count: usize,
    pub(crate) consume_option_count: usize,
}

/// The single gate for "can this actor move right now": an active stage-lock, being
/// anchored to a room by an explicit stay rule, or movement.json's reactive suppression
/// rule (e.g. too low on stamina). This is the one place to check why an actor isn't moving.
pub(crate) fn is_actor_movement_locked(
    content: &ContentPack,
    state: &WorldState,
    actor_id: &str,
    suppression: &MovementSuppressionContext,
) -> Result<bool, Box<dyn Error>> {
    if content
        .movement
        .stage_locks
        .iter()
        .any(|stage_id| state.active_objective_stage_ids.contains(stage_id))
    {
        return Ok(true);
    }
    let default_room_id = content
        .actor(actor_id)
        .map(|actor| actor.room_id.clone())
        .unwrap_or_default();
    let current_room_id = state.actor_room_id(actor_id, &default_room_id);
    let rules = content.movement_rules(actor_id);
    if actor_is_locked_to_target_room(state, &rules, current_room_id) {
        return Ok(true);
    }
    let Some(config) = content.movement.suppress_when.as_ref() else {
        return Ok(false);
    };
    let actor_stats = state.actor_stats_snapshot(actor_id);
    evaluate_symbolic_boolean_rule(
        config.clone(),
        json!({
            "actor_stats": actor_stats,
            "affordances": {
                "rest": {
                    "available": suppression.rest_available,
                    "option_count": usize::from(suppression.rest_available),
                },
                "eat": {
                    "available": suppression.eat_option_count > 0,
                    "option_count": suppression.eat_option_count,
                },
                "drink": {
                    "available": suppression.drink_option_count > 0,
                    "option_count": suppression.drink_option_count,
                },
                "consume": {
                    "available": suppression.consume_option_count > 0,
                    "option_count": suppression.consume_option_count,
                },
            },
        }),
    )
}

#[derive(Debug, Clone)]
pub(crate) struct RelationshipMoveTarget {
    pub(crate) actor_name: Option<String>,
    pub(crate) room_id: String,
    pub(crate) social_note: Option<String>,
    priority: i32,
}

pub(crate) fn required_movement_target_room_id(
    state: &WorldState,
    rules: &ActorMovementRulesDefinition,
    current_room_id: &str,
) -> Option<String> {
    let target = resolved_movement_target(state, rules)?;
    if target.room_id == current_room_id {
        None
    } else {
        Some(target.room_id)
    }
}

pub(crate) fn planned_move_target_room_id(move_events: &[WorldEvent]) -> Option<&str> {
    move_events.iter().find_map(|event| match event {
        WorldEvent::ActorMoved { to_room_id, .. } => Some(to_room_id.as_str()),
        WorldEvent::ActorCommandUsed {
            command_id,
            target_room_id: Some(target_room_id),
            ..
        } if command_id == "move" => Some(target_room_id.as_str()),
        _ => None,
    })
}

pub(crate) fn pair_stats_move_target(
    content: &ContentPack,
    state: &WorldState,
    actor: &ActorDefinition,
    current_room_id: &str,
) -> Option<RelationshipMoveTarget> {
    content
        .actors
        .iter()
        .filter(|candidate| candidate.id != actor.id)
        .filter_map(|candidate| {
            let candidate_room_id = state.actor_room_id(&candidate.id, &candidate.room_id);
            if candidate_room_id == current_room_id {
                return None;
            }
            let priority = room_candidate_score(
                content,
                state,
                &actor.id,
                &candidate.id,
                current_room_id,
                candidate_room_id,
            );
            if priority <= 0 {
                return None;
            }
            Some(RelationshipMoveTarget {
                actor_name: Some(candidate.name.clone()),
                room_id: candidate_room_id.to_string(),
                social_note: pair_state_note(
                    content,
                    state,
                    &actor.id,
                    &candidate.id,
                    &candidate.name,
                ),
                priority,
            })
        })
        .max_by(|left, right| {
            left.priority
                .cmp(&right.priority)
                .then_with(|| left.actor_name.cmp(&right.actor_name).reverse())
        })
}

pub(crate) fn exploration_move_target(
    content: &ContentPack,
    state: &WorldState,
    actor: &ActorDefinition,
    current_room_id: &str,
) -> Option<RelationshipMoveTarget> {
    let target_room_id = nearest_unvisited_room(content, state, actor, current_room_id)?;
    let target_room = content.room(&target_room_id)?;
    Some(RelationshipMoveTarget {
        actor_name: None,
        room_id: target_room_id,
        social_note: Some(
            content
                .system_text
                .exploration_unvisited_room_note_template
                .replace("{room_title}", target_room.title.as_str()),
        ),
        priority: 0,
    })
}

fn movement_target_rule_matches(
    state: &WorldState,
    rule: &ActorMovementTargetRuleDefinition,
) -> bool {
    (rule.when_player_room_id.is_empty() || rule.when_player_room_id == state.current_room_id)
        && (rule.required_story_var.is_empty() || state.story_vars.has(&rule.required_story_var))
        && (rule.any_active_stage_ids.is_empty()
            || rule
                .any_active_stage_ids
                .iter()
                .any(|stage_id| state.active_objective_stage_ids.contains(stage_id)))
}

#[derive(Debug, Clone)]
struct ResolvedMovementTarget {
    room_id: String,
    behavior: MovementTargetBehavior,
}

fn resolved_movement_target(
    state: &WorldState,
    rules: &ActorMovementRulesDefinition,
) -> Option<ResolvedMovementTarget> {
    let rule = rules
        .target_rules
        .iter()
        .find(|rule| movement_target_rule_matches(state, rule))?;
    let room_id = if !rule.target_from_story_var.is_empty() {
        state
            .story_vars
            .get(&rule.target_from_story_var)?
            .to_string()
    } else {
        rule.target_room_id.clone()
    };
    let behavior = rule.target_behavior?;
    if room_id.is_empty() {
        None
    } else {
        Some(ResolvedMovementTarget { room_id, behavior })
    }
}

fn actor_is_locked_to_target_room(
    state: &WorldState,
    rules: &ActorMovementRulesDefinition,
    current_room_id: &str,
) -> bool {
    matches!(
        resolved_movement_target(state, rules),
        Some(ResolvedMovementTarget {
            room_id,
            behavior: MovementTargetBehavior::Stay,
        }) if room_id == current_room_id
    )
}

fn nearest_unvisited_room(
    content: &ContentPack,
    state: &WorldState,
    actor: &ActorDefinition,
    current_room_id: &str,
) -> Option<String> {
    let mut queue = VecDeque::from([(current_room_id.to_string(), None::<String>)]);
    let mut visited = BTreeSet::from([current_room_id.to_string()]);

    while let Some((room_id, first_step)) = queue.pop_front() {
        let room = content.room(&room_id)?;
        for exit in &room.exits {
            if !content.room_is_reachable(&exit.room_id) || !visited.insert(exit.room_id.clone()) {
                continue;
            }
            let candidate_first_step = first_step.clone().unwrap_or_else(|| exit.room_id.clone());
            if !state.actor_has_visited_room(&actor.id, &exit.room_id) {
                return Some(candidate_first_step);
            }
            queue.push_back((exit.room_id.clone(), Some(candidate_first_step)));
        }
    }
    None
}
