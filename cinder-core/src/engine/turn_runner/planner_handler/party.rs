use crate::engine::turn_runner::types::PlannedTurn;
use crate::content::types::{ContentPack, PartyOrderKind};
use crate::engine::events::WorldEvent;
use crate::engine::state::{ActorStance, WorldState};

/// Plans an `order <actor> <order>` command against an allied onstage member in
/// the current room. Rejects when the member is unavailable or the reference is
/// ambiguous.
pub(super) fn plan_party_order(
    content: &ContentPack,
    planner_state: &WorldState,
    current_room_id: &str,
    actor_reference: &str,
    order: PartyOrderKind,
    planned: &mut PlannedTurn,
) -> bool {
    let reference = actor_reference.trim();
    let mut matches = content
        .onstage_actors()
        .filter(|actor| {
            planner_state.stance(&actor.id) == ActorStance::Allied
                && planner_state.actor_is_in_room(content, &actor.id, current_room_id)
                && !planner_state
                    .actor_is_defeated(&actor.id, &content.settings.combat.health_stat_id)
        })
        .filter(|actor| {
            actor.id.eq_ignore_ascii_case(reference)
                || actor.name.eq_ignore_ascii_case(reference)
                || actor
                    .aliases
                    .iter()
                    .any(|alias| alias.eq_ignore_ascii_case(reference))
        });
    let Some(actor) = matches.next() else {
        planned.events.push(WorldEvent::ActionRejected {
            message: content
                .render_message("party.order_member_unavailable", &[])
                .unwrap_or_default(),
        });
        return false;
    };
    if matches.next().is_some() {
        planned.events.push(WorldEvent::ActionRejected {
            message: content
                .render_message("party.order_member_ambiguous", &[])
                .unwrap_or_default(),
        });
        return false;
    }
    planned.events.push(WorldEvent::PartyOrderAssigned {
        actor_id: actor.id.clone(),
        order,
    });
    false
}

/// Plans a `follow <actor>` or `unfollow` command. When target is empty/None or
/// "none"/"nobody", clears the followed actor. Otherwise resolves target
/// against non-player actors. If the followed actor is in another room, moves
/// the player to that room.
pub(super) fn plan_follow_command(
    content: &ContentPack,
    planner_state: &WorldState,
    current_room_id: &str,
    target: Option<&str>,
    planned: &mut PlannedTurn,
) -> bool {
    let Some(target) = target.map(str::trim).filter(|s| !s.is_empty()) else {
        planned.events.push(WorldEvent::PlayerFollowedActor { actor_id: None });
        return false;
    };
    if target.eq_ignore_ascii_case("none") || target.eq_ignore_ascii_case("nobody") {
        planned.events.push(WorldEvent::PlayerFollowedActor { actor_id: None });
        return false;
    }
    let matched_actor = content
        .actors
        .iter()
        .filter(|actor| !content.is_player_actor(&actor.id))
        .find(|actor| {
            actor.id.eq_ignore_ascii_case(target)
                || actor.name.eq_ignore_ascii_case(target)
                || actor
                    .aliases
                    .iter()
                    .any(|alias| alias.eq_ignore_ascii_case(target))
        });
    let Some(actor) = matched_actor else {
        planned.events.push(WorldEvent::ActionRejected {
            message: format!("You don't see '{target}' anywhere to follow."),
        });
        return false;
    };
    planned.events.push(WorldEvent::PlayerFollowedActor {
        actor_id: Some(actor.id.clone()),
    });
    let actor_room = planner_state.actor_room_id(&actor.id, &actor.room_id);
    if actor_room != current_room_id {
        planned.events.push(WorldEvent::PlayerMoved {
            from_room_id: current_room_id.to_string(),
            to_room_id: actor_room.to_string(),
        });
        planned.events.push(WorldEvent::CurrentRoomObserved {
            room_id: actor_room.to_string(),
            mode: crate::engine::events::ObservationMode::Summary,
        });
    }
    false
}