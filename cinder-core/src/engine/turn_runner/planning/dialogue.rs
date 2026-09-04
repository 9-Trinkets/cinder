use super::super::types::{PendingDialogue, PlannedTurn};
use super::PlanningContext;
use crate::content::types::{ActionDefinition, ContentPack, PlayerCommandTargetMode};
use crate::engine::commands::{resolve_actor_reference_input, unknown_target_token};
use crate::engine::dialogue_grounding::viewer_participant_id;
use crate::engine::events::WorldEvent;

fn pending_dialogue_for(
    content: &ContentPack,
    context: &PlanningContext<'_>,
    actor_id: String,
    other_person_message: Option<String>,
) -> PendingDialogue {
    PendingDialogue {
        actor_id,
        current_room_id: context.current_room_id.to_string(),
        raw_input: context.raw_input.to_string(),
        other_person_id: viewer_participant_id(content),
        other_person_name: content.opening.title.clone(),
        other_person_message,
        turn_number: context.turn_number,
    }
}

pub(super) fn plan_dialogue_command(
    content: &ContentPack,
    action: &ActionDefinition,
    input: Option<&str>,
    context: &PlanningContext<'_>,
    planned: &mut PlannedTurn,
) -> bool {
    let metadata = action
        .player_command
        .as_ref()
        .unwrap_or_else(|| panic!("action '{}' should define player_command", action.id));
    if context.channel_surfing_only {
        planned.events.push(WorldEvent::UnknownInput {
            raw_input: context.raw_input.to_string(),
        });
        return false;
    }

    match metadata.target_mode {
        PlayerCommandTargetMode::ActorReference => {
            let remainder = input.unwrap_or_default();
            if let Some(resolved) = resolve_actor_reference_input(
                content,
                context.planner_state,
                context.current_room_id,
                remainder,
            ) {
                if resolved.actor_in_room {
                    planned.pending_dialogue = Some(pending_dialogue_for(
                        content,
                        context,
                        resolved.actor_id,
                        resolved.player_message,
                    ));
                    metadata.advances_time
                } else {
                    planned.events.push(WorldEvent::ActionRejected {
                        message: content.render_template(
                            &content.presentation.error_text.actor_not_here,
                            &[("actor_name", resolved.actor_name.as_str())],
                        ),
                    });
                    false
                }
            } else {
                planned.events.push(WorldEvent::ActionRejected {
                    message: content.render_template(
                        &content.presentation.error_text.actor_unknown,
                        &[("target", unknown_target_token(remainder).as_str())],
                    ),
                });
                false
            }
        }
        PlayerCommandTargetMode::FirstActorInRoom => {
            let actors_here: Vec<_> = content
                .actors
                .iter()
                .filter(|actor| {
                    context
                        .planner_state
                        .actor_room_id(&actor.id, &actor.room_id)
                        == context.current_room_id
                })
                .collect();
            if let Some(actor) = actors_here.first() {
                planned.pending_dialogue = Some(pending_dialogue_for(
                    content,
                    context,
                    actor.id.clone(),
                    None,
                ));
                metadata.advances_time
            } else {
                planned.events.push(WorldEvent::ActionRejected {
                    message: content
                        .render_message("error.no_actor_to_listen", &[])
                        .unwrap_or_default(),
                });
                false
            }
        }
        other => {
            panic!(
                "dialogue player command '{}' has unsupported target_mode '{other:?}'",
                action.id,
            )
        }
    }
}
