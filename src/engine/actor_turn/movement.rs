use crate::content::types::{
    ActorDefinition, ActorMovementRulesDefinition, ActorMovementTargetRuleDefinition, ContentPack,
};
use crate::engine::events::WorldEvent;
use crate::engine::hooks::{pair_state_note, room_candidate_score};
use crate::engine::state::WorldState;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Debug, Clone)]
pub(crate) struct RelationshipMoveTarget {
    pub(crate) actor_name: Option<String>,
    pub(crate) room_id: String,
    pub(crate) social_note: Option<String>,
    priority: i32,
}

pub(crate) fn resolved_movement_rule_target_room_id(
    state: &WorldState,
    rules: &ActorMovementRulesDefinition,
) -> Option<String> {
    rules
        .target_rules
        .iter()
        .find(|rule| movement_target_rule_matches(state, rule))
        .map(|rule| rule.target_room_id.clone())
        .or_else(|| {
            (!rules.default_target_room_id.is_empty()).then(|| rules.default_target_room_id.clone())
        })
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
                    &BTreeMap::new(),
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
        && (rule.required_story_var.is_empty()
            || state.story_vars.contains_key(&rule.required_story_var))
        && (rule.any_active_stage_ids.is_empty()
            || rule
                .any_active_stage_ids
                .iter()
                .any(|stage_id| state.active_objective_stage_ids.contains(stage_id)))
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
            if !visited.insert(exit.room_id.clone()) {
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
