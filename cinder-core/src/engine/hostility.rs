//! Hostile-strike policy: decides *which* hostile actors declare strikes on a
//! background tick. The reducer resolves the mechanics of each declared
//! [`WorldEvent::HostileStrike`] generically; this module only selects.

use crate::content::types::ContentPack;
use crate::engine::events::WorldEvent;
use crate::engine::state::{ActorStance, GamePhase, WorldState};

/// Rules policy: hostile actors sharing the player's room whose attack
/// cooldown has elapsed declare one strike each.
pub(crate) fn plan_rules_hostility(content: &ContentPack, state: &WorldState) -> Vec<WorldEvent> {
    if state.phase != GamePhase::Active {
        return Vec::new();
    }
    let current_time_minutes = state.current_time_minutes;
    state
        .relationships
        .iter()
        .filter(|(_, relationship)| relationship.stance == ActorStance::Hostile)
        .filter(|(actor_id, _)| {
            state.actor_stat(actor_id, &content.settings.combat.health_stat_id) > 0
                && state
                    .next_hostile_strike_at
                    .get(*actor_id)
                    .is_none_or(|next_strike| current_time_minutes >= *next_strike)
                && {
                    let default_room_id = content
                        .actor(actor_id)
                        .map(|actor| actor.room_id.clone())
                        .unwrap_or_default();
                    state.actor_room_id(actor_id, &default_room_id) == state.current_room_id
                }
        })
        .map(|(actor_id, _)| WorldEvent::HostileStrike {
            actor_id: actor_id.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::loader::load_named_pack;
    use crate::engine::state::GamePhase;

    /// Fixture reusing the loaded pack's own actors: the first becomes the
    /// hostile creature sharing the player's room, the second waits elsewhere.
    /// Mutating definitions in place keeps the pack's id index consistent.
    fn hostile_fixture() -> (ContentPack, WorldState, String, String) {
        let mut content = load_named_pack("aera", Some("en")).expect("load aera");
        assert!(content.actors.len() >= 2, "fixture needs two actors");
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
