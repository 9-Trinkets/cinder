use crate::content::types::ContentPack;
use crate::engine::dialogue_grounding::viewer_participant_id;
use crate::engine::hook_ids;
use crate::engine::hooks::apply_world_hook_effects;
use crate::engine::state::{ConversationMemoryKind, ConversationMemoryLine, WorldState};
use serde_json::json;

use super::beat_advance::advance_objective_for_signal;
use super::observation::actors_in_room;

pub(super) fn advance_actor_stats_on_tick(
    state: &mut WorldState,
    content: &ContentPack,
    previous_time_minutes: u32,
    current_time_minutes: u32,
) {
    let stepped_stats = content
        .stats
        .actor
        .iter()
        .filter_map(|(stat_key, definition)| {
            definition.time_step_minutes.map(|interval| {
                (
                    stat_key.as_str(),
                    crossed_interval_steps(previous_time_minutes, current_time_minutes, interval),
                )
            })
        })
        .filter(|(_, steps)| *steps > 0)
        .collect::<Vec<_>>();
    if stepped_stats.is_empty() {
        return;
    }
    for actor in &content.actors {
        for (stat_key, steps) in &stepped_stats {
            for _ in 0..*steps {
                apply_world_hook_effects(
                    state,
                    content,
                    hook_ids::ACTOR_TIME_ADVANCED,
                    json!({
                        "actor_id": &actor.id,
                        "previous_time_minutes": previous_time_minutes,
                        "current_time_minutes": current_time_minutes,
                        "stat": stat_key,
                    }),
                )
                .unwrap_or_else(|error| {
                    eprintln!("[cinder] hook warning (actor.time_advanced): {error}")
                });
            }
        }
    }
}

pub(super) fn crossed_interval_steps(
    previous_time_minutes: u32,
    current_time_minutes: u32,
    interval: u32,
) -> u32 {
    if interval == 0 || current_time_minutes <= previous_time_minutes {
        return 0;
    }
    (current_time_minutes / interval).saturating_sub(previous_time_minutes / interval)
}

pub(super) fn advance_house_progress_objectives(
    state: &mut WorldState,
    content: &ContentPack,
) -> Vec<String> {
    let met_everyone = all_actors_have_met_everyone(state, content);
    let learned_house = all_actors_have_learned_house(state, content);
    let mut messages = Vec::new();
    if met_everyone {
        messages.extend(advance_objective_for_signal(
            state,
            content,
            "all-actors-met-everyone",
        ));
    }
    if learned_house {
        messages.extend(advance_objective_for_signal(
            state,
            content,
            "all-actors-learned-house",
        ));
    }
    if met_everyone && learned_house {
        messages.extend(advance_objective_for_signal(
            state,
            content,
            "all-actors-met-everyone-and-learned-house",
        ));
    }
    messages
}

pub(super) fn increment_shared_room_safety(state: &mut WorldState, content: &ContentPack) {
    for actor in &content.actors {
        let room_id = state.actor_room_id(&actor.id, &actor.room_id).to_string();
        for other in content.actors.iter().filter(|other| other.id > actor.id) {
            let other_room_id = state.actor_room_id(&other.id, &other.room_id).to_string();
            if room_id == other_room_id {
                apply_world_hook_effects(
                    state,
                    content,
                    hook_ids::SHARED_ROOM_TICK,
                    json!({
                        "event_kind": "shared_room_tick",
                        "participant_a_id": actor.id,
                        "participant_b_id": other.id,
                    }),
                )
                .unwrap_or_else(|error| {
                    eprintln!("[cinder] hook warning (shared_room_tick): {error}")
                });
            }
        }
        if state.current_room_id == room_id {
            apply_world_hook_effects(
                state,
                content,
                hook_ids::SHARED_ROOM_TICK,
                json!({
                    "event_kind": "shared_room_tick",
                    "participant_a_id": actor.id,
                    "participant_b_id": viewer_participant_id(content),
                }),
            )
            .expect("shared room tick hook should evaluate");
        }
    }
}

pub(super) fn pending_reply_broken_by_move(
    state: &WorldState,
    actor_id: &str,
    from_room_id: &str,
) -> Option<crate::engine::state::PendingReplyState> {
    state
        .pending_replies
        .values()
        .find(|pending| {
            pending.turn_number + 1 == state.turn_number
                && pending.room_id == from_room_id
                && (pending.speaker_id == actor_id || pending.listener_id == actor_id)
        })
        .cloned()
}

pub(super) fn all_actors_have_met_everyone(state: &WorldState, content: &ContentPack) -> bool {
    content.actors.iter().enumerate().all(|(index, actor)| {
        content.actors.iter().skip(index + 1).all(|other| {
            state
                .conversation_history(&actor.id, &other.id)
                .iter()
                .any(|line| line.kind == ConversationMemoryKind::Speech)
        })
    })
}

pub(super) fn all_actors_have_learned_house(state: &WorldState, content: &ContentPack) -> bool {
    let room_ids = content
        .rooms
        .iter()
        .map(|room| room.id.as_str())
        .collect::<Vec<_>>();
    content.actors.iter().all(|actor| {
        room_ids
            .iter()
            .all(|room_id| state.actor_has_visited_room(&actor.id, room_id))
    })
}

pub(super) fn record_room_action_memory(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    actor_name: &str,
    room_id: &str,
    text: &str,
) {
    for other_actor in actors_in_room(content, state, room_id) {
        if other_actor.id == actor_id {
            continue;
        }
        state.push_conversation_line(
            actor_id,
            &other_actor.id,
            ConversationMemoryLine {
                turn_number: state.turn_number,
                event_sequence: 0,
                speaker_id: actor_id.to_string(),
                speaker_name: actor_name.to_string(),
                kind: ConversationMemoryKind::Action,
                target_label: Some("room".to_string()),
                text: text.to_string(),
            },
        );
    }
}
