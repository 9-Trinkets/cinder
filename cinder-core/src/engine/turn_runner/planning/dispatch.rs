use super::content::plan_content_command;
use super::dialogue::plan_dialogue_command;
use super::observe::{plan_observe_room, plan_observe_target};
use super::targeted::plan_targeted_state_command;
use super::targetless::plan_targetless_command;
use super::super::types::PlannedTurn;
use super::PlanningContext;
use crate::content::types::{ActionDefinition, CommandEffect, CommandOutcomeMode, ContentPack};

fn plan_command_effects(
    content: &ContentPack,
    action: &ActionDefinition,
    input: Option<&str>,
    context: &PlanningContext<'_>,
    planned: &mut PlannedTurn,
) -> bool {
    if action.has_effect(CommandEffect::MoveActor) && !action.destination_room_id.is_empty() {
        // A fixed-destination move (e.g. descending a ladder) is targetless.
        plan_targetless_command(content, action, context, planned)
    } else if action.has_effect(CommandEffect::ObserveRoom) {
        let target = input.unwrap_or_default().trim();
        if target.is_empty() {
            plan_observe_room(context, planned)
        } else {
            plan_observe_target(content, target, context, planned)
        }
    } else if action.has_any_effect(&[
        CommandEffect::ObserveFeature,
        CommandEffect::ObserveActor,
        CommandEffect::MoveActor,
        CommandEffect::AttackTarget,
    ]) {
        plan_targeted_state_command(content, action, input, context, planned)
    } else if action.has_any_effect(&[
        CommandEffect::DropItem,
        CommandEffect::PickUpItem,
        CommandEffect::EquipItem,
        CommandEffect::UnequipItem,
        CommandEffect::UseItem,
    ]) {
        plan_targetless_command(content, action, context, planned)
    } else {
        panic!(
            "player command '{}' uses command effects without a supported planner effect",
            action.id
        )
    }
}

pub(crate) fn plan_authored_command(
    content: &ContentPack,
    command_id: &str,
    input: Option<&str>,
    context: PlanningContext<'_>,
    planned: &mut PlannedTurn,
) -> bool {
    let action = content
        .command(command_id)
        .unwrap_or_else(|| panic!("missing command definition '{command_id}'"));
    if action.outcome_mode == CommandOutcomeMode::Dialogue {
        plan_dialogue_command(content, action, input, &context, planned)
    } else if !action.effects.is_empty() {
        plan_command_effects(content, action, input, &context, planned)
    } else {
        plan_content_command(content, action, input, &context, planned)
    }
}
