mod beat_advance;
mod command_effects;
mod handlers;
mod observation;
mod tick;

use self::handlers::{
    apply_content_event, handle_action_rejected, handle_actor_command_used_event,
    handle_actor_moved, handle_actor_observed, handle_actor_observed_actor,
    handle_actor_observed_feature, handle_actor_observed_room, handle_actor_relocated,
    handle_actor_spoke, handle_actor_spoke_to_room, handle_current_room_observed,
    handle_feature_observed, handle_help_shown, handle_item_acquired, handle_item_consumed,
    handle_item_observed,
    handle_menu_choice_made, handle_menu_opened, handle_narrative_line, handle_pair_stat_adjusted,
    handle_player_moved, handle_session_ended, handle_turn_started, handle_unknown_input,
};

pub(crate) use self::observation::render_actor_speech_line;

use crate::content::types::ContentPack;
use crate::engine::events::{TimestampedWorldEvent, WorldEvent};
use crate::engine::state::WorldState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReducerOutput {
    pub lines: Vec<String>,
    pub game_over: bool,
}

pub fn apply_events(
    state: &mut WorldState,
    content: &ContentPack,
    events: &[TimestampedWorldEvent],
) -> ReducerOutput {
    let mut lines = Vec::new();
    for entry in events {
        match &entry.event {
            WorldEvent::TurnStarted {
                turn_number,
                advances_time,
                ..
            } => {
                handle_turn_started(state, content, *turn_number, *advances_time, &mut lines);
            }
            WorldEvent::CurrentRoomObserved { room_id, mode } => {
                if state.game_over {
                    continue;
                }
                handle_current_room_observed(state, content, room_id, mode.clone(), &mut lines);
            }
            WorldEvent::FeatureObserved {
                room_id,
                feature_id,
            } => {
                handle_feature_observed(state, content, room_id, feature_id, &mut lines);
            }
            WorldEvent::ActorObserved { actor_id } => {
                handle_actor_observed(state, content, actor_id, &mut lines);
            }
            WorldEvent::ActorSpoke {
                actor_id,
                actor_name,
                other_person_id,
                other_person_name,
                other_person_message,
                room_id,
                text,
            } => {
                handle_actor_spoke(
                    state,
                    content,
                    actor_id,
                    actor_name,
                    other_person_id,
                    other_person_name,
                    other_person_message,
                    room_id,
                    text,
                    &mut lines,
                );
            }
            WorldEvent::ActorSpokeToRoom {
                actor_id,
                actor_name,
                audience_actor_ids,
                room_id,
                text,
            } => {
                handle_actor_spoke_to_room(
                    state,
                    content,
                    actor_id,
                    actor_name,
                    audience_actor_ids,
                    room_id,
                    text,
                    &mut lines,
                );
            }
            WorldEvent::ActorStatAdjusted { actor_id, stat, delta } => {
                if let Err(e) = state.adjust_actor_stat(actor_id, stat, *delta) {
                    eprintln!("[cinder] ActorStatAdjusted error: {e}");
                }
            }
            WorldEvent::PairStatAdjusted {
                participant_a_id,
                participant_b_id,
                stat,
                delta,
            } => {
                handle_pair_stat_adjusted(state, participant_a_id, participant_b_id, stat, *delta);
            }
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
                    state,
                    content,
                    actor_id,
                    actor_name,
                    room_id,
                    command_id,
                    target_room_id.as_deref(),
                    target_actor_id.as_deref(),
                    target_actor_name.as_deref(),
                    context_label.as_deref(),
                    feature_id.as_deref(),
                    consumable_id.as_deref(),
                    freeform_text.as_deref(),
                    &mut lines,
                );
            }
            WorldEvent::ActorObservedRoom {
                actor_id,
                actor_name,
                room_id,
            } => {
                handle_actor_observed_room(
                    state, content, actor_id, actor_name, room_id, &mut lines,
                );
            }
            WorldEvent::ActorObservedFeature {
                actor_id,
                actor_name,
                room_id,
                feature_id,
            } => {
                handle_actor_observed_feature(
                    state, content, actor_id, actor_name, room_id, feature_id, &mut lines,
                );
            }
            WorldEvent::ActorObservedActor {
                actor_id,
                actor_name,
                room_id,
                target_actor_id,
                target_actor_name,
            } => {
                handle_actor_observed_actor(
                    state,
                    content,
                    actor_id,
                    actor_name,
                    room_id,
                    target_actor_id,
                    target_actor_name,
                    &mut lines,
                );
            }
            WorldEvent::ActorRelocated {
                actor_id,
                to_room_id,
            } => {
                handle_actor_relocated(state, content, actor_id, to_room_id, &mut lines);
            }
            WorldEvent::ActorMoved {
                actor_id,
                from_room_id,
                to_room_id,
                ..
            } => {
                handle_actor_moved(
                    state,
                    content,
                    actor_id,
                    from_room_id,
                    to_room_id,
                    &mut lines,
                );
            }
            WorldEvent::PlayerMoved { to_room_id, .. } => {
                handle_player_moved(state, content, to_room_id, &mut lines);
            }
            WorldEvent::MenuOpened { menu_id } => {
                handle_menu_opened(state, content, menu_id, &mut lines);
            }
            WorldEvent::MenuChoiceMade {
                menu_id,
                option_id,
                title,
            } => {
                handle_menu_choice_made(state, content, menu_id, option_id, title, &mut lines);
            }
            WorldEvent::NarrativeLine { text } => {
                handle_narrative_line(text, &mut lines);
            }
            WorldEvent::ActionRejected { message } => {
                handle_action_rejected(message, &mut lines);
            }
            WorldEvent::HelpShown => {
                handle_help_shown(state, content, &mut lines);
            }
            WorldEvent::UnknownInput { raw_input } => {
                handle_unknown_input(content, raw_input, &mut lines);
            }
            WorldEvent::SessionEnded => {
                handle_session_ended(state, content, &mut lines);
            }
            WorldEvent::ItemAcquired { item_id } => {
                handle_item_acquired(state, content, item_id, &mut lines);
            }
            WorldEvent::ItemConsumed {
                item_id,
                consumer_id,
                consumer_name,
            } => {
                handle_item_consumed(state, content, item_id, consumer_id, consumer_name, &mut lines);
            }
            WorldEvent::ItemObserved { item_id } => {
                handle_item_observed(state, content, item_id, &mut lines);
            }
            WorldEvent::ContentEvent { event_id, payload } => {
                apply_content_event(state, content, event_id, payload, &mut lines);
            }
        }
    }
    ReducerOutput {
        lines,
        game_over: state.game_over,
    }
}

#[cfg(test)]
mod tests;
