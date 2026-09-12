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