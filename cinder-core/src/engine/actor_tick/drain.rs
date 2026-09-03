use super::{room_is_in_tick_scope, tick_scope_room_ids};
use crate::content::types::{ContentPack, ItemStorageTarget};
use crate::engine::events::WorldEvent;
use crate::engine::state::{ActorStance, GamePhase, WorldState};

pub(crate) fn plan_drain_events(content: &ContentPack, state: &WorldState) -> Vec<WorldEvent> {
    let combat = &content.settings.combat;
    let Some(drain_item_id) = combat.drain_item_id.as_deref() else {
        return Vec::new();
    };
    if combat.drain_damage_per_tick <= 0 || state.phase != GamePhase::Active {
        return Vec::new();
    }
    let scope_room_ids = tick_scope_room_ids(content, state);
    let mut events = Vec::new();
    for actor in &content.actors {
        if content.is_player_actor(&actor.id) {
            continue;
        }
        if state.stance(&actor.id) != ActorStance::Hostile {
            continue;
        }
        if state.actor_stat(&actor.id, &combat.health_stat_id) <= 0 {
            continue;
        }
        let room_id = state.actor_room_id(&actor.id, &actor.room_id);
        if !room_is_in_tick_scope(&scope_room_ids, room_id) {
            continue;
        }
        if !state.has_item_in_storage(drain_item_id, ItemStorageTarget::CurrentRoom, room_id) {
            continue;
        }
        events.push(WorldEvent::ActorDrained {
            actor_id: actor.id.clone(),
        });
    }
    events
}
