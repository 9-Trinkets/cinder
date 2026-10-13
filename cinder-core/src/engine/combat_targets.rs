use crate::content::types::ContentPack;
use crate::engine::orders::is_hold;
use crate::engine::state::{ActorStance, WorldState};

pub(crate) fn select_hostile_target(
    content: &ContentPack,
    state: &WorldState,
    attacker_id: &str,
) -> Option<String> {
    let room_id = state.actor_current_room_id(content, attacker_id);
    let party_candidates = living_party_actors_in_room(content, state, &room_id);

    // 1. Any allied actor on "guard" order intercepts first.
    if let Some(guard) = party_candidates.iter().find(|actor_id| {
        state
            .party_order(content, actor_id)
            .is_some_and(|order| is_hold(&order))
    }) {
        return Some(guard.clone());
    }

    // 2. Defenseless civilians/offerings in the attacker's room.
    // Only hostile siege attackers target civilians or offerings.
    let is_siege_attacker = state
        .actor(content, attacker_id)
        .is_some_and(|a| a.tags.iter().any(|t| t == "siege"));
    if is_siege_attacker {
        let civilians = living_civilians_in_room(content, state, &room_id);
        if let Some(civilian) = civilians.first() {
            return Some(civilian.clone());
        }
    }

    // 3. Player or other living party members in the room.
    party_candidates
        .iter()
        .find(|actor_id| *actor_id == &content.settings.combat.player_actor_id)
        .or_else(|| party_candidates.first())
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
                .is_some_and(|order| is_hold(&order))
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
            if is_player && state.is_player_invisible() {
                return false;
            }
            (is_player || state.stance(&actor.id) == ActorStance::Allied)
                && party_actor_is_in_room(content, state, &actor.id, room_id)
                && !state.actor_is_defeated(&actor.id, &content.settings.combat.health_stat_id)
        })
        .map(|actor| actor.id.clone())
        .chain(
            state
                .actors
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

fn living_civilians_in_room(
    content: &ContentPack,
    state: &WorldState,
    room_id: &str,
) -> Vec<String> {
    content
        .actors
        .iter()
        .filter(|actor| {
            !content.is_player_actor(&actor.id)
                && state.stance(&actor.id) != ActorStance::Hostile
                && actor
                    .tags
                    .iter()
                    .any(|tag| tag == "civilian" || tag == "offering")
                && !state.actor_is_imprisoned(content, &actor.id)
                && state.actor_is_in_room(content, &actor.id, room_id)
                && !state.actor_is_defeated(&actor.id, &content.settings.combat.health_stat_id)
        })
        .map(|actor| actor.id.clone())
        .chain(
            state
                .actors
                .spawned_actors
                .values()
                .filter(|actor| {
                    state.stance(&actor.id) != ActorStance::Hostile
                        && actor
                            .tags
                            .iter()
                            .any(|tag| tag == "civilian" || tag == "offering")
                        && !state.actor_is_imprisoned(content, &actor.id)
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
            .actors
            .actor_room_overrides
            .insert(guard_id.clone(), room_id.clone());
        state.set_stance(&guard_id, ActorStance::Allied);
        state
            .relations
            .party_orders
            .insert(guard_id.clone(), "guard".to_string());

        assert_eq!(
            select_hostile_target(&content, &state, &hostile_id),
            Some(guard_id)
        );
        assert_ne!(player_id, hostile_id);
    }

    #[test]
    fn civilians_targeted_when_unguarded_and_ignored_when_moved() {
        let mut content = crate::engine::test_fixtures::minimal_test_pack();
        let player_id = content.settings.combat.player_actor_id.clone();
        let hostile_id = content.actors[0].id.clone();
        let room_id = content.actors[0].room_id.clone();
        let civilian_id = content.actors[1].id.clone();
        content.actors[0].tags.push("siege".to_string());
        content.actors[1].tags.push("civilian".to_string());
        let mut player_actor = content.actors[0].clone();
        player_actor.id = player_id.clone();
        player_actor.name = "Player".to_string();
        player_actor.room_id = room_id.clone();
        content.actors.push(player_actor);
        crate::engine::test_fixtures::rebuild_test_pack_indexes(&mut content);

        let mut state = WorldState::new(&content);
        state.current_room_id = room_id.clone();
        state
            .actors
            .actor_room_overrides
            .insert(civilian_id.clone(), room_id.clone());

        // 1. Without a guard, hostile attacks civilian instead of player
        assert_eq!(
            select_hostile_target(&content, &state, &hostile_id),
            Some(civilian_id.clone())
        );

        // 2. When civilian is moved to another room, hostile attacks player in room
        state
            .actors
            .actor_room_overrides
            .insert(civilian_id.clone(), "other_room".to_string());
        assert_eq!(
            select_hostile_target(&content, &state, &hostile_id),
            Some(player_id.clone())
        );

        // 3. When an ally guards in the room with civilian, guard is targeted over civilian
        state
            .actors
            .actor_room_overrides
            .insert(civilian_id.clone(), room_id.clone());
        let guard_id = "test-guard".to_string();
        let mut guard_actor = content.actors[0].clone();
        guard_actor.id = guard_id.clone();
        guard_actor.name = "Guard".to_string();
        guard_actor.room_id = room_id.clone();
        content.actors.push(guard_actor);
        crate::engine::test_fixtures::rebuild_test_pack_indexes(&mut content);

        state
            .actors
            .actor_room_overrides
            .insert(guard_id.clone(), room_id.clone());
        state.set_stance(&guard_id, ActorStance::Allied);
        state
            .relations
            .party_orders
            .insert(guard_id.clone(), "guard".to_string());
        assert_eq!(
            select_hostile_target(&content, &state, &hostile_id),
            Some(guard_id)
        );
    }

    #[test]
    fn non_siege_hostile_does_not_target_civilians() {
        let mut content = crate::engine::test_fixtures::minimal_test_pack();
        let player_id = content.settings.combat.player_actor_id.clone();
        let hostile_id = content.actors[0].id.clone();
        let room_id = content.actors[0].room_id.clone();
        let civilian_id = content.actors[1].id.clone();
        content.actors[1].tags.push("civilian".to_string());
        let mut player_actor = content.actors[0].clone();
        player_actor.id = player_id;
        player_actor.name = "Player".to_string();
        player_actor.room_id = "other_room".to_string();
        content.actors.push(player_actor);
        crate::engine::test_fixtures::rebuild_test_pack_indexes(&mut content);

        let mut state = WorldState::new(&content);
        state.current_room_id = "other_room".to_string();
        state
            .actors
            .actor_room_overrides
            .insert(civilian_id, room_id);

        // Hostile is in room with civilian, but lacks "siege" tag. Must not target civilian!
        assert_eq!(select_hostile_target(&content, &state, &hostile_id), None);
    }

    #[test]
    fn caged_offerings_not_targeted_before_cages_opened() {
        let mut content = crate::engine::test_fixtures::minimal_test_pack();
        let hostile_id = content.actors[0].id.clone();
        let room_id = content.actors[0].room_id.clone();
        let offering_id = content.actors[1].id.clone();
        content.actors[0].tags.push("siege".to_string());
        content.actors[1].tags.push("offering".to_string());
        content.actors[1].unlocked_by_story_var = Some("courtyard_cages_opened".to_string());
        content.actors[1].room_id = room_id.clone();
        crate::engine::test_fixtures::rebuild_test_pack_indexes(&mut content);

        let mut state = WorldState::new(&content);
        state.current_room_id = "other_room".to_string();
        state
            .actors
            .actor_room_overrides
            .insert(offering_id.clone(), room_id);

        // Cages not opened -> offering not targetable even by siege mob
        assert_eq!(select_hostile_target(&content, &state, &hostile_id), None);

        // Cages opened -> offering is targetable by siege mob
        let _ = state.story_vars.set("courtyard_cages_opened", "true");
        assert_eq!(
            select_hostile_target(&content, &state, &hostile_id),
            Some(offering_id)
        );
    }
}
