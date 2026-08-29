//! Hostile-strike policy: decides *which* hostile actors declare strikes on a
//! background tick. The reducer resolves the mechanics of each declared
//! [`WorldEvent::HostileStrike`] generically; this module only selects.
//!
//! The selection policy itself is content-driven: each pack's `behavior.json`
//! `strike` rule decides per actor (see [`crate::engine::behavior`]).
//! No strike eligibility is hardcoded in Rust.

use crate::content::types::ContentPack;
use crate::engine::behavior;
use crate::engine::events::WorldEvent;
use crate::engine::state::WorldState;

/// Rules policy: a hostile actor declares a strike when its pack's `behavior`
/// `strike` rule fires.
pub(crate) fn plan_rules_hostility(content: &ContentPack, state: &WorldState) -> Vec<WorldEvent> {
    let actor_ids = content
        .actors
        .iter()
        .map(|actor| actor.id.clone())
        .collect::<Vec<_>>();
    actor_ids
        .into_iter()
        .filter_map(|actor_id| behavior::strike_event(content, state, &actor_id))
        .collect()
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

        let events = plan_rules_hostility(&content, &state);

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

        let events = plan_rules_hostility(&content, &state);

        assert!(events.is_empty(), "no actor was eligible, got {events:?}");
    }

    #[test]
    fn rules_policy_skips_hostile_in_another_room() {
        let (content, mut state, brute_id, _) = hostile_fixture();
        state.current_room_id = "annex".to_string();
        state.set_stance(&brute_id, ActorStance::Hostile);

        assert!(plan_rules_hostility(&content, &state).is_empty());
    }

    #[test]
    fn rules_policy_yields_nothing_when_not_active() {
        let (content, mut state, brute_id, _) = hostile_fixture();
        state.phase = GamePhase::GameEnded;
        state.set_stance(&brute_id, ActorStance::Hostile);

        assert!(plan_rules_hostility(&content, &state).is_empty());
    }
}
