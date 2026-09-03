use crate::content::types::{
    ActionDefinition, ActionItemStorageTarget, CommandEffect, ContentPack, ItemStorageTarget,
    PanelDataSource, BeatObjectiveAffordanceTarget, BeatObjectiveCompletionTrigger,
    BeatObjectiveConditionalGuidanceDefinition, BeatObjectiveDefinition, BeatObjectiveProgressRef,
};
use crate::engine::dialogue::{ActorTurnActionRequest, ActorTurnCommandInvocation};
use crate::engine::state::{ActorStance, WorldState};

const OBJECTIVE_ACTOR_COMPLETE_STORY_VAR_PREFIX: &str = "beat_objective:actor_complete";
const OBJECTIVE_PROGRESS_STORY_VAR_PREFIX: &str = "beat_objective:progress";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CommandAvailabilityIssue {
    StageInactive,
    MissingObjectiveProgress(Vec<String>),
    BlockedByObjectiveProgress(Vec<String>),
    /// A room-item condition is unmet (`requires_room_item` / `requires_room_without_item`).
    RoomItemCondition(String),
}

pub(crate) fn apply_actor_turn_policies(
    content: &ContentPack,
    state: &WorldState,
    request: &mut ActorTurnActionRequest,
) {
    let actor_id = request.actor_id.clone();
    for objective in active_objectives(content, state)
        .filter(|objective| objective_applies_to_actor(content, state, objective, &actor_id))
    {
        let notes = objective_guidance_notes_for_actor(content, state, objective, &actor_id);
        request.current_beat_notes.extend(notes);
        apply_objective_affordance_priorities(objective, content, state, request);
    }
}

pub(crate) fn actor_objective_guidance_notes(
    content: &ContentPack,
    state: &WorldState,
    actor_id: &str,
) -> Vec<String> {
    active_objectives(content, state)
        .filter(|objective| objective_applies_to_actor(content, state, objective, actor_id))
        .flat_map(|objective| objective_guidance_notes_for_actor(content, state, objective, actor_id))
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ObjectiveSpeechEvent {
    ToActor,
    ToRoom,
}

pub(crate) fn mark_actor_objective_progress_for_speech_event(
    content: &ContentPack,
    state: &mut WorldState,
    actor_id: &str,
    event: ObjectiveSpeechEvent,
) {
    let keys = active_objectives(content, state)
        .filter(|objective| objective_applies_to_actor(content, state, objective, actor_id))
        .filter(|objective| {
            objective
                .completion
                .mark_actor_complete_on
                .iter()
                .any(|trigger| speech_trigger_matches(*trigger, event))
        })
        .map(|objective| objective_actor_complete_key(&objective.id, actor_id))
        .collect::<Vec<_>>();
    for key in keys {
        state.story_vars.set_unchecked(&key, "true");
    }
}

pub(crate) fn command_availability_issue(
    content: &ContentPack,
    state: &WorldState,
    action: &ActionDefinition,
) -> Option<CommandAvailabilityIssue> {
    let a = &action.available;
    if !a.available_during.is_empty()
        && !a
            .available_during
            .iter()
            .any(|stage_id| state.active_objective_stage_ids.contains(stage_id))
    {
        return Some(CommandAvailabilityIssue::StageInactive);
    }

    let missing = a
        .required_objective_progress
        .iter()
        .filter(|progress| !objective_progress_is_met(content, state, progress))
        .map(|progress| objective_progress_label(content, progress))
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Some(CommandAvailabilityIssue::MissingObjectiveProgress(missing));
    }

    let blocked = a
        .blocked_by_objective_progress
        .iter()
        .filter(|progress| objective_progress_is_met(content, state, progress))
        .map(|progress| objective_progress_label(content, progress))
        .collect::<Vec<_>>();
    if !blocked.is_empty() {
        return Some(CommandAvailabilityIssue::BlockedByObjectiveProgress(blocked));
    }

    if !a.requires_room_item.is_empty()
        && !state.has_item_in_storage(
            &a.requires_room_item,
            ItemStorageTarget::CurrentRoom,
            &state.current_room_id,
        )
    {
        return Some(CommandAvailabilityIssue::RoomItemCondition(
            "required".to_string(),
        ));
    }
    if !a.requires_room_without_item.is_empty()
        && state.has_item_in_storage(
            &a.requires_room_without_item,
            ItemStorageTarget::CurrentRoom,
            &state.current_room_id,
        )
    {
        return Some(CommandAvailabilityIssue::RoomItemCondition(
            "forbidden".to_string(),
        ));
    }
    if !a.requires_story_var.is_empty() && !story_var_is_truthy(state, &a.requires_story_var) {
        return Some(CommandAvailabilityIssue::StageInactive);
    }

    None
}

/// A story variable counts as set/truthy when it exists and isn't a falsy
/// string ("", "false", "0").
pub(crate) fn story_var_is_truthy(state: &WorldState, key: &str) -> bool {
    match state.story_vars.get(key) {
        Some(value) => !matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "" | "false" | "0"
        ),
        None => false,
    }
}

pub(crate) fn command_unavailable_message(
    content: &ContentPack,
    action: &ActionDefinition,
    issue: &CommandAvailabilityIssue,
) -> String {
    let verb = action.command.to_ascii_lowercase();
    match issue {
        CommandAvailabilityIssue::StageInactive => content
            .render_message("error.command_not_now", &[("verb", verb.as_str())])
            .unwrap_or_default(),
        CommandAvailabilityIssue::MissingObjectiveProgress(labels) => {
            let labels_text = labels.join(", ");
            content
                .render_message(
                    "error.command_needs_more",
                    &[("verb", verb.as_str()), ("labels", labels_text.as_str())],
                )
                .unwrap_or_default()
        }
        CommandAvailabilityIssue::BlockedByObjectiveProgress(labels) => {
            let labels_text = labels.join(", ");
            content
                .render_message(
                    "error.command_blocked_by_progress",
                    &[("verb", verb.as_str()), ("labels", labels_text.as_str())],
                )
                .unwrap_or_default()
        }
        CommandAvailabilityIssue::RoomItemCondition(_) => content
            .render_message("error.command_not_now", &[("verb", verb.as_str())])
            .unwrap_or_default(),
    }
}

pub(crate) fn apply_command_objective_progress_effects(
    state: &mut WorldState,
    action: &ActionDefinition,
) {
    for progress in &action.sets_objective_progress {
        state.story_vars.set_unchecked(
            &objective_progress_story_var_key(&progress.objective_id, &progress.key),
            "true",
        );
    }
    for progress in &action.clears_objective_progress {
        state
            .story_vars
            .values_mut()
            .remove(&objective_progress_story_var_key(
                &progress.objective_id,
                &progress.key,
            ));
    }
}

fn to_item_storage(storage: ActionItemStorageTarget) -> ItemStorageTarget {
    match storage {
        ActionItemStorageTarget::PlayerInventory => ItemStorageTarget::PlayerInventory,
        ActionItemStorageTarget::CurrentRoom => ItemStorageTarget::CurrentRoom,
    }
}

pub fn action_is_available(
    content: &ContentPack,
    state: &WorldState,
    action: &ActionDefinition,
    context_room_id: &str,
) -> bool {
    let a = &action.available;

    if !a.allowed_rooms.is_empty() && !a.allowed_rooms.contains(&context_room_id.to_string()) {
        return false;
    }

    if !a.requires_room_item.is_empty()
        && !state.has_item_in_storage(
            &a.requires_room_item,
            ItemStorageTarget::CurrentRoom,
            context_room_id,
        )
    {
        return false;
    }
    if !a.requires_room_without_item.is_empty()
        && state.has_item_in_storage(
            &a.requires_room_without_item,
            ItemStorageTarget::CurrentRoom,
            context_room_id,
        )
    {
        return false;
    }
    if !a.requires_story_var.is_empty() && !story_var_is_truthy(state, &a.requires_story_var) {
        return false;
    }

    if !a.available_during.is_empty()
        && !a
            .available_during
            .iter()
            .any(|stage_id| state.active_objective_stage_ids.contains(stage_id))
    {
        return false;
    }

    // Item-possession gating so the bar hides actions whose items you lack.
    if let Some(item_id) = &a.consumes_item
        && !state.has_item_in_storage(
            item_id,
            to_item_storage(a.consumes_item_storage.clone()),
            context_room_id,
        )
    {
        return false;
    }
    if !a.requires_any.is_empty() || !a.consumes_any.is_empty() {
        let all_required: Vec<_> = a.requires_any.iter().chain(a.consumes_any.iter()).collect();
        let has_any = all_required.iter().any(|id| {
            let storage = if a.consumes_any.iter().any(|c| c == *id) {
                to_item_storage(a.consumes_any_storage.clone())
            } else {
                to_item_storage(a.requires_any_storage.clone())
            };
            state.has_item_in_storage(id, storage, context_room_id)
        });
        if !has_any {
            return false;
        }
    }
    // Item-state gating derived from the effect, so content never has to restate
    // the obvious: equipping/using needs the item in the inventory, unequipping
    // needs it worn.
    if action.has_effect(CommandEffect::EquipItem) && !state.has_item(&action.item_id) {
        return false;
    }
    if action.has_effect(CommandEffect::UseItem) && !state.has_item(&action.item_id) {
        return false;
    }
    if action.has_effect(CommandEffect::UnequipItem) {
        let equipped = content
            .item(&action.item_id)
            .and_then(|item| state.equipped_item(&item.equip_slot));
        if equipped != Some(action.item_id.as_str()) {
            return false;
        }
    }

    for progress in &a.required_objective_progress {
        if !objective_progress_is_met(content, state, progress) {
            return false;
        }
    }

    for progress in &a.blocked_by_objective_progress {
        if objective_progress_is_met(content, state, progress) {
            return false;
        }
    }

    if !action_has_available_target(content, state, action, context_room_id) {
        return false;
    }

    true
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
            content.actors.iter().any(|actor| {
                if content.is_player_actor(&actor.id)
                    || state.actor_room_id(&actor.id, &actor.room_id) != room_id
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
        PanelDataSource::CraftableItems => action.item_creation.as_ref().is_some_and(|item_creation| {
            item_creation.craftable_items.iter().any(|item_id| {
                match item_creation.craftable_item_gates.get(item_id) {
                    None => true,
                    Some(gate) => gate.is_empty() || story_var_is_truthy(state, gate),
                }
            })
        }),
        PanelDataSource::Exits | PanelDataSource::Features => true,
        PanelDataSource::LooseRoomItems => !state.loose_room_items(room_id).is_empty(),
        PanelDataSource::InventoryItems => state
            .player_inventory
            .iter()
            .any(|(item_id, count)| {
                *count > 0 && !state.equipment.values().any(|equipped| equipped == item_id)
            }),
    }
}

fn objective_guidance_notes_for_actor(
    content: &ContentPack,
    state: &WorldState,
    objective: &BeatObjectiveDefinition,
    actor_id: &str,
) -> Vec<String> {
    let mut notes = Vec::new();
    let completed_actor_count = content
        .actors
        .iter()
        .map(|actor| actor.id.as_str())
        .filter(|actor_id| actor_is_complete(state, objective, actor_id))
        .count();
    if completed_actor_count < content.actors.len() {
        let actor_is_complete = actor_is_complete(state, objective, actor_id);
        if actor_is_complete {
            if !objective
                .guidance
                .prompt_note_if_others_incomplete
                .trim()
                .is_empty()
            {
                notes.push(objective.guidance.prompt_note_if_others_incomplete.clone());
            }
        } else if !objective
            .guidance
            .prompt_note_if_actor_incomplete
            .trim()
            .is_empty()
        {
            notes.push(objective.guidance.prompt_note_if_actor_incomplete.clone());
        }
    }
    for conditional in matching_conditional_guidance(content, state, objective) {
        if !conditional.prompt_note.trim().is_empty() {
            notes.push(conditional.prompt_note.clone());
        }
    }
    notes
}

fn apply_objective_affordance_priorities(
    objective: &BeatObjectiveDefinition,
    content: &ContentPack,
    state: &WorldState,
    request: &mut ActorTurnActionRequest,
) {
    let priorities = objective
        .guidance
        .conditional
        .iter()
        .filter(|conditional| conditional_guidance_matches(content, state, conditional))
        .flat_map(|conditional| conditional.prioritize.iter())
        .chain(objective.guidance.prioritize.iter())
        .collect::<Vec<_>>();
    if priorities.is_empty() {
        return;
    }
    request.affordances.sort_by_key(|affordance| {
        for (index, priority) in priorities.iter().enumerate() {
            let ActorTurnCommandInvocation::Command {
                command_id,
                target_actor_id,
                ..
            } = &affordance.invocation;
            let target_matches = match priority.target {
                BeatObjectiveAffordanceTarget::Any => true,
                BeatObjectiveAffordanceTarget::Actor => target_actor_id.is_some(),
                BeatObjectiveAffordanceTarget::Room => target_actor_id.is_none(),
            };
            if *command_id == priority.command_id && target_matches {
                return index;
            }
        }
        usize::MAX
    });
}

fn active_objectives<'a>(
    content: &'a ContentPack,
    state: &WorldState,
) -> impl Iterator<Item = &'a BeatObjectiveDefinition> {
    content.beat_objectives.objectives.iter().filter(|objective| {
        objective_stage_ids(objective).into_iter().any(|stage_id| {
            state
                .active_objective_stage_ids
                .iter()
                .any(|active| active == stage_id)
        })
    })
}

fn objective_applies_to_actor(
    content: &ContentPack,
    state: &WorldState,
    objective: &BeatObjectiveDefinition,
    actor_id: &str,
) -> bool {
    let active_stages = objective_stage_ids(objective)
        .into_iter()
        .filter(|stage_id| {
            state
                .active_objective_stage_ids
                .iter()
                .any(|active| active == *stage_id)
        })
        .filter_map(|stage_id| {
            content
                .beats
                .stages
                .iter()
                .find(|stage| stage.id == *stage_id)
        })
        .collect::<Vec<_>>();
    if active_stages.is_empty() {
        return true;
    }
    if active_stages
        .iter()
        .all(|stage| stage.target_actor_story_var.is_empty())
    {
        return true;
    }
    active_stages.iter().any(|stage| {
        !stage.target_actor_story_var.is_empty()
            && state
                .story_vars
                .get(&stage.target_actor_story_var)
                .map(|ids| ids.split(',').any(|id| id.trim() == actor_id))
                .unwrap_or(false)
    })
}

pub(crate) fn clear_inactive_objective_state(content: &ContentPack, state: &mut WorldState) {
    for objective in content.beat_objectives.objectives.iter().filter(|objective| {
        !objective_stage_ids(objective).into_iter().any(|stage_id| {
            state
                .active_objective_stage_ids
                .iter()
                .any(|active| active == stage_id)
        })
    }) {
        for actor in &content.actors {
            state
                .story_vars
                .values_mut()
                .remove(&objective_actor_complete_key(&objective.id, &actor.id));
        }
        for progress in &objective.progress.keys {
            state
                .story_vars
                .values_mut()
                .remove(&objective_progress_story_var_key(&objective.id, &progress.key));
        }
    }
}

fn objective_stage_ids(objective: &BeatObjectiveDefinition) -> Vec<&str> {
    objective.stage_ids.iter().map(String::as_str).collect()
}

fn actor_is_complete(state: &WorldState, objective: &BeatObjectiveDefinition, actor_id: &str) -> bool {
    state
        .story_vars
        .get(&objective_actor_complete_key(&objective.id, actor_id))
        .is_some_and(|value| value == "true")
}

fn objective_actor_complete_key(objective_id: &str, actor_id: &str) -> String {
    format!("{OBJECTIVE_ACTOR_COMPLETE_STORY_VAR_PREFIX}:{objective_id}:{actor_id}")
}

fn objective_progress_story_var_key(objective_id: &str, key: &str) -> String {
    format!("{OBJECTIVE_PROGRESS_STORY_VAR_PREFIX}:{objective_id}:{key}")
}

fn objective_progress_is_met(
    content: &ContentPack,
    state: &WorldState,
    progress: &BeatObjectiveProgressRef,
) -> bool {
    content
        .beat_objectives
        .objectives
        .iter()
        .any(|objective| objective.id == progress.objective_id)
        && state
            .story_vars
            .get(&objective_progress_story_var_key(
                &progress.objective_id,
                &progress.key,
            ))
            .is_some_and(|value| value == "true")
}

fn objective_progress_label(content: &ContentPack, progress: &BeatObjectiveProgressRef) -> String {
    content
        .beat_objectives
        .objectives
        .iter()
        .find(|objective| objective.id == progress.objective_id)
        .and_then(|objective| {
            objective
                .progress
                .keys
                .iter()
                .find(|entry| entry.key == progress.key)
        })
        .map(|entry| {
            if entry.label.trim().is_empty() {
                entry.key.clone()
            } else {
                entry.label.clone()
            }
        })
        .unwrap_or_else(|| progress.key.clone())
}

fn matching_conditional_guidance<'a>(
    content: &'a ContentPack,
    state: &'a WorldState,
    objective: &'a BeatObjectiveDefinition,
) -> impl Iterator<Item = &'a BeatObjectiveConditionalGuidanceDefinition> {
    objective
        .guidance
        .conditional
        .iter()
        .filter(|conditional| conditional_guidance_matches(content, state, conditional))
}

fn conditional_guidance_matches(
    content: &ContentPack,
    state: &WorldState,
    conditional: &BeatObjectiveConditionalGuidanceDefinition,
) -> bool {
    conditional
        .required_objective_progress
        .iter()
        .all(|progress| objective_progress_is_met(content, state, progress))
        && conditional
            .blocked_by_objective_progress
            .iter()
            .all(|progress| !objective_progress_is_met(content, state, progress))
}

fn speech_trigger_matches(trigger: BeatObjectiveCompletionTrigger, event: ObjectiveSpeechEvent) -> bool {
    matches!(
        (trigger, event),
        (
            BeatObjectiveCompletionTrigger::SpeechToActor,
            ObjectiveSpeechEvent::ToActor
        ) | (
            BeatObjectiveCompletionTrigger::SpeechToRoom,
            ObjectiveSpeechEvent::ToRoom
        )
    )
}
