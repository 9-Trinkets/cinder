use super::{room_is_in_tick_scope, tick_scope_room_ids};
use crate::content::types::{
    ActorDefinition, ActorMovementRulesDefinition, ContentPack, WanderDefinition, WanderMode,
};
use crate::engine::actor_turn::movement::required_movement_target_room_id;
use crate::engine::behavior::should_hold;
use crate::engine::events::WorldEvent;
use crate::engine::state::{ActorStance, GamePhase, WorldState};
use rand::Rng;
use std::collections::{BTreeSet, VecDeque};
use std::error::Error;
use std::sync::Arc;

pub(crate) fn plan_wander_moves(content: &ContentPack, state: &WorldState) -> Vec<WorldEvent> {
    if state.phase != GamePhase::Active {
        return Vec::new();
    }
    let scope_room_ids = tick_scope_room_ids(content, state);
    let mut events = Vec::new();
    for actor in content.onstage_actors() {
        if content.is_player_actor(&actor.id) {
            continue;
        }
        let Some(wander) = resolve_wander(content, &actor.id) else {
            continue;
        };
        if wander.cadence_ticks == 0 {
            continue;
        }
        if state.stance(&actor.id) != ActorStance::Hostile {
            continue;
        }
        if state.actor_stat(&actor.id, &content.settings.combat.health_stat_id) <= 0 {
            continue;
        }
        if !room_is_in_tick_scope(
            &scope_room_ids,
            state.actor_room_id(&actor.id, &actor.room_id),
        ) {
            continue;
        }
        if !state
            .current_time_minutes
            .is_multiple_of(wander.cadence_ticks)
        {
            continue;
        }
        let current_room_id = state.actor_room_id(&actor.id, &actor.room_id);
        if should_hold(content, state, &actor.id) {
            continue;
        }
        let Some(to_room_id) = wander_destination(content, state, actor, current_room_id, &wander)
        else {
            continue;
        };
        if to_room_id == current_room_id {
            continue;
        }
        events.push(WorldEvent::ActorMoved {
            actor_id: actor.id.clone(),
            from_room_id: current_room_id.to_string(),
            to_room_id,
        });
    }
    events
}

fn resolve_wander(content: &ContentPack, actor_id: &str) -> Option<WanderDefinition> {
    content
        .movement
        .actors
        .get(actor_id)
        .and_then(|rules| rules.wander.clone())
        .or_else(|| content.movement.defaults.wander.clone())
}

fn wander_destination(
    content: &ContentPack,
    state: &WorldState,
    actor: &ActorDefinition,
    current_room_id: &str,
    wander: &WanderDefinition,
) -> Option<String> {
    match wander.mode {
        WanderMode::RandomAdjacent => {
            let neighbors = content.adjacent_room_ids(current_room_id);
            if neighbors.is_empty() {
                return None;
            }
            let index = rand::thread_rng().gen_range(0..neighbors.len());
            Some(neighbors[index].clone())
        }
        WanderMode::TowardPlayer => {
            next_room_toward(content, current_room_id, &state.current_room_id)
        }
        WanderMode::Stay => None,
        WanderMode::To => {
            let destination = if wander.room_id.is_empty() {
                actor.room_id.clone()
            } else {
                wander.room_id.clone()
            };
            next_room_toward(content, current_room_id, &destination)
        }
    }
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
    Ok(next_room_toward(&content, current_room_id, &target_room_id)
        .map(|next_room_id| {
            vec![WorldEvent::ActorMoved {
                actor_id: actor.id.clone(),
                from_room_id: current_room_id.to_string(),
                to_room_id: next_room_id,
            }]
        })
        .unwrap_or_default())
}

fn next_room_toward(
    content: &ContentPack,
    current_room_id: &str,
    target_room_id: &str,
) -> Option<String> {
    if current_room_id.is_empty() || target_room_id.is_empty() || current_room_id == target_room_id
    {
        return None;
    }
    let mut queue = VecDeque::from([(current_room_id.to_string(), None::<String>)]);
    let mut visited = BTreeSet::from([current_room_id.to_string()]);

    while let Some((room_id, first_step)) = queue.pop_front() {
        let room = content.room(&room_id)?;
        for exit in &room.exits {
            let is_target = exit.room_id == target_room_id;
            if (!is_target && !content.room_is_reachable(&exit.room_id))
                || !visited.insert(exit.room_id.clone())
            {
                continue;
            }
            let candidate_first_step = first_step.clone().unwrap_or_else(|| exit.room_id.clone());
            if is_target {
                return Some(candidate_first_step);
            }
            queue.push_back((exit.room_id.clone(), Some(candidate_first_step)));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
