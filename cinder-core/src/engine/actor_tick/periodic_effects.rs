use super::{room_is_in_tick_scope, tick_scope_room_ids};
use crate::content::types::{ContentPack, ItemStorageTarget, PeriodicActorEffectTargets};
use crate::engine::events::WorldEvent;
use crate::engine::state::{ActorStance, GamePhase, WorldState};

pub(crate) fn plan_periodic_effect_events(
    content: &ContentPack,
    state: &WorldState,
) -> Vec<WorldEvent> {
    if state.phase != GamePhase::Active {
        return Vec::new();
    }
    let scope_room_ids = tick_scope_room_ids(content, state);
    let mut events = Vec::new();
    for definition in &content.settings.periodic_actor_effects {
        for actor in &content.actors {
            if !target_matches(content, state, &actor.id, definition.targets) {
                continue;
            }
            let room_id = state.actor_room_id(&actor.id, &actor.room_id);
            if !room_is_in_tick_scope(&scope_room_ids, room_id) {
                continue;
            }
            if !state.has_item_in_storage(
                &definition.trigger.room_item,
                ItemStorageTarget::CurrentRoom,
                room_id,
            ) {
                continue;
            }
            events.push(WorldEvent::PeriodicActorEffectApplied {
                actor_id: actor.id.clone(),
                effect_id: definition.id.clone(),
            });
        }
    }
    events
}

pub(crate) fn target_matches(
    content: &ContentPack,
    state: &WorldState,
    actor_id: &str,
    targets: PeriodicActorEffectTargets,
) -> bool {
    if state.actor_stat(actor_id, &content.settings.combat.health_stat_id) <= 0 {
        return false;
    }
    match targets {
        PeriodicActorEffectTargets::HostileLiving => state.stance(actor_id) == ActorStance::Hostile,
        PeriodicActorEffectTargets::AlliedLiving => state.stance(actor_id) == ActorStance::Allied,
        PeriodicActorEffectTargets::AnyLiving => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::types::{
        ActorTickScope, ItemDefinition, PeriodicActorEffect, PeriodicActorEffectDefinition,
        PeriodicActorEffectTrigger, RoomDefinition,
    };

    fn periodic_fixture(scope: ActorTickScope) -> (ContentPack, WorldState, String) {
        let mut content = crate::engine::test_fixtures::minimal_test_pack();
        let actor_id = content.actors[0].id.clone();
        content.settings.actor_tick_scope = scope;
        content.settings.combat.health_stat_id = "stamina".to_string();
        content.settings.periodic_actor_effects = vec![PeriodicActorEffectDefinition {
            id: "hazard".to_string(),
            trigger: PeriodicActorEffectTrigger {
                room_item: "hazard-token".to_string(),
            },
            targets: PeriodicActorEffectTargets::HostileLiving,
            effect: PeriodicActorEffect::Damage { amount: 2 },
            message: "combat.hazard".to_string(),
        }];
        content.items.push(ItemDefinition {
            id: "hazard-token".to_string(),
            ..ItemDefinition::default()
        });
        let mut state = WorldState::new(&content);
        state.set_stance(&actor_id, ActorStance::Hostile);
        (content, state, actor_id)
    }

    #[test]
    fn plans_configured_effect_for_matching_target_and_room_item() {
        let (content, mut state, actor_id) = periodic_fixture(ActorTickScope::CurrentBoard);
        let room_id = state
            .actor_room_id(&actor_id, &content.actors[0].room_id)
            .to_string();
        state.add_item_to_storage("hazard-token", ItemStorageTarget::CurrentRoom, &room_id);

        assert_eq!(
            plan_periodic_effect_events(&content, &state),
            vec![WorldEvent::PeriodicActorEffectApplied {
                actor_id,
                effect_id: "hazard".to_string(),
            }]
        );
    }

    #[test]
    fn current_board_scope_excludes_effects_on_disconnected_rooms() {
        let (mut content, mut state, actor_id) = periodic_fixture(ActorTickScope::CurrentBoard);
        content.rooms.push(RoomDefinition {
            id: "remote".to_string(),
            title: "Remote".to_string(),
            summary: String::new(),
            inspect_text: String::new(),
            allow_rest: false,
            features: Vec::new(),
            exits: Vec::new(),
            descriptions: Vec::new(),
        });
        state
            .actor_room_overrides
            .insert(actor_id.clone(), "remote".to_string());
        state.add_item_to_storage("hazard-token", ItemStorageTarget::CurrentRoom, "remote");

        assert!(plan_periodic_effect_events(&content, &state).is_empty());
        content.settings.actor_tick_scope = ActorTickScope::AllRooms;
        assert_eq!(plan_periodic_effect_events(&content, &state).len(), 1);
    }

    #[test]
    fn typed_targets_distinguish_hostile_allied_and_any_living() {
        let (content, mut state, actor_id) = periodic_fixture(ActorTickScope::AllRooms);

        assert!(target_matches(
            &content,
            &state,
            &actor_id,
            PeriodicActorEffectTargets::HostileLiving
        ));
        state.set_stance(&actor_id, ActorStance::Allied);
        assert!(target_matches(
            &content,
            &state,
            &actor_id,
            PeriodicActorEffectTargets::AlliedLiving
        ));
        state.set_stance(&actor_id, ActorStance::Neutral);
        assert!(target_matches(
            &content,
            &state,
            &actor_id,
            PeriodicActorEffectTargets::AnyLiving
        ));
    }
}
