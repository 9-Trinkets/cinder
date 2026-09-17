//! Event handlers for the reducer, grouped by domain.
//!
//! The reducer's `apply_events` dispatches each `WorldEvent` to a `handle_*`
//! function here. Handlers are split by responsibility so no single file
//! becomes a dumping ground:
//! * `combat` — hostile strikes and stat damage.
//! * `combat_reactions` — post-damage party reactions and narration.
//! * `speech` — dialogue memory and speech-triggered objectives.
//! * `observation` — inspecting rooms, features, and actors.
//! * `movement` — placement, relocation, and follower syncing.
//! * `menus` — menu session state and selection resolution.
//! * `items` — item acquisition, movement, and consumption.
//! * `lifecycle` — turn/time and act/session progression.
//! * `feedback` — direct player-facing output (narration, errors, help).
//! * `directives` — resolving externally-directed commands and content events.

mod combat;
mod combat_reactions;
mod directives;
mod feedback;
mod items;
mod lifecycle;
mod menus;
mod movement;
mod observation;
mod party;
mod speech;

pub(super) use combat::{handle_hostile_strike, handle_pair_stat_adjusted};
pub(super) use directives::{apply_content_event, handle_actor_command_used_event};
pub(super) use feedback::{
    handle_action_rejected, handle_help_shown, handle_narrative_line, handle_unknown_input,
};
pub(crate) use feedback::{handler_attributed_line, push_message, push_rendered_message};
pub(super) use items::{
    handle_item_acquired, handle_item_consumed, handle_item_observed, handle_player_dropped_item,
    handle_player_took_item,
};
pub(super) use lifecycle::{handle_act_ended, handle_turn_started};
pub(super) use menus::{
    handle_menu_choice_made, handle_menu_opened, handle_menu_selection_toggled,
};
pub(super) use movement::{
    handle_actor_moved, handle_actor_relocated, handle_player_moved, sync_followers_to_room,
};
pub(super) use observation::{
    ActorObservationContext, handle_actor_observed, handle_actor_observed_actor,
    handle_actor_observed_feature, handle_actor_observed_room, handle_current_room_observed,
    handle_feature_observed,
};
pub(super) use party::{handle_party_order_assigned, handle_player_followed_actor};
pub(super) use speech::handle_channel_message;
