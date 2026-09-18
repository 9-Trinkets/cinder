use crate::engine::turn_runner::types::PlannedTurn;
use crate::content::types::{ContentPack, PartyOrderKind};
use crate::engine::events::WorldEvent;
use crate::engine::state::{ActorStance, WorldState};

use super::items::matching_items;
use crate::content::types::ActorDefinition;

pub(super) fn find_party_member<'a>(
    content: &'a ContentPack,
    planner_state: &WorldState,
    current_room_id: &str,
    actor_reference: &str,
) -> Result<&'a ActorDefinition, &'static str> {
    let reference = actor_reference.trim();
    if reference.is_empty() {
        return Err("party.order_member_unavailable");
    }
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
        return Err("party.order_member_unavailable");
    };
    if matches.next().is_some() {
        return Err("party.order_member_ambiguous");
    }
    Ok(actor)
}

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
    let actor = match find_party_member(content, planner_state, current_room_id, actor_reference) {
        Ok(actor) => actor,
        Err(err_key) => {
            planned.events.push(WorldEvent::ActionRejected {
                message: content.render_message(err_key, &[]).unwrap_or_default(),
            });
            return false;
        }
    };
    planned.events.push(WorldEvent::PartyOrderAssigned {
        actor_id: actor.id.clone(),
        order,
    });
    false
}

pub(super) fn plan_give_to_party_member(
    content: &ContentPack,
    planner_state: &WorldState,
    current_room_id: &str,
    item_target: &str,
    actor_reference: &str,
    planned: &mut PlannedTurn,
) -> bool {
    let reference = actor_reference.trim();
    if reference.is_empty() {
        planned.events.push(WorldEvent::ActionRejected {
            message: content
                .render_message("party.give_member_unavailable", &[])
                .unwrap_or_else(|| "Who do you want to give that to?".to_string()),
        });
        return false;
    }
    let actor = match find_party_member(content, planner_state, current_room_id, reference) {
        Ok(actor) => actor,
        Err("party.order_member_ambiguous") => {
            planned.events.push(WorldEvent::ActionRejected {
                message: content
                    .render_message("party.order_member_ambiguous", &[])
                    .unwrap_or_default(),
            });
            return false;
        }
        Err(_) => {
            planned.events.push(WorldEvent::ActionRejected {
                message: content
                    .render_message("party.give_member_unavailable", &[])
                    .unwrap_or_default(),
            });
            return false;
        }
    };

    let item_target = item_target.trim();
    if item_target.is_empty() {
        planned.events.push(WorldEvent::ActionRejected {
            message: content
                .render_message("party.give_not_have", &[])
                .unwrap_or_default(),
        });
        return false;
    }

    let candidates = matching_items(content, item_target);
    let in_inventory = candidates
        .iter()
        .filter(|item| planner_state.player_inventory.contains_key(&item.id))
        .copied()
        .collect::<Vec<_>>();

    if in_inventory.is_empty() {
        let is_equipped = candidates.iter().any(|item| {
            planner_state
                .equipment
                .values()
                .any(|equipped_id| equipped_id == &item.id)
        });
        if is_equipped {
            let label = candidates
                .iter()
                .find(|item| {
                    planner_state
                        .equipment
                        .values()
                        .any(|equipped_id| equipped_id == &item.id)
                })
                .map(|item| item.label.as_str())
                .unwrap_or(item_target);
            planned.events.push(WorldEvent::ActionRejected {
                message: content
                    .render_message("party.give_equipped", &[("item", label)])
                    .unwrap_or_else(|| format!("Unequip the {label} before giving it.")),
            });
            return false;
        }
        planned.events.push(WorldEvent::ActionRejected {
            message: content
                .render_message("party.give_not_have", &[])
                .unwrap_or_default(),
        });
        return false;
    }

    let chosen = in_inventory
        .iter()
        .find(|item| {
            item.id.eq_ignore_ascii_case(item_target)
                || item.label.eq_ignore_ascii_case(item_target)
        })
        .copied()
        .unwrap_or(in_inventory[0]);

    if !chosen.is_takeable() {
        planned.events.push(WorldEvent::ActionRejected {
            message: content
                .render_message("party.give_denied", &[("item", &chosen.label)])
                .unwrap_or_default(),
        });
        return false;
    }

    planned.events.push(WorldEvent::PlayerGaveItemToPartyMember {
        actor_id: actor.id.clone(),
        item_id: chosen.id.clone(),
    });
    true
}

pub(super) fn plan_take_from_party_member(
    content: &ContentPack,
    planner_state: &WorldState,
    current_room_id: &str,
    item_target: &str,
    actor_reference: &str,
    planned: &mut PlannedTurn,
) -> bool {
    let reference = actor_reference.trim();
    if reference.is_empty() {
        planned.events.push(WorldEvent::ActionRejected {
            message: content
                .render_message("party.take_member_unavailable", &[])
                .unwrap_or_else(|| "Who do you want to take that from?".to_string()),
        });
        return false;
    }
    let actor = match find_party_member(content, planner_state, current_room_id, reference) {
        Ok(actor) => actor,
        Err("party.order_member_ambiguous") => {
            planned.events.push(WorldEvent::ActionRejected {
                message: content
                    .render_message("party.order_member_ambiguous", &[])
                    .unwrap_or_default(),
            });
            return false;
        }
        Err(_) => {
            planned.events.push(WorldEvent::ActionRejected {
                message: content
                    .render_message("party.take_member_unavailable", &[])
                    .unwrap_or_default(),
            });
            return false;
        }
    };

    let item_target = item_target.trim();
    if item_target.is_empty() {
        let actor_name = crate::engine::state::display_actor_name(planner_state, actor);
        planned.events.push(WorldEvent::ActionRejected {
            message: content
                .render_message("party.take_not_have", &[("actor", &actor_name)])
                .unwrap_or_default(),
        });
        return false;
    }

    let candidates = matching_items(content, item_target);
    let carrying = candidates
        .iter()
        .filter(|item| {
            planner_state.actor_has_item(&actor.id, &item.id)
                || planner_state.actor_item_is_equipped(content, &actor.id, item)
        })
        .copied()
        .collect::<Vec<_>>();

    if carrying.is_empty() {
        let actor_name = crate::engine::state::display_actor_name(planner_state, actor);
        planned.events.push(WorldEvent::ActionRejected {
            message: content
                .render_message("party.take_not_have", &[("actor", &actor_name)])
                .unwrap_or_default(),
        });
        return false;
    }

    let chosen = carrying
        .iter()
        .find(|item| {
            item.id.eq_ignore_ascii_case(item_target)
                || item.label.eq_ignore_ascii_case(item_target)
        })
        .copied()
        .unwrap_or(carrying[0]);

    planned.events.push(WorldEvent::PlayerTookItemFromPartyMember {
        actor_id: actor.id.clone(),
        item_id: chosen.id.clone(),
    });
    true
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