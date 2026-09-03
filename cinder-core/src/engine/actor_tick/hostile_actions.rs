use super::HostilityStageEnvelope;
use super::workflow::{
    ActorTickRoleRunner, ActorTickWorkflowState, extract_inbound_message, route_tick_workflow,
};
use crate::content::types::{AutonomousHostilityMode, ContentPack};
use crate::engine::dialogue::{HostilityCandidate, HostilityPlanRequest};
use crate::engine::events::WorldEvent;
use crate::engine::hostile_actions::plan_rules_hostile_actions;
use crate::engine::state::WorldState;

impl ActorTickRoleRunner {
    pub(super) fn handle_hostility_decide(&self, prompt: &str) -> Result<String, String> {
        let inbound = extract_inbound_message(prompt)?;
        let mut workflow_state: ActorTickWorkflowState =
            serde_json::from_str(&inbound).map_err(|error| error.to_string())?;
        // behavior.json defines the eligible strikes. The mode only chooses
        // whether rules apply all of them or an LLM selects a validated subset.
        let eligible_events =
            plan_rules_hostile_actions(self.content.as_ref(), &workflow_state.state);
        let events = if matches!(
            self.content.settings.autonomous_hostility_mode,
            AutonomousHostilityMode::Llm
        ) && !eligible_events.is_empty()
        {
            let request = build_hostility_plan_request(
                self.content.as_ref(),
                &workflow_state.state,
                &eligible_events,
            );
            self.emit_trace(
                "world_hostility",
                "plan.request",
                serde_json::to_value(&request).map_err(|error| error.to_string())?,
            )?;
            let decision = self
                .dialogue
                .plan_hostility_actions(&request)
                .map_err(|error| {
                    let _ = self.emit_trace(
                        "world_hostility",
                        "workflow.error",
                        serde_json::json!({ "message": error }),
                    );
                    error
                })?;
            self.emit_trace(
                "world_hostility",
                "plan.decision",
                serde_json::json!({ "strikes": decision.strikes }),
            )?;
            decision
                .strikes
                .into_iter()
                .map(|actor_id| WorldEvent::HostileStrike { actor_id })
                .collect()
        } else {
            eligible_events
        };
        workflow_state.hostility_stage = HostilityStageEnvelope::Decided { events };
        route_tick_workflow("world_hostility_apply", &workflow_state)
    }

    pub(super) fn handle_hostility_apply(&self, prompt: &str) -> Result<String, String> {
        let inbound = extract_inbound_message(prompt)?;
        let mut workflow_state: ActorTickWorkflowState =
            serde_json::from_str(&inbound).map_err(|error| error.to_string())?;
        let events = match std::mem::replace(
            &mut workflow_state.hostility_stage,
            HostilityStageEnvelope::Idle,
        ) {
            HostilityStageEnvelope::Decided { events } => events,
            _ => {
                return Err(
                    "world_hostility_apply expected decided hostility stage envelope".to_string(),
                );
            }
        };
        self.apply_and_refresh(&mut workflow_state.state, &events)?;
        workflow_state.emitted_events.extend(events);
        workflow_state.hostility_stage = HostilityStageEnvelope::Applied;
        route_tick_workflow("npc_tick_orchestrator", &workflow_state)
    }
}

fn build_hostility_plan_request(
    content: &ContentPack,
    state: &WorldState,
    eligible_events: &[WorldEvent],
) -> HostilityPlanRequest {
    let current_time_minutes = state.current_time_minutes;
    let candidates = eligible_events
        .iter()
        .filter_map(|event| {
            let WorldEvent::HostileStrike { actor_id } = event else {
                return None;
            };
            Some(actor_id)
        })
        .map(|actor_id| {
            let actor = content.actor(actor_id);
            let due_at = *state
                .next_hostile_strike_at
                .get(actor_id)
                .unwrap_or(&current_time_minutes);
            let interval = actor
                .map(|actor| {
                    actor.attack_interval_minutes(
                        content.settings.combat.default_attack_interval_minutes,
                    )
                })
                .unwrap_or(content.settings.combat.default_attack_interval_minutes);
            HostilityCandidate {
                actor_id: actor_id.clone(),
                actor_name: actor
                    .map(|actor| actor.name.clone())
                    .unwrap_or_else(|| actor_id.clone()),
                room_id: state
                    .actor_room_id(
                        actor_id,
                        &actor.map(|actor| actor.room_id.clone()).unwrap_or_default(),
                    )
                    .to_string(),
                hp: state.actor_stat(actor_id, &content.settings.combat.health_stat_id),
                strength: state.actor_stat(actor_id, &content.settings.combat.attack_stat_id),
                minutes_since_last_strike: current_time_minutes
                    .saturating_sub(due_at.saturating_sub(interval)),
                attack_interval_minutes: interval,
            }
        })
        .collect();
    HostilityPlanRequest {
        player_room_id: state.current_room_id.to_string(),
        player_hp: state.actor_stat(
            &content.settings.combat.player_actor_id,
            &content.settings.combat.health_stat_id,
        ),
        candidates,
        system_prompt: content.system_text.hostility_planner_system_prompt.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_candidates_come_only_from_behavior_selected_events() {
        let content = crate::engine::test_fixtures::minimal_test_pack();
        let mut state = WorldState::new(&content);
        state.current_room_id = "kitchen".to_string();
        let selected_actor_id = content.actors[0].id.clone();

        let request = build_hostility_plan_request(
            &content,
            &state,
            &[WorldEvent::HostileStrike {
                actor_id: selected_actor_id.clone(),
            }],
        );

        assert_eq!(request.candidates.len(), 1);
        assert_eq!(request.candidates[0].actor_id, selected_actor_id);
    }
}
