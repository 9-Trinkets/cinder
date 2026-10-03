use crate::content::types::ContentPack;
use crate::engine::state::{ActorStance, WorldState};

pub(crate) fn select_hostile_target(
    content: &ContentPack,
    state: &WorldState,
    attacker_id: &str,
) -> Option<String> {
    let room_id = state.actor_current_room_id(content, attacker_id);
    let candidates = living_party_actors_in_room(content, state, &room_id);

    candidates
        .iter()
        .find(|actor_id| {
            state
                .party_order(content, actor_id)
                .is_some_and(|order| order.eq_ignore_ascii_case("guard"))
        })
        .or_else(|| {
            candidates
                .iter()
                .find(|actor_id| *actor_id == &content.settings.combat.player_actor_id)
        })
        .or_else(|| candidates.first())
        .cloned()
}

pub(crate) fn room_has_living_guard(
    content: &ContentPack,
    state: &WorldState,
    room_id: &str,
) -> bool {
    living_party_actors_in_room(content, state, room_id)
        .into_iter()
        .any(|actor_id| {
            state
                .party_order(content, &actor_id)
                .is_some_and(|order| order.eq_ignore_ascii_case("guard"))
        })
}

pub(crate) fn party_actor_is_in_room(
    content: &ContentPack,
    state: &WorldState,
    actor_id: &str,
    room_id: &str,
) -> bool {
    if content.is_player_actor(actor_id) {
        state.current_room_id == room_id
    } else {
        state.actor_is_in_room(content, actor_id, room_id)
    }
}

fn living_party_actors_in_room(
    content: &ContentPack,
    state: &WorldState,
    room_id: &str,
) -> Vec<String> {
    content
        .actors
        .iter()
        .filter(|actor| {
            let is_player = content.is_player_actor(&actor.id);
            (is_player || state.stance(&actor.id) == ActorStance::Allied)
                && party_actor_is_in_room(content, state, &actor.id, room_id)
                && !state.actor_is_defeated(&actor.id, &content.settings.combat.health_stat_id)
        })
        .map(|actor| actor.id.clone())
        .chain(
            state
                .spawned_actors
                .values()
                .filter(|actor| {
                    state.stance(&actor.id) == ActorStance::Allied
                        && state.actor_is_in_room(content, &actor.id, room_id)
                        && !state
                            .actor_is_defeated(&actor.id, &content.settings.combat.health_stat_id)
                })
                .map(|actor| actor.id.clone()),
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_is_preferred_over_player_and_other_allies() {
        let content = crate::engine::test_fixtures::minimal_test_pack();
        let player_id = content.settings.combat.player_actor_id.clone();
        let guard_id = content.actors[1].id.clone();
        let hostile_id = content.actors[0].id.clone();
        let room_id = content.actors[0].room_id.clone();
        let mut state = WorldState::new(&content);
        state.current_room_id = room_id.clone();
        state
            .actor_room_overrides
            .insert(guard_id.clone(), room_id.clone());
        state.set_stance(&guard_id, ActorStance::Allied);
        state
            .party_orders
            .insert(guard_id.clone(), "guard".to_string());

        assert_eq!(
            select_hostile_target(&content, &state, &hostile_id),
            Some(guard_id)
        );
        assert_ne!(player_id, hostile_id);
    }
}
