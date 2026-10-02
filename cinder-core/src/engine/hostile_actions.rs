//! Hostile-action policy: decides *which* actors declare hostile strikes on a
//! background tick. The reducer resolves the mechanics of each declared
//! [`WorldEvent::HostileStrike`] generically; this module only selects.
//!
//! The selection policy itself is content-driven: each pack's `behavior.json`
//! `strike` rule decides per actor (see [`crate::engine::behavior`]).
//! Rust retains reducer eligibility backstops, cooldown scheduling, and damage
//! mechanics, but does not add another selection policy here.

use crate::content::types::{ContentPack, SkillKind};
use crate::engine::behavior;
use crate::engine::events::WorldEvent;
use crate::engine::state::{ActorStance, WorldState};

/// Rules mode: an actor declares a strike or heal when ready.
/// If an actor has healing capability and any wounded hostile ally (or self)
/// is present in the room, it prioritizes healing.
/// Otherwise, it declares a strike against the player.
pub(crate) fn plan_rules_hostile_actions(
    content: &ContentPack,
    state: &WorldState,
) -> Vec<WorldEvent> {
    let actor_ids = content
        .actors
        .iter()
        .map(|actor| actor.id.clone())
        .collect::<Vec<_>>();
    actor_ids
        .into_iter()
        .filter_map(|actor_id| {
            let strike = behavior::strike_event(content, state, &actor_id)?;
            if let Some((amount, message)) = hostile_healing_spec(content, state, &actor_id)
                && let Some(target_id) = select_hostile_heal_target(content, state, &actor_id)
            {
                return Some(WorldEvent::HostileHeal {
                    actor_id,
                    target_id,
                    amount,
                    message,
                });
            }
            Some(strike)
        })
        .collect()
}

/// Healing available to a hostile, preferring a declared `heal` skill over the
/// legacy `ActorDefinition.healing` fallback.
fn hostile_healing_spec(
    content: &ContentPack,
    state: &WorldState,
    actor_id: &str,
) -> Option<(i32, String)> {
    let legacy = state
        .actor(content, actor_id)
        .and_then(|actor| actor.healing.as_ref());
    if let Some(skill) = state.actor_skill_of_kind(content, actor_id, SkillKind::Heal)
        && let Some(heal) = &skill.heal
    {
        let message = skill
            .narration_key
            .clone()
            .or_else(|| legacy.map(|healing| healing.message.clone()))
            .unwrap_or_default();
        return Some((heal.amount, message));
    }
    legacy.map(|healing| (healing.amount, healing.message.clone()))
}

fn select_hostile_heal_target(
    content: &ContentPack,
    state: &WorldState,
    actor_id: &str,
) -> Option<String> {
    let health_stat = &content.settings.combat.health_stat_id;
    let default_room = content
        .actor(actor_id)
        .map(|a| a.room_id.as_str())
        .unwrap_or_default();
    let room_id = state.actor_room_id(actor_id, default_room);

    // Self-preservation: if self is wounded below 50% HP, prioritize self-healing.
    let self_hp = state.actor_stat(actor_id, health_stat);
    let self_max = state
        .actor_stat_maximum(content, actor_id, health_stat)
        .max(1);
    if self_hp * 2 < self_max {
        return Some(actor_id.to_string());
    }

    // Otherwise, collect all wounded living hostile allies in the same room (including self if below max).
    let mut wounded_allies = state
        .onstage_actors(content)
        .map(|actor| actor.id.clone())
        .filter(|other_id| {
            state.actor_is_in_room(content, other_id, &room_id)
                && state.stance(other_id) == ActorStance::Hostile
                && state.actor_stat(other_id, health_stat) > 0
                && state.actor_stat(other_id, health_stat)
                    < state.actor_stat_maximum(content, other_id, health_stat)
        })
        .collect::<Vec<_>>();

    if wounded_allies.is_empty() {
        return None;
    }

    // Select the wounded ally with the lowest health percentage.
    // If self is equally low, prioritize self.
    wounded_allies.sort_by(|left, right| {
        let (left_hp, left_max) = (
            state.actor_stat(left, health_stat),
            state.actor_stat_maximum(content, left, health_stat).max(1),
        );
        let (right_hp, right_max) = (
            state.actor_stat(right, health_stat),
            state.actor_stat_maximum(content, right, health_stat).max(1),
        );
        (i64::from(left_hp) * i64::from(right_max))
            .cmp(&(i64::from(right_hp) * i64::from(left_max)))
            .then_with(|| {
                if left == actor_id {
                    std::cmp::Ordering::Less
                } else if right == actor_id {
                    std::cmp::Ordering::Greater
                } else {
                    left.cmp(right)
                }
            })
    });

    wounded_allies.into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::state::{ActorStance, GamePhase};

    /// The strike decision re-declared as a neuron `effect_table` rule: strike
    /// when the actor is a living hostile sharing the player's room whose
    /// attack cooldown has elapsed. Mirrors the historical built-in policy.
    fn strike_default_rule() -> serde_json::Value {
        serde_json::json!({
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
                        { "path": "world.cooldown_elapsed", "operator": "equal", "value": true },
                        { "path": "world.in_player_room", "operator": "equal", "value": true }
                    ],
                    "payload_template": { "kind": "strike" }
                }]
            }
        })
    }

    /// Fixture reusing the synthetic pack's own actors: the first becomes the
    /// hostile creature sharing the player's room, the second waits elsewhere.
    /// Mutating definitions in place keeps the pack's id index consistent.
    fn hostile_fixture() -> (ContentPack, WorldState, String, String) {
        let mut content = crate::engine::test_fixtures::minimal_test_pack();
        assert!(content.actors.len() >= 2, "fixture needs two actors");
        // Re-declare the strike eligibility as content (matching the historical
        // hardcoded policy) so the fixture exercises the real content-driven path.
        content.behavior.defaults.strike = Some(strike_default_rule());
        let brute_id = content.actors[0].id.clone();
        let bystander_id = content.actors[1].id.clone();
        content.actors[0].room_id = "hall".to_string();
        content.actors[0].attack_interval_minutes = Some(3);
        content.actors[1].room_id = "annex".to_string();
        let mut state = WorldState::new(&content);
        state.current_room_id = "hall".to_string();
        state
            .actor_stats
            .entry(brute_id.clone())
            .or_default()
            .insert("hp".to_string(), 6);
        state
            .actor_stats
            .entry(brute_id.clone())
            .or_default()
            .insert("strength".to_string(), 3);
        (content, state, brute_id, bystander_id)
    }

    #[test]
    fn rules_policy_selects_due_hostile_in_player_room() {
        let (content, mut state, brute_id, _) = hostile_fixture();
        state.set_stance(&brute_id, ActorStance::Hostile);

        let events = plan_rules_hostile_actions(&content, &state);

        assert_eq!(
            events,
            vec![WorldEvent::HostileStrike { actor_id: brute_id }]
        );
    }

    #[test]
    fn rules_policy_skips_future_cooldown_dead_and_distant_actors() {
        let (content, mut state, brute_id, bystander_id) = hostile_fixture();
        state.set_stance(&brute_id, ActorStance::Hostile);
        state
            .next_hostile_strike_at
            .insert(brute_id.clone(), state.current_time_minutes + 5);
        // Dead hostile in the same room.
        state.set_stance(&bystander_id, ActorStance::Hostile);
        state
            .actor_stats
            .entry(bystander_id.clone())
            .or_default()
            .insert("hp".to_string(), 0);

        let events = plan_rules_hostile_actions(&content, &state);

        assert!(events.is_empty(), "no actor was eligible, got {events:?}");
    }

    #[test]
    fn rules_policy_skips_hostile_in_another_room() {
        let (content, mut state, brute_id, _) = hostile_fixture();
        state.current_room_id = "annex".to_string();
        state.set_stance(&brute_id, ActorStance::Hostile);

        assert!(plan_rules_hostile_actions(&content, &state).is_empty());
    }

    #[test]
    fn rules_policy_yields_nothing_when_not_active() {
        let (content, mut state, brute_id, _) = hostile_fixture();
        state.phase = GamePhase::GameEnded;
        state.set_stance(&brute_id, ActorStance::Hostile);

        assert!(plan_rules_hostile_actions(&content, &state).is_empty());
    }
}
