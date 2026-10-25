//! Evaluates [`Condition`]s: the single gate used by actions, exits, room
//! descriptions, topics, stage signals and everything else that depends on the
//! state of the world.

use crate::content::types::{Condition, ContentPack, ItemStorageTarget, PathCondition};
use crate::engine::state::WorldState;

/// A story variable value counts as set when it is not empty, `false` or `0`.
pub fn story_value_is_truthy(value: &str) -> bool {
    !matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "" | "false" | "0"
    )
}

/// Whether a story variable exists and holds a truthy value.
pub fn story_var_is_truthy(state: &WorldState, key: &str) -> bool {
    state.story_vars.get(key).is_some_and(story_value_is_truthy)
}

/// Where a condition is being asked. `room_id` is the room the question is about:
/// the player's room, or an actor's own room, or the room an exit leaves from.
pub struct ConditionContext<'a> {
    pub content: &'a ContentPack,
    pub state: &'a WorldState,
    pub room_id: &'a str,
}

impl<'a> ConditionContext<'a> {
    pub fn new(content: &'a ContentPack, state: &'a WorldState, room_id: &'a str) -> Self {
        Self {
            content,
            state,
            room_id,
        }
    }
}

/// Whether every condition holds. An empty list holds.
pub fn all_hold(conditions: &[Condition], ctx: &ConditionContext<'_>) -> bool {
    conditions.iter().all(|condition| holds(condition, ctx))
}

/// Whether one condition holds.
pub fn holds(condition: &Condition, ctx: &ConditionContext<'_>) -> bool {
    match condition {
        Condition::StoryVar(name) => story_var_is_truthy(ctx.state, name),
        Condition::StoryVarIs { name, value } => ctx.state.story_vars.get(name) == Some(value),
        Condition::Carries(item) => ctx.state.has_item(item),
        Condition::ItemHere(item) => {
            ctx.state
                .has_item_in_storage(item, ItemStorageTarget::CurrentRoom, ctx.room_id)
        }
        Condition::TraceMarkHere => {
            ctx.state
                .loose_room_items(ctx.room_id)
                .into_iter()
                .any(|(item_id, count)| {
                    count > 0 && ctx.content.item(&item_id).is_some_and(|i| i.trace_mark)
                })
        }
        Condition::Defeated(actor) => ctx
            .state
            .actor_is_defeated(actor, &ctx.content.settings.combat.health_stat_id),
        Condition::Not(inner) => !holds(inner, ctx),
        Condition::Path(path) => path_holds(path, &world_view(ctx.state)),
    }
}

/// The state of the world as JSON, for conditions written as paths.
pub fn world_view(state: &WorldState) -> serde_json::Value {
    serde_json::json!({
        "current_time_minutes": state.clock.current_time_minutes,
        "current_room_id": state.current_room_id,
        "active_stage_ids": state.active_objective_stage_ids,
        "stages_completed": state.stages_completed,
        "actor_stats": state.combat.actor_stats,
        "pair_stats": state.combat.pair_stats,
        "story_vars": state.story_vars,
        "inventory": state.inventory.player_inventory,
    })
}

/// Evaluates a path condition against a JSON view.
pub fn path_holds(cond: &PathCondition, view: &serde_json::Value) -> bool {
    let actual = resolve_path(view, &cond.path);
    match cond.operator.as_str() {
        "equal" => actual == Some(&cond.value),
        "greater_than" => compare_numbers(&actual, &cond.value, |a, b| a > b),
        "less_than" => compare_numbers(&actual, &cond.value, |a, b| a < b),
        "gte" => compare_numbers(&actual, &cond.value, |a, b| a >= b),
        "lte" => compare_numbers(&actual, &cond.value, |a, b| a <= b),
        "not_equal" => actual != Some(&cond.value),
        "array_contains" => actual
            .and_then(|v| v.as_array())
            .map(|arr| arr.contains(&cond.value))
            .unwrap_or(false),
        "starts_with" => match (actual.and_then(|v| v.as_str()), cond.value.as_str()) {
            (Some(s), Some(p)) => s.starts_with(p),
            _ => false,
        },
        "ends_with" => match (actual.and_then(|v| v.as_str()), cond.value.as_str()) {
            (Some(s), Some(p)) => s.ends_with(p),
            _ => false,
        },
        "contains" => match (actual.and_then(|v| v.as_str()), cond.value.as_str()) {
            (Some(s), Some(p)) => s.contains(p),
            _ => false,
        },
        _ => false,
    }
}

fn resolve_path<'a>(value: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    path.split('.')
        .try_fold(value, |acc, key| acc.as_object()?.get(key))
}

fn compare_numbers(
    actual: &Option<&serde_json::Value>,
    expected: &serde_json::Value,
    cmp: impl Fn(f64, f64) -> bool,
) -> bool {
    let Some(a) = actual.and_then(|v| v.as_f64()) else {
        return false;
    };
    let Some(b) = expected.as_f64() else {
        return false;
    };
    cmp(a, b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::test_fixtures::minimal_test_pack;

    fn path(path: &str, operator: &str, value: serde_json::Value) -> PathCondition {
        PathCondition {
            path: path.to_string(),
            operator: operator.to_string(),
            value,
        }
    }

    #[test]
    fn truthiness_is_one_rule() {
        for falsy in ["", "false", "FALSE", " 0 "] {
            assert!(!story_value_is_truthy(falsy), "{falsy:?}");
        }
        for truthy in ["true", "1", "yes", "night"] {
            assert!(story_value_is_truthy(truthy), "{truthy:?}");
        }
    }

    #[test]
    fn an_empty_list_holds() {
        let pack = minimal_test_pack();
        let state = WorldState::new(&pack);
        assert!(all_hold(&[], &ConditionContext::new(&pack, &state, "x")));
    }

    #[test]
    fn story_variables_items_and_negation() {
        let pack = minimal_test_pack();
        let mut state = WorldState::new(&pack);
        state.story_vars.set_unchecked("door_open", "true");
        state.story_vars.set_unchecked("phase", "night");
        state.add_item("key");
        let ctx = ConditionContext::new(&pack, &state, "x");
        let check = |text: &str| holds(&Condition::parse(text).unwrap(), &ctx);
        assert!(check("door_open"));
        assert!(!check("!door_open"));
        assert!(!check("gate_open"));
        assert!(check("!gate_open"));
        assert!(check("phase=night"));
        assert!(!check("phase=day"));
        assert!(check("item:key"));
        assert!(!check("item:coin"));
        assert!(!check("here:key"));
    }

    #[test]
    fn a_story_variable_set_to_false_is_not_set() {
        let pack = minimal_test_pack();
        let mut state = WorldState::new(&pack);
        state.story_vars.set_unchecked("open", "false");
        let ctx = ConditionContext::new(&pack, &state, "x");
        assert!(!holds(&Condition::parse("open").unwrap(), &ctx));
        assert!(holds(&Condition::parse("!open").unwrap(), &ctx));
    }

    #[test]
    fn path_conditions_compare_numbers_and_text() {
        let input = serde_json::json!({ "score": 5, "room": "courtyard_center" });
        assert!(path_holds(&path("score", "gte", 4.into()), &input));
        assert!(!path_holds(&path("score", "gte", 6.into()), &input));
        assert!(path_holds(
            &path("room", "starts_with", "courtyard_".into()),
            &input
        ));
        assert!(!path_holds(
            &path("room", "starts_with", "village_".into()),
            &input
        ));
        assert!(!path_holds(&path("missing", "gte", 0.into()), &input));
    }
}
