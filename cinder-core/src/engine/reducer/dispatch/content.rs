use super::super::actor_commands::ActorCommandContext;
use super::super::beat_advance;
use super::super::context::ReduceCtx;
use super::super::handlers::{apply_content_event, handle_actor_command_used_event};
use crate::engine::events::WorldEvent;
use crate::engine::state::GamePhase;
use crate::engine::turn_policies::apply_command_objective_progress_effects;

pub(super) fn dispatch(ctx: &mut ReduceCtx<'_>, event: &WorldEvent) -> bool {
    match event {
        WorldEvent::ActorCommandUsed {
            actor_id,
            actor_name,
            room_id,
            command_id,
            target_room_id,
            target_actor_id,
            target_actor_name,
            context_label,
            feature_id,
            consumable_id,
            freeform_text,
        } => {
            handle_actor_command_used_event(
                ctx,
                command_id,
                &ActorCommandContext {
                    actor_id,
                    actor_name,
                    room_id,
                    target_room_id: target_room_id.as_deref(),
                    target_actor_id: target_actor_id.as_deref(),
                    target_actor_name: target_actor_name.as_deref(),
                    context_label: context_label.as_deref(),
                    feature_id: feature_id.as_deref(),
                    consumable_id: consumable_id.as_deref(),
                    freeform_text: freeform_text.as_deref(),
                },
            );
            true
        }
        WorldEvent::ContentEvent { event_id, payload } => {
            apply_content_event(ctx, event_id, payload);
            true
        }
        WorldEvent::CommandObjectiveProgressApplied { command_id } => {
            if let Some(command) = ctx.content.command(command_id) {
                apply_command_objective_progress_effects(&mut *ctx.state, command);
            }
            true
        }
        WorldEvent::StoryVarSet { key, value } => {
            ctx.state.story_vars.set_unchecked(key, value);
            true
        }
        WorldEvent::ActorRelationshipUpdated {
            actor_id,
            relationship,
        } => {
            if ctx.state.phase == GamePhase::Active {
                ctx.state.set_relationship(actor_id, *relationship);
            }
            true
        }
        WorldEvent::StageEffectApplied { effect } => {
            let mut effect_lines = Vec::new();
            beat_advance::apply_stage_effects(
                &mut *ctx.state,
                ctx.content,
                std::slice::from_ref(effect),
                &mut effect_lines,
            );
            ctx.lines.extend_lines(effect_lines);
            true
        }
        WorldEvent::ScriptedSequenceStepPlayed {
            sequence_id,
            next_step,
            finished,
        } => {
            ctx.state
                .apply_scripted_sequence_progress(sequence_id, *next_step, *finished);
            true
        }
        _ => false,
    }
}
