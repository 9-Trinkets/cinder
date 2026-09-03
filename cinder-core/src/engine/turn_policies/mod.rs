//! Turn-time rules that decide which actions are on an actor's turn menu
//! and how beats steer each actor's turn.
//!
//! Split by responsibility:
//! * [`availability`] — whether an action is currently available, and why not.
//! * [`objectives`] — "beat objective" guidance, affordance priorities, and
//!   the story-var tracking behind objective progress.

mod availability;
mod objectives;

pub use availability::action_is_available;

pub(crate) use availability::{
    command_availability_issue, command_unavailable_message, story_var_is_truthy,
};
pub(crate) use objectives::{
    actor_objective_guidance_notes, apply_actor_turn_policies,
    apply_command_objective_progress_effects, clear_inactive_objective_state,
    mark_actor_objective_progress_for_speech_event, ObjectiveSpeechEvent,
};
