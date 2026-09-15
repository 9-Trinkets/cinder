use super::CinderRuntime;
use super::stats_trace::stats_trace_snapshot;
use crate::engine::actor_tick::{ActorTickError, run_actor_tick};
use crate::engine::conversation_memory::refresh_conversation_summaries;
use crate::engine::events::{TimestampedWorldEvent, WorldEvent};
use crate::engine::narrative::NarrativeLines;
use crate::engine::neuron::WorkflowTraceContext;
use crate::engine::reducer::apply_events;
use crate::engine::state::{GamePhase, TurnOutcome};
use std::error::Error;
use std::sync::Arc;

impl CinderRuntime {
    pub fn run_tick(&self) -> Result<TurnOutcome, Box<dyn Error>> {
        let (phase_at_entry, turn_number) = {
            let state = self
                .state
                .lock()
                .map_err(|_| "failed to lock runtime state to start npc tick")?;
            (state.phase.clone(), state.turn_number)
        };
        let requires_first_action = !self.content.settings.channel_surfing_only
            && !self.content.settings.autonomous_actor_dialogue;
        if phase_at_entry != GamePhase::Active || (requires_first_action && turn_number == 0) {
            // A session that is already over must not re-emit or re-persist
            // the act-end narration: reconnecting realtime tickers would
            // otherwise grow the transcript by one line per visit.
            // Also, a fresh session (turn_number == 0) for player-driven packs
            // must not tick until the player has taken their first action,
            // protecting the opening narrative from wandering threats.
            return Ok(TurnOutcome {
                text: String::new(),
                phase: phase_at_entry,
                lines: Vec::new(),
            });
        }
        let outcome = match self.run_actor_turns() {
            Ok((text, phase, lines)) => TurnOutcome {
                text,
                phase,
                lines,
            },
            Err(error) => {
                if let Some(actor_tick_error) = error.downcast_ref::<ActorTickError>() {
                    eprintln!("[cinder] actor tick error: {}", actor_tick_error.message);
                    TurnOutcome {
                        text: self.actor_tick_soft_error_text(actor_tick_error),
                        phase: GamePhase::Active,
                        lines: Vec::new(),
                    }
                } else {
                    return Err(error);
                }
            }
        };
        let outcome = self.apply_stage_assignments(outcome)?;
        let outcome = match outcome.phase {
            GamePhase::ActEnded => {
                let ended_text = &self.content.presentation.presentation_text.act_ended;
                let text = if outcome.text.is_empty() {
                    ended_text.clone()
                } else {
                    format!("{}\n\n{}", outcome.text, ended_text)
                };
                TurnOutcome { text, ..outcome }
            }
            GamePhase::GameEnded => {
                let ended_text = &self.content.presentation.presentation_text.act_ended;
                let text = if outcome.text.is_empty() {
                    ended_text.clone()
                } else {
                    format!("{}\n\n{}", outcome.text, ended_text)
                };
                TurnOutcome { text, ..outcome }
            }
            GamePhase::Active => outcome,
        };
        if !outcome.text.is_empty() {
            self.push_transcript_line(&outcome.text).ok();
        }
        Ok(outcome)
    }

    fn actor_tick_soft_error_text(&self, error: &ActorTickError) -> String {
        let actor_name = error
            .trace_records
            .iter()
            .find_map(|trace| {
                (trace.role_name == "npc_actor_turn" && trace.topic == "workflow.error")
                    .then(|| {
                        trace
                            .payload
                            .get("actor_name")
                            .and_then(serde_json::Value::as_str)
                    })
                    .flatten()
            })
            .unwrap_or(&self.content.ui_text.follow_unknown_actor_name);
        self.content.render_template(
            &self.content.ui_text.npc_tick_soft_error,
            &[("actor_name", actor_name)],
        )
    }

    fn run_actor_turns(
        &self,
    ) -> Result<(String, GamePhase, Vec<crate::engine::narrative::NarrativeLine>), Box<dyn Error>> {
        let mut lines = NarrativeLines::default();
        let tracer = WorkflowTraceContext::new(self.trace_events, &self.trace_dir)?;
        tracer
            .emit(
                "npc_tick",
                "workflow.start",
                serde_json::json!({
                    "entry_role": "npc_tick",
                    "instruction": "tick",
                    "workflow": "cinder_npc_tick",
                }),
            )
            .map_err(std::io::Error::other)?;
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| "failed to lock runtime state to start npc tick")?;
            if state.phase != GamePhase::Active {
                let phase = state.phase.clone();
                return Ok((String::new(), phase, Vec::new()));
            }
            let tick_start = [TimestampedWorldEvent::now(WorldEvent::TurnStarted {
                turn_number: state.turn_number + 1,
                raw_input: "tick".to_string(),
                advances_time: true,
            })];
            let reduced = apply_events(&mut state, self.content.as_ref(), &tick_start);
            refresh_conversation_summaries(
                self.content.as_ref(),
                self.dialogue.as_ref(),
                &mut state,
            )
            .map_err(std::io::Error::other)?;
            lines.extend(reduced.lines.0);
        }
        let state_snapshot = {
            let state = self
                .state
                .lock()
                .map_err(|_| "failed to lock runtime state for npc turns")?;
            if state.phase != GamePhase::Active {
                let phase = state.phase.clone();
                return Ok((lines.to_text(), phase, lines.0));
            }
            state.clone()
        };
        tracer
            .emit(
                "npc_tick",
                "stats.state",
                serde_json::json!({
                    "phase": "before_tick",
                    "snapshot": stats_trace_snapshot(&state_snapshot),
                }),
            )
            .map_err(std::io::Error::other)?;
        let tick = match run_actor_tick(
            Arc::clone(&self.content),
            Arc::clone(&self.dialogue),
            &self.actor_tick_workflow,
            &state_snapshot,
        ) {
            Ok(tick) => tick,
            Err(error) => {
                for trace in &error.trace_records {
                    tracer
                        .emit(&trace.role_name, &trace.topic, trace.payload.clone())
                        .map_err(std::io::Error::other)?;
                }
                tracer
                    .emit(
                        "npc_tick",
                        "workflow.error",
                        serde_json::json!({
                            "workflow": "cinder_npc_tick",
                            "message": error.to_string(),
                            "stats": stats_trace_snapshot(&state_snapshot),
                        }),
                    )
                    .map_err(std::io::Error::other)?;
                return Err(Box::new(error));
            }
        };
        for trace in tick.trace_records {
            tracer
                .emit(&trace.role_name, &trace.topic, trace.payload)
                .map_err(std::io::Error::other)?;
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state to apply npc events")?;
        if state.phase != GamePhase::Active {
            let phase = state.phase.clone();
            return Ok((lines.to_text(), phase, lines.0));
        }
        let mut logged_events = tick
            .events
            .into_iter()
            .map(TimestampedWorldEvent::now)
            .collect::<Vec<_>>();
        logged_events.extend(
            crate::engine::actor_tick::plan_wander_moves(self.content.as_ref(), &state)
                .into_iter()
                .map(TimestampedWorldEvent::now),
        );
        logged_events.extend(
            crate::engine::actor_tick::plan_periodic_effect_events(self.content.as_ref(), &state)
                .into_iter()
                .map(TimestampedWorldEvent::now),
        );
        if !logged_events.is_empty() {
            let reduced = apply_events(&mut state, self.content.as_ref(), &logged_events);
            refresh_conversation_summaries(
                self.content.as_ref(),
                self.dialogue.as_ref(),
                &mut state,
            )
            .map_err(std::io::Error::other)?;
            lines.extend(reduced.lines.0);
        }
        drop(state);
        let final_state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state after npc turns")?
            .clone();
        let phase = final_state.phase.clone();
        let final_stats = stats_trace_snapshot(&final_state);
        tracer
            .emit(
                "npc_tick",
                "stats.state",
                serde_json::json!({
                    "phase": "after_tick",
                    "snapshot": final_stats.clone(),
                }),
            )
            .map_err(std::io::Error::other)?;
        tracer
            .emit(
                "npc_tick",
                "workflow.complete",
                serde_json::json!({
                    "phase": format!("{:?}", phase),
                    "text": lines.to_text(),
                    "stats": final_stats,
                }),
            )
            .map_err(std::io::Error::other)?;
        Ok((lines.to_text(), phase, lines.0))
    }
}
