mod active_menu;
mod options;

use super::CinderRuntime;
use crate::engine::conversation_memory::refresh_conversation_summaries;
use crate::engine::events::{ObservationMode, TimestampedWorldEvent, WorldEvent};
use crate::engine::reducer::apply_events;
use crate::engine::state::{TurnOutcome, display_actor_name};
use std::error::Error;

impl CinderRuntime {
    pub fn switch_room_view(&self, room_id: &str) -> Result<TurnOutcome, Box<dyn Error>> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for room switching")?;
        let Some(room) = self.content.room(room_id) else {
            return Err(format!("missing room '{room_id}'").into());
        };
        let turn_number = state.turn_number + 1;
        let mut events = vec![TimestampedWorldEvent::now(WorldEvent::TurnStarted {
            turn_number,
            raw_input: format!("switch-room:{room_id}"),
            advances_time: false,
        })];
        if state.current_room_id != room.id {
            // Only allow moving to a currently-visible exit of the current room,
            // so a gated (hidden) exit can't be reached by bypassing the menu.
            let ok = self
                .content
                .resolve_exit_for(&state.current_room_id, room_id, |key| {
                    crate::engine::turn_policies::story_var_is_truthy(&state, key)
                })
                .is_some();
            if !ok {
                return Err(format!("'{room_id}' is not an open exit of the current room").into());
            }
            events.push(TimestampedWorldEvent::now(WorldEvent::PlayerMoved {
                from_room_id: state.current_room_id.clone(),
                to_room_id: room.id.clone(),
            }));
        }
        events.push(TimestampedWorldEvent::now(
            WorldEvent::CurrentRoomObserved {
                room_id: room.id.clone(),
                mode: ObservationMode::Summary,
            },
        ));
        let reduced = apply_events(&mut state, self.content.as_ref(), &events);
        refresh_conversation_summaries(self.content.as_ref(), self.dialogue.as_ref(), &mut state)
            .map_err(std::io::Error::other)?;
        Ok(TurnOutcome {
            text: reduced.lines.to_text(),
            phase: reduced.phase,
            lines: reduced.lines.0,
        })
    }

    pub fn follow_actor(&self, actor_id: Option<&str>) -> Result<TurnOutcome, Box<dyn Error>> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for following actor")?;
        let turn_number = state.turn_number + 1;
        let feedback_line = format!(
            "> {}",
            match actor_id {
                Some(actor_id) => {
                    let actor = self
                        .content
                        .actor(actor_id)
                        .ok_or_else(|| format!("missing actor '{actor_id}'"))?;
                    let actor_name = display_actor_name(&state, actor);
                    self.content
                        .ui_text
                        .follow_actor_transcript
                        .replace("{title}", &actor_name)
                }
                None => self.content.ui_text.follow_actor_stop_transcript.clone(),
            }
        );
        let mut events = vec![TimestampedWorldEvent::now(WorldEvent::TurnStarted {
            turn_number,
            raw_input: format!("follow:{}", actor_id.unwrap_or("none")),
            advances_time: false,
        })];
        match actor_id {
            Some(actor_id) => {
                let actor = self
                    .content
                    .actor(actor_id)
                    .ok_or_else(|| format!("missing actor '{actor_id}'"))?;
                state.followed_actor_id = Some(actor_id.to_string());
                let room_id = state.actor_room_id(actor_id, &actor.room_id).to_string();
                if state.current_room_id != room_id {
                    events.push(TimestampedWorldEvent::now(WorldEvent::PlayerMoved {
                        from_room_id: state.current_room_id.clone(),
                        to_room_id: room_id.clone(),
                    }));
                    events.push(TimestampedWorldEvent::now(
                        WorldEvent::CurrentRoomObserved {
                            room_id,
                            mode: ObservationMode::Summary,
                        },
                    ));
                }
            }
            None => {
                state.followed_actor_id = None;
            }
        }
        let reduced = apply_events(&mut state, self.content.as_ref(), &events);
        refresh_conversation_summaries(self.content.as_ref(), self.dialogue.as_ref(), &mut state)
            .map_err(std::io::Error::other)?;
        let mut lines = crate::engine::narrative::NarrativeLines::default();
        lines.narration(feedback_line);
        lines.extend(reduced.lines.0);
        Ok(TurnOutcome {
            text: lines.to_text(),
            phase: reduced.phase,
            lines: lines.0,
        })
    }
}
