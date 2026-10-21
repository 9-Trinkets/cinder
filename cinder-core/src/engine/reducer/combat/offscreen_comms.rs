use std::collections::BTreeSet;

use crate::content::types::ContentPack;
use crate::engine::narrative::{CommsMilestone, NarrativeLines, PendingCommsUpgrade};
use crate::engine::state::{ActorStance, WorldState};

use super::actor_display_name;

/// Whether an actor possesses the ability to transmit tactical comms dispatches.
/// This includes named allies with the authored `comms` skill and any party member
/// who has undergone awakening (transformation).
pub fn actor_can_use_comms(state: &WorldState, actor_id: &str) -> bool {
    state.actor_has_skill(actor_id, "comms") || state.is_actor_awakened(actor_id)
}

/// Selects a single primary reporter for a room:
/// 1. The first actor among candidates possessing the `comms` skill, prioritizing
///    authored order in the content pack.
/// 2. Any other candidate possessing the `comms` skill.
/// 3. Fallback to the first candidate.
pub fn select_reporter<'a>(
    state: &WorldState,
    candidates: &[&'a str],
    content: &'a ContentPack,
) -> Option<&'a str> {
    if candidates.is_empty() {
        return None;
    }
    if let Some(id) = content
        .actors
        .iter()
        .map(|a| a.id.as_str())
        .find(|&id| candidates.contains(&id) && state.actor_has_skill(id, "comms"))
    {
        return Some(id);
    }
    if let Some(&id) = candidates
        .iter()
        .find(|&&id| state.actor_has_skill(id, "comms"))
    {
        return Some(id);
    }
    candidates.first().copied()
}

fn format_milestone_fallback(
    content: &ContentPack,
    reporter_id: &str,
    milestone: CommsMilestone,
    room_name: &str,
    fallen_name: Option<&str>,
    enemies_remaining: usize,
) -> String {
    let persona = if reporter_id == "commander_astrid" || reporter_id == "astrid" {
        "astrid"
    } else if reporter_id == "einar" {
        "einar"
    } else {
        "default"
    };
    let milestone_key = match milestone {
        CommsMilestone::Contact => "contact",
        CommsMilestone::LowHealth => "low_health",
        CommsMilestone::AllyDown => "ally_down",
        CommsMilestone::AreaCleared => "area_cleared",
        CommsMilestone::PeriodicStatus => "periodic_status",
    };
    let fallen = fallen_name
        .map(str::to_string)
        .or_else(|| content.render_message("comms.ally_down.unnamed", &[]))
        .unwrap_or_default();
    let remaining = enemies_remaining.to_string();
    content
        .render_message(
            &format!("comms.{milestone_key}.{persona}"),
            &[
                ("room_name", room_name),
                ("fallen", &fallen),
                ("enemies_remaining", &remaining),
            ],
        )
        .unwrap_or_default()
}

/// Evaluates offscreen combat states across all non-player rooms where party members are stationed,
/// emitting autonomous radio dispatches for high-stakes tactical milestones.
pub fn evaluate_offscreen_combat_dispatches(
    state: &mut WorldState,
    content: &ContentPack,
    lines: &mut NarrativeLines,
) {
    let health_stat = &content.settings.combat.health_stat_id;
    let current_room_id = state.current_room_id.clone();

    // Identify candidate rooms: rooms containing allied party members or with tracked offscreen combat
    let mut rooms_to_check: BTreeSet<String> = state
        .relations
        .relationships
        .keys()
        .filter(|actor_id| {
            state.is_party_member(content, actor_id) && state.actor_stat(actor_id, health_stat) > 0
        })
        .map(|actor_id| state.actor_current_room_id(content, actor_id).to_string())
        .filter(|room_id| room_id != &current_room_id)
        .collect();

    rooms_to_check.extend(
        state
            .offscreen_combat_states
            .keys()
            .filter(|&room_id| room_id != &current_room_id)
            .cloned(),
    );

    // If the player moved into an offscreen combat room, clear its tracked offscreen state
    state.offscreen_combat_states.remove(&current_room_id);

    for room_id in rooms_to_check {
        let living_party_ids: Vec<String> = state
            .relations
            .relationships
            .keys()
            .filter(|id| {
                state.is_party_member(content, id)
                    && state.actor_stat(id, health_stat) > 0
                    && state.actor_current_room_id(content, id) == room_id
            })
            .cloned()
            .collect();

        // Hostile actors alive in this room
        let living_hostiles: Vec<String> = content
            .actors
            .iter()
            .chain(state.actors.spawned_actors.values())
            .map(|a| a.id.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .filter(|id| {
                state.stance(id) == ActorStance::Hostile
                    && state.actor_stat(id, health_stat) > 0
                    && !state.actor_is_offstage(content, id)
                    && state.actor_current_room_id(content, id) == room_id
            })
            .collect();

        // If no party members remain alive in this room, no one can report or fight
        if living_party_ids.is_empty() {
            state.offscreen_combat_states.remove(&room_id);
            continue;
        }

        // Check if any living party member has the comms ability
        let mut comms_candidates: Vec<&str> = living_party_ids
            .iter()
            .filter(|id| actor_can_use_comms(state, id))
            .map(|s| s.as_str())
            .collect();
        comms_candidates.sort();

        if comms_candidates.is_empty() {
            if living_hostiles.is_empty() {
                state.offscreen_combat_states.remove(&room_id);
            }
            continue;
        }

        let reporter_id =
            select_reporter(state, &comms_candidates, content).expect("reporter candidate");
        let reporter_name = actor_display_name(state, content, reporter_id);
        let room_name = content
            .room(&room_id)
            .map(|r| r.title.clone())
            .unwrap_or_else(|| room_id.clone());
        let ally_names: Vec<String> = living_party_ids
            .iter()
            .map(|id| actor_display_name(state, content, id))
            .collect();

        // Check if combat was active and now cleared
        if living_hostiles.is_empty() {
            if let Some(combat_state) = state.offscreen_combat_states.get(&room_id)
                && (combat_state.contact_dispatched || combat_state.engaged_at_turn > 0)
            {
                let fallback = format_milestone_fallback(
                    content,
                    reporter_id,
                    CommsMilestone::AreaCleared,
                    &room_name,
                    None,
                    0,
                );
                lines.comms_dispatch(
                    format!("{reporter_name}: {fallback}"),
                    PendingCommsUpgrade {
                        reporter_id: reporter_id.to_string(),
                        reporter_name,
                        room_id: room_id.clone(),
                        room_name,
                        milestone: CommsMilestone::AreaCleared,
                        fallback_text: fallback,
                        enemies_remaining: 0,
                        ally_names,
                    },
                );
            }
            state.offscreen_combat_states.remove(&room_id);
            continue;
        }

        // Milestone B check: Low Health (< 30% or <= 6 HP)
        let any_critical = living_party_ids.iter().any(|id| {
            let hp = state.actor_stat(id, health_stat);
            let initial_hp = content
                .actor(id)
                .map(|a| a.initial_stats.get(health_stat).copied().unwrap_or(20))
                .unwrap_or(20);
            hp <= (initial_hp * 30 / 100).max(6)
        });

        // Active combat: living_hostiles is non-empty
        let combat_state = state
            .offscreen_combat_states
            .entry(room_id.clone())
            .or_default();
        if combat_state.engaged_at_turn == 0 {
            combat_state.engaged_at_turn = state.clock.turn_number;
        }

        // Milestone A: Ally Down
        if !combat_state.newly_fallen_allies.is_empty() {
            let fallen_name = combat_state.newly_fallen_allies.remove(0);
            let fallback = format_milestone_fallback(
                content,
                reporter_id,
                CommsMilestone::AllyDown,
                &room_name,
                Some(&fallen_name),
                living_hostiles.len(),
            );
            combat_state.last_dispatch_turn = state.clock.turn_number;
            combat_state.contact_dispatched = true;
            lines.comms_dispatch(
                format!("{reporter_name}: {fallback}"),
                PendingCommsUpgrade {
                    reporter_id: reporter_id.to_string(),
                    reporter_name,
                    room_id: room_id.clone(),
                    room_name,
                    milestone: CommsMilestone::AllyDown,
                    fallback_text: fallback,
                    enemies_remaining: living_hostiles.len(),
                    ally_names,
                },
            );
            continue;
        }

        if any_critical && !combat_state.low_health_dispatched {
            combat_state.low_health_dispatched = true;
            combat_state.contact_dispatched = true;
            combat_state.last_dispatch_turn = state.clock.turn_number;
            let fallback = format_milestone_fallback(
                content,
                reporter_id,
                CommsMilestone::LowHealth,
                &room_name,
                None,
                living_hostiles.len(),
            );
            lines.comms_dispatch(
                format!("{reporter_name}: {fallback}"),
                PendingCommsUpgrade {
                    reporter_id: reporter_id.to_string(),
                    reporter_name,
                    room_id: room_id.clone(),
                    room_name,
                    milestone: CommsMilestone::LowHealth,
                    fallback_text: fallback,
                    enemies_remaining: living_hostiles.len(),
                    ally_names,
                },
            );
            continue;
        }

        // Milestone C: Contact
        if !combat_state.contact_dispatched {
            combat_state.contact_dispatched = true;
            combat_state.last_dispatch_turn = state.clock.turn_number;
            let fallback = format_milestone_fallback(
                content,
                reporter_id,
                CommsMilestone::Contact,
                &room_name,
                None,
                living_hostiles.len(),
            );
            lines.comms_dispatch(
                format!("{reporter_name}: {fallback}"),
                PendingCommsUpgrade {
                    reporter_id: reporter_id.to_string(),
                    reporter_name,
                    room_id: room_id.clone(),
                    room_name,
                    milestone: CommsMilestone::Contact,
                    fallback_text: fallback,
                    enemies_remaining: living_hostiles.len(),
                    ally_names,
                },
            );
            continue;
        }

        // Milestone D: Periodic Status (every 3 turns)
        if combat_state.contact_dispatched
            && state.clock.turn_number >= combat_state.last_dispatch_turn.saturating_add(3)
        {
            combat_state.last_dispatch_turn = state.clock.turn_number;
            let fallback = format_milestone_fallback(
                content,
                reporter_id,
                CommsMilestone::PeriodicStatus,
                &room_name,
                None,
                living_hostiles.len(),
            );
            lines.comms_dispatch(
                format!("{reporter_name}: {fallback}"),
                PendingCommsUpgrade {
                    reporter_id: reporter_id.to_string(),
                    reporter_name,
                    room_id: room_id.clone(),
                    room_name,
                    milestone: CommsMilestone::PeriodicStatus,
                    fallback_text: fallback,
                    enemies_remaining: living_hostiles.len(),
                    ally_names,
                },
            );
            continue;
        }
    }
}
