//! Content-driven hostile behavior decisions during a background tick.
//!
//! Non-skill movement holds and compatibility hostile strikes are declared in
//! `behavior.json` as neuron `effect_table` rules evaluated against a shared
//! actor/world input JSON:
//!
//! - `strike`: compatibility path for packs without centralized skills.
//! - `hold`: yields `{ "kind": "hold" }` when the actor must stay put
//!   (blocking movement); an empty result means it is free to move.
//!
//! Strict skill packs author autonomous skill eligibility in `skills.json`.
//! Movement destination and cadence remain in `movement.json`.

use crate::content::types::{BehaviorActorDefinition, ContentPack};
use crate::engine::combat_targets::select_hostile_target;
use crate::engine::events::WorldEvent;
use crate::engine::neuron::evaluate_symbolic_value;
use crate::engine::state::{ActorStance, WorldState};
use serde_json::{Value, json};

/// The actor/world context every behavior rule is evaluated against. Keeping a
/// single shared shape means pack authors can express all conditions over the
/// same fields, and adding context here automatically enriches every rule.
pub(crate) fn build_input(content: &ContentPack, state: &WorldState, actor_id: &str) -> Value {
    let stance = state.stance(actor_id);
    let hp = state.actor_stat(actor_id, &content.settings.combat.health_stat_id);
    let default_room_id = content
        .actor(actor_id)
        .map(|actor| actor.room_id.clone())
        .unwrap_or_default();
    let room_id = state.actor_room_id(actor_id, &default_room_id);
    let current_time_minutes = state.current_time_minutes;
    let cooldown_elapsed = state
        .next_hostile_strike_at
        .get(actor_id)
        .is_none_or(|next_strike| current_time_minutes >= *next_strike);
    let opposing_party_actor_in_room = select_hostile_target(content, state, actor_id).is_some();

    json!({
        "actor": {
            "id": actor_id,
            "stance": match stance { ActorStance::Hostile => "hostile", _ => "neutral" },
            "alive": hp > 0,
            "hp": hp,
            "in_player_room": room_id == state.current_room_id,
            "room_id": room_id,
        },
        "world": {
            "in_player_room": room_id == state.current_room_id,
            "opposing_party_actor_in_room": opposing_party_actor_in_room,
            "cooldown_elapsed": cooldown_elapsed,
            "time": state.current_time_minutes,
        }
    })
}

/// Resolve the effective behavior rules for an actor: per-actor overrides on
/// top of the pack-wide defaults. Fields left unset inherit the default.
fn resolved_behavior(content: &ContentPack, actor_id: &str) -> BehaviorActorDefinition {
    content
        .behavior
        .actors
        .get(actor_id)
        .cloned()
        .unwrap_or_default()
        .resolved_with_default(&content.behavior.defaults)
}

enum Decision {
    Yes,
    No,
}

/// Evaluate one `effect_table` rule and report whether it produced at least
/// one effect whose `kind` matches. Any error (malformed rule, eval failure)
/// is treated as `No`, matching the non-striking/non-holding default so a bad
/// content edit degrades safely instead of crashing the tick.
fn rule_decides(
    rule: &Option<Value>,
    field: &str,
    content: &ContentPack,
    state: &WorldState,
    actor_id: &str,
) -> Decision {
    let Some(config) = rule.as_ref() else {
        return Decision::No;
    };
    let input = build_input(content, state, actor_id);
    let payload = match evaluate_symbolic_value(config, &input) {
        Ok(payload) => payload,
        Err(_) => return Decision::No,
    };
    let effects = payload
        .get("effects")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if effects
        .iter()
        .any(|effect| effect.get("kind").and_then(Value::as_str) == Some(field))
    {
        Decision::Yes
    } else {
        Decision::No
    }
}

pub(crate) fn skill_rule_emits(
    rule: &Value,
    effect_kind: &str,
    content: &ContentPack,
    state: &WorldState,
    actor_id: &str,
) -> bool {
    matches!(
        rule_decides(&Some(rule.clone()), effect_kind, content, state, actor_id),
        Decision::Yes
    )
}

/// Should this actor declare a strike this tick? Fully decided by the pack's
/// `strike` rule in `behavior.json` (stance, health, cooldown, room, etc.).
pub(crate) fn strike_event(
    content: &ContentPack,
    state: &WorldState,
    actor_id: &str,
) -> Option<WorldEvent> {
    if state.phase != crate::engine::state::GamePhase::Active {
        return None;
    }
    let behavior = resolved_behavior(content, actor_id);
    if !behavior.strike_skill_id.is_empty()
        && !state.actor_has_skill(actor_id, &behavior.strike_skill_id)
    {
        return None;
    }
    match rule_decides(&behavior.strike, "strike", content, state, actor_id) {
        Decision::Yes if select_hostile_target(content, state, actor_id).is_some() => {
            Some(WorldEvent::HostileStrike {
                actor_id: actor_id.to_string(),
            })
        }
        Decision::No => None,
        Decision::Yes => None,
    }
}

/// Must this actor hold still (blocked from moving) this tick? Decided by the
/// pack's `hold` rule in `behavior.json`.
pub(crate) fn should_hold(content: &ContentPack, state: &WorldState, actor_id: &str) -> bool {
    let behavior = resolved_behavior(content, actor_id);
    matches!(
        rule_decides(&behavior.hold, "hold", content, state, actor_id),
        Decision::Yes
    )
}
