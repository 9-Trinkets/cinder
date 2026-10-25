use crate::content::types::{ActionDefinition, CommandEffect, ContentPack, PanelDataSource};
use crate::engine::state::{ActorStance, WorldState};

mod rules;

/// Why a specific item-related effect cannot run right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CommandItemState {
    /// The item the effect needs is not in the relevant storage.
    NoneHeld(String),
    /// `EquipItem` for an item that is already worn.
    AlreadyEquipped(String),
    /// `UnequipItem` for an item that is not currently worn.
    NotEquipped(String),
}

/// The single reason an action is unavailable, produced by the first failing
/// [`AvailabilityRule`]. Ordered from most fundamental to most situational so
/// the message points at the most useful blocker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CommandAvailabilityIssue {
    MissingSkill,
    StageInactive,
    MissingObjectiveProgress(Vec<String>),
    BlockedByObjectiveProgress(Vec<String>),
    /// One of the action's `when` conditions does not hold.
    ConditionNotMet,
    /// The current room is not in `allowed_rooms`.
    RoomRestricted,
    /// `requires_item` / `consumes_item` / `requires_any` names an item we lack.
    MissingItem(String),
    /// `requires_any` / `consumes_any` had no satisfiable candidate.
    NothingToConsume,
    /// `item_consumer: first_actor_in_room` but no living actor is present.
    NoActorToServe,
    /// A command effect needs a specific item state that does not hold.
    ItemState(CommandItemState),
}

/// Everything an [`AvailabilityRule`] needs to decide whether an action may be
/// attempted. `room_id` is the room the check is evaluated in (the current room
/// for the player, the actor's own room for NPCs), which may differ from
/// `state.current_room_id`.
pub(crate) struct AvailabilityContext<'a> {
    pub(crate) content: &'a ContentPack,
    pub(crate) state: &'a WorldState,
    pub(crate) action: &'a ActionDefinition,
    pub(crate) actor_id: &'a str,
    pub(crate) room_id: &'a str,
}

/// One composable rule deciding whether an action is available. The first rule
/// that returns `Err` is the reported [`CommandAvailabilityIssue`]; when every
/// rule returns `Ok`, the action is available.
pub(crate) trait AvailabilityRule {
    fn check(&self, ctx: &AvailabilityContext<'_>) -> Result<(), CommandAvailabilityIssue>;
}

/// Runs the shared rule chain, returning the first unmet requirement (if any).
pub(crate) fn evaluate_availability(
    ctx: &AvailabilityContext<'_>,
) -> Result<(), CommandAvailabilityIssue> {
    for rule in rules::availability_rules() {
        rule.check(ctx)?;
    }
    Ok(())
}

pub(crate) fn command_availability_issue(
    content: &ContentPack,
    state: &WorldState,
    action: &ActionDefinition,
) -> Option<CommandAvailabilityIssue> {
    command_availability_issue_for_actor(
        content,
        state,
        action,
        &content.settings.combat.player_actor_id,
    )
}

pub(crate) fn command_availability_issue_for_actor(
    content: &ContentPack,
    state: &WorldState,
    action: &ActionDefinition,
    actor_id: &str,
) -> Option<CommandAvailabilityIssue> {
    command_availability_issue_for_actor_in_room(
        content,
        state,
        action,
        actor_id,
        &state.current_room_id,
    )
}

pub(crate) fn command_availability_issue_for_actor_in_room(
    content: &ContentPack,
    state: &WorldState,
    action: &ActionDefinition,
    actor_id: &str,
    room_id: &str,
) -> Option<CommandAvailabilityIssue> {
    evaluate_availability(&AvailabilityContext {
        content,
        state,
        action,
        actor_id,
        room_id,
    })
    .err()
}

pub(crate) use crate::engine::conditions::story_var_is_truthy;

pub(crate) fn command_unavailable_message(
    content: &ContentPack,
    action: &ActionDefinition,
    issue: &CommandAvailabilityIssue,
) -> String {
    let verb = action.command.to_ascii_lowercase();
    match issue {
        CommandAvailabilityIssue::StageInactive
        | CommandAvailabilityIssue::MissingSkill
        | CommandAvailabilityIssue::ConditionNotMet => content
            .render_message("error.command_not_now", &[("verb", verb.as_str())])
            .unwrap_or_default(),
        CommandAvailabilityIssue::MissingObjectiveProgress(labels) => {
            let labels_text = content.join_list(labels);
            content
                .render_message(
                    "error.command_needs_more",
                    &[("verb", verb.as_str()), ("labels", labels_text.as_str())],
                )
                .unwrap_or_default()
        }
        CommandAvailabilityIssue::BlockedByObjectiveProgress(labels) => {
            let labels_text = content.join_list(labels);
            content
                .render_message(
                    "error.command_blocked_by_progress",
                    &[("verb", verb.as_str()), ("labels", labels_text.as_str())],
                )
                .unwrap_or_default()
        }
        CommandAvailabilityIssue::RoomRestricted => {
            let needed = action
                .available
                .allowed_rooms
                .first()
                .and_then(|room_id| content.room(room_id))
                .map(|room| room.title.as_str())
                .unwrap_or("another room");
            let command_lower = action.command.to_lowercase();
            content
                .render_message(
                    "error.cannot_command_here",
                    &[("command", command_lower.as_str()), ("room", needed)],
                )
                .unwrap_or_default()
        }
        CommandAvailabilityIssue::MissingItem(item_id) => {
            let label = content.item_label(item_id);
            content
                .render_message("error.missing_item", &[("label", label)])
                .unwrap_or_default()
        }
        CommandAvailabilityIssue::NothingToConsume => content
            .render_message("error.nothing_to_consume", &[])
            .unwrap_or_default(),
        CommandAvailabilityIssue::NoActorToServe => content
            .render_message("error.no_actor_to_serve", &[])
            .unwrap_or_default(),
        CommandAvailabilityIssue::ItemState(state) => {
            let item_label = |item_id: &str| {
                content
                    .item(item_id)
                    .map(|item| item.label.clone())
                    .unwrap_or_default()
            };
            match state {
                CommandItemState::NoneHeld(item_id) => content
                    .render_message(&format!("item.{item_id}.none_held"), &[])
                    .or_else(|| content.render_message("item.none_held", &[]))
                    .unwrap_or_default(),
                CommandItemState::AlreadyEquipped(item_id) => {
                    let label = item_label(item_id);
                    content
                        .render_message("equipment.already_equipped", &[("item", label.as_str())])
                        .unwrap_or_default()
                }
                CommandItemState::NotEquipped(item_id) => {
                    let label = item_label(item_id);
                    content
                        .render_message("equipment.not_equipped", &[("item", label.as_str())])
                        .unwrap_or_default()
                }
            }
        }
    }
}

pub fn action_is_available(
    content: &ContentPack,
    state: &WorldState,
    action: &ActionDefinition,
    context_room_id: &str,
) -> bool {
    action_is_available_for_actor(
        content,
        state,
        action,
        context_room_id,
        &content.settings.combat.player_actor_id,
    )
}

pub(crate) fn action_is_available_for_actor(
    content: &ContentPack,
    state: &WorldState,
    action: &ActionDefinition,
    context_room_id: &str,
    actor_id: &str,
) -> bool {
    evaluate_availability(&AvailabilityContext {
        content,
        state,
        action,
        actor_id,
        room_id: context_room_id,
    })
    .is_ok()
        && action_has_available_target(content, state, action, context_room_id)
}

/// Whether a target-selection action currently has at least one eligible
/// target. This is the generic "hide when there are no targets" rule: an
/// action whose panel selects a target (actors in the room, or craftable
/// items) is hidden while its target list is empty. Informational panels
/// (`features`/look, `exits`/move) are never hidden here.
fn action_has_available_target(
    content: &ContentPack,
    state: &WorldState,
    action: &ActionDefinition,
    room_id: &str,
) -> bool {
    let Some(panel_config) = &action.ui.panel_config else {
        return true;
    };
    match panel_config.data_source {
        PanelDataSource::ActorsInRoom => {
            let is_attack = action.has_effect(CommandEffect::AttackTarget);
            state.actors(content).any(|actor| {
                if content.is_player_actor(&actor.id)
                    || !state.actor_is_in_room(content, &actor.id, room_id)
                    || state.actor_is_defeated(&actor.id, &content.settings.combat.health_stat_id)
                {
                    return false;
                }
                if is_attack {
                    let relationship = state.relationship(&actor.id);
                    // Attack never targets party members (allies or followers).
                    relationship.stance != ActorStance::Allied && !relationship.follows_player
                } else {
                    true
                }
            })
        }
        PanelDataSource::CraftableItems => {
            action.item_creation.as_ref().is_some_and(|item_creation| {
                item_creation.craftable_items.iter().any(|item_id| {
                    let unlocked = match item_creation.craftable_item_gates.get(item_id) {
                        None => true,
                        Some(gate) => gate.is_empty() || story_var_is_truthy(state, gate),
                    };
                    let already_traced = content.item(item_id).is_some_and(|item| item.trace_mark)
                        && state.has_item_in_storage(
                            item_id,
                            crate::content::types::ItemStorageTarget::CurrentRoom,
                            room_id,
                        );
                    unlocked && !already_traced
                })
            })
        }
        PanelDataSource::Exits | PanelDataSource::Features | PanelDataSource::FollowActors => true,
        PanelDataSource::ActorTopics => {
            !crate::engine::topics::room_topics(content, state).is_empty()
        }
        PanelDataSource::TeleportAnchors => content.has_any_teleport_destination(state),
        PanelDataSource::LooseRoomItems => {
            state
                .loose_room_items(room_id)
                .iter()
                .any(|(item_id, _)| content.item(item_id).is_none_or(|item| item.is_takeable()))
                || !state.takeable_feature_items(content, room_id).is_empty()
        }
        PanelDataSource::InventoryItems => state
            .inventory
            .player_inventory
            .iter()
            .any(|(_, count)| *count > 0),
    }
}
