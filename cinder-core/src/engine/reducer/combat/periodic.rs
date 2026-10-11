use crate::content::types::{
    ContentPack, ItemStorageTarget, PeriodicActorEffect, PeriodicActorEffectDefinition,
};
use crate::engine::narrative::NarrativeLines;
use crate::engine::state::{GamePhase, WorldState};

use super::super::context::ReduceCtx;
use super::{actor_display_name, defeat_actor, defeat_player_if_dead};

pub(in crate::engine::reducer) fn handle_periodic_actor_effect_applied(
    ctx: &mut ReduceCtx<'_>,
    actor_id: &str,
    effect_id: &str,
) {
    let state = &mut *ctx.state;
    let content = ctx.content;
    let lines = &mut *ctx.lines;
    let Some(definition) = content
        .settings
        .periodic_actor_effects
        .iter()
        .find(|definition| definition.id == effect_id)
    else {
        return;
    };
    if state.phase != GamePhase::Active
        || !crate::engine::actor_tick::periodic_effect_target_matches(
            content,
            state,
            actor_id,
            definition.targets,
        )
    {
        return;
    }
    let room_id = state.actor_current_room_id(content, actor_id).to_string();
    if !state.has_item_in_storage(
        &definition.trigger.room_item,
        ItemStorageTarget::CurrentRoom,
        &room_id,
    ) {
        return;
    }
    let PeriodicActorEffect::Damage { amount } = &definition.effect;
    if *amount <= 0 {
        return;
    }
    let health_stat_id = &content.settings.combat.health_stat_id;
    if let Err(error) = state.adjust_actor_stat(content, actor_id, health_stat_id, -*amount) {
        eprintln!("[cinder] periodic effect stat error ({effect_id}, {actor_id}): {error}");
        return;
    }
    let remaining = state.actor_stat(actor_id, health_stat_id);
    let actor_name = actor_display_name(state, content, actor_id);

    let is_current_room = room_id == state.current_room_id;
    let is_adjacent = !is_current_room
        && content
            .adjacent_room_ids(&state.current_room_id)
            .iter()
            .any(|r| r == &room_id);
    let is_boss = is_boss_actor(content, actor_id);
    let room_title = content
        .room(&room_id)
        .map(|r| r.title.as_str())
        .unwrap_or(&room_id);
    let exit_direction = if is_adjacent {
        resolve_exit_direction(content, &state.current_room_id, &room_id)
    } else {
        None
    };

    if is_current_room {
        if let Some(line) = content.render_message(
            &definition.message,
            &[
                ("actor", actor_name.as_str()),
                ("damage", amount.to_string().as_str()),
                ("remaining", remaining.to_string().as_str()),
                ("effect_id", effect_id),
                ("room", room_title),
            ],
        ) {
            lines.narration(line);
        }
    } else if remaining > 0 {
        maybe_emit_ambient_cue(
            state,
            content,
            definition,
            &actor_name,
            &room_id,
            room_title,
            exit_direction,
            is_adjacent,
            is_boss,
            lines,
        );
    }

    if remaining <= 0 {
        if actor_id == content.settings.combat.player_actor_id {
            defeat_player_if_dead(state, content, lines);
        } else {
            defeat_actor(state, content, actor_id, &room_id, lines);
        }
        if !is_current_room {
            maybe_emit_offscreen_kill_cue(
                state,
                content,
                definition,
                &room_id,
                room_title,
                exit_direction,
                is_adjacent,
                lines,
            );
        }
    }

    handle_trigger_depletion(
        state,
        content,
        definition,
        &room_id,
        room_title,
        exit_direction,
        is_current_room,
        is_adjacent,
        lines,
    );
}

fn is_boss_actor(content: &ContentPack, actor_id: &str) -> bool {
    actor_id == "fire-elemental"
        || content
            .actor(actor_id)
            .map(|a| a.tags.iter().any(|t| t == "boss"))
            .unwrap_or(false)
}

fn resolve_exit_direction<'a>(
    content: &'a ContentPack,
    from_room_id: &str,
    to_room_id: &str,
) -> Option<&'a str> {
    content
        .room(from_room_id)?
        .exits
        .iter()
        .find(|e| e.room_id == to_room_id)
        .map(|e| e.label.as_str())
}

fn format_direction_phrase(raw: &str) -> String {
    let lower = raw.trim().to_ascii_lowercase();
    match lower.as_str() {
        "up" => "above".to_string(),
        "down" => "below".to_string(),
        _ if lower.starts_with("the ") => lower,
        _ => format!("the {lower}"),
    }
}

fn maybe_emit_ambient_cue(
    state: &mut WorldState,
    content: &ContentPack,
    definition: &PeriodicActorEffectDefinition,
    actor_name: &str,
    room_id: &str,
    room_title: &str,
    exit_direction: Option<&str>,
    is_adjacent: bool,
    is_boss: bool,
    lines: &mut NarrativeLines,
) {
    let kill_key = format!("{room_id}::{}::kill", definition.id);
    if state.periodic_effects_reported.get(&kill_key) == Some(&state.turn_number) {
        return;
    }
    let report_key = if is_boss {
        format!("{room_id}::{}::boss", definition.id)
    } else {
        format!("{room_id}::{}", definition.id)
    };
    if state.periodic_effects_reported.get(&report_key) == Some(&state.turn_number) {
        return;
    }

    let cue = if is_adjacent {
        if is_boss {
            render_adjacent_boss_cue(content, definition, actor_name, exit_direction, room_title)
        } else {
            render_adjacent_cue(content, definition, exit_direction, room_title)
        }
    } else if is_boss {
        render_distant_boss_cue(content, definition, actor_name, room_title)
    } else {
        render_distant_cue(content, definition, room_title)
    };

    if let Some(line) = cue {
        lines.narration(line);
        state
            .periodic_effects_reported
            .insert(report_key, state.turn_number);
    }
}

fn maybe_emit_offscreen_kill_cue(
    state: &mut WorldState,
    content: &ContentPack,
    definition: &PeriodicActorEffectDefinition,
    room_id: &str,
    room_title: &str,
    exit_direction: Option<&str>,
    is_adjacent: bool,
    lines: &mut NarrativeLines,
) {
    let kill_key = format!("{room_id}::{}::kill", definition.id);
    if state.periodic_effects_reported.get(&kill_key) == Some(&state.turn_number) {
        return;
    }

    let cue = if is_adjacent {
        render_adjacent_kill_cue(content, definition, exit_direction, room_title)
    } else {
        render_distant_kill_cue(content, definition, room_title)
    };

    if let Some(line) = cue {
        lines.narration(line);
        state
            .periodic_effects_reported
            .insert(kill_key, state.turn_number);
        // Also suppress non-lethal trigger cue for this room this turn
        state
            .periodic_effects_reported
            .insert(format!("{room_id}::{}", definition.id), state.turn_number);
    }
}

fn render_adjacent_kill_cue(
    content: &ContentPack,
    definition: &PeriodicActorEffectDefinition,
    exit_direction: Option<&str>,
    room_title: &str,
) -> Option<String> {
    let item_label = content.item_label(&definition.trigger.room_item);
    if let Some(direction_raw) = exit_direction {
        let direction = format_direction_phrase(direction_raw);
        let default_key = format!("combat.{}_adjacent_kill", definition.id);
        let key = definition
            .adjacent_kill_message
            .as_deref()
            .unwrap_or(&default_key);
        if let Some(line) = content.render_message(
            key,
            &[
                ("direction", direction.as_str()),
                ("room", room_title),
                ("item", item_label),
            ],
        ) {
            return Some(line);
        }
    }
    let fallback_key = format!("combat.{}_adjacent_kill_no_exit", definition.id);
    content.render_message(&fallback_key, &[("room", room_title), ("item", item_label)])
}

fn render_distant_kill_cue(
    content: &ContentPack,
    definition: &PeriodicActorEffectDefinition,
    room_title: &str,
) -> Option<String> {
    let item_label = content.item_label(&definition.trigger.room_item);
    let default_key = format!("combat.{}_distant_kill", definition.id);
    let key = definition
        .distant_kill_message
        .as_deref()
        .unwrap_or(&default_key);
    content.render_message(key, &[("room", room_title), ("item", item_label)])
}

fn render_adjacent_cue(
    content: &ContentPack,
    definition: &PeriodicActorEffectDefinition,
    exit_direction: Option<&str>,
    room_title: &str,
) -> Option<String> {
    let item_label = content.item_label(&definition.trigger.room_item);
    if let Some(direction_raw) = exit_direction {
        let direction = format_direction_phrase(direction_raw);
        let default_key = format!("combat.{}_adjacent", definition.id);
        let key = definition
            .adjacent_message
            .as_deref()
            .unwrap_or(&default_key);
        if let Some(line) = content.render_message(
            key,
            &[
                ("direction", direction.as_str()),
                ("room", room_title),
                ("item", item_label),
            ],
        ) {
            return Some(line);
        }
    }
    let fallback_key = format!("combat.{}_adjacent_no_exit", definition.id);
    content.render_message(&fallback_key, &[("room", room_title), ("item", item_label)])
}

fn render_adjacent_boss_cue(
    content: &ContentPack,
    definition: &PeriodicActorEffectDefinition,
    actor_name: &str,
    exit_direction: Option<&str>,
    room_title: &str,
) -> Option<String> {
    let item_label = content.item_label(&definition.trigger.room_item);
    if let Some(direction_raw) = exit_direction {
        let direction = format_direction_phrase(direction_raw);
        let default_key = format!("combat.{}_adjacent_boss", definition.id);
        let key = definition.boss_message.as_deref().unwrap_or(&default_key);
        if let Some(line) = content.render_message(
            key,
            &[
                ("actor", actor_name),
                ("direction", direction.as_str()),
                ("room", room_title),
                ("item", item_label),
            ],
        ) {
            return Some(line);
        }
    }
    let fallback_key = format!("combat.{}_adjacent_boss_no_exit", definition.id);
    content.render_message(
        &fallback_key,
        &[
            ("actor", actor_name),
            ("room", room_title),
            ("item", item_label),
        ],
    )
}

fn render_distant_boss_cue(
    content: &ContentPack,
    definition: &PeriodicActorEffectDefinition,
    actor_name: &str,
    room_title: &str,
) -> Option<String> {
    let item_label = content.item_label(&definition.trigger.room_item);
    let key = format!("combat.{}_distant_boss", definition.id);
    content.render_message(
        &key,
        &[
            ("actor", actor_name),
            ("room", room_title),
            ("item", item_label),
        ],
    )
}

fn render_distant_cue(
    content: &ContentPack,
    definition: &PeriodicActorEffectDefinition,
    room_title: &str,
) -> Option<String> {
    let item_label = content.item_label(&definition.trigger.room_item);
    let default_key = format!("combat.{}_distant", definition.id);
    let key = definition
        .distant_message
        .as_deref()
        .unwrap_or(&default_key);
    content.render_message(key, &[("room", room_title), ("item", item_label)])
}

fn handle_trigger_depletion(
    state: &mut WorldState,
    content: &ContentPack,
    definition: &PeriodicActorEffectDefinition,
    room_id: &str,
    room_title: &str,
    exit_direction: Option<&str>,
    is_current_room: bool,
    is_adjacent: bool,
    lines: &mut NarrativeLines,
) {
    let Some(max_activations) = definition.trigger.max_activations else {
        return;
    };
    let key = format!("{room_id}::{}", definition.trigger.room_item);
    let remaining_charges = state
        .room_item_charges
        .get(&key)
        .copied()
        .unwrap_or(max_activations);
    if remaining_charges <= 1 {
        state.remove_items_from_room(room_id, &definition.trigger.room_item);
        if is_current_room {
            if let Some(deplete_key) = definition.trigger.deplete_message.as_deref() {
                let item_label = content.item_label(&definition.trigger.room_item);
                if let Some(line) = content.render_message(
                    deplete_key,
                    &[
                        ("item", item_label),
                        ("item_id", &definition.trigger.room_item),
                        ("room", room_title),
                    ],
                ) {
                    lines.narration(line);
                }
            }
        } else if is_adjacent {
            if let Some(line) =
                render_adjacent_depleted_cue(content, definition, exit_direction, room_title)
            {
                lines.narration(line);
            }
        } else if let Some(line) = render_distant_depleted_cue(content, definition, room_title) {
            lines.narration(line);
        }
    } else {
        state.room_item_charges.insert(key, remaining_charges - 1);
    }
}

fn render_adjacent_depleted_cue(
    content: &ContentPack,
    definition: &PeriodicActorEffectDefinition,
    exit_direction: Option<&str>,
    room_title: &str,
) -> Option<String> {
    let item_label = content.item_label(&definition.trigger.room_item);
    if let Some(direction_raw) = exit_direction {
        let direction = format_direction_phrase(direction_raw);
        let default_key = format!("combat.{}_adjacent_depleted", definition.id);
        let key = definition
            .trigger
            .deplete_adjacent_message
            .as_deref()
            .unwrap_or(&default_key);
        if let Some(line) = content.render_message(
            key,
            &[
                ("direction", direction.as_str()),
                ("room", room_title),
                ("item", item_label),
            ],
        ) {
            return Some(line);
        }
    }
    let fallback_key = format!("combat.{}_adjacent_depleted_no_exit", definition.id);
    content.render_message(&fallback_key, &[("room", room_title), ("item", item_label)])
}

fn render_distant_depleted_cue(
    content: &ContentPack,
    definition: &PeriodicActorEffectDefinition,
    room_title: &str,
) -> Option<String> {
    let item_label = content.item_label(&definition.trigger.room_item);
    let default_key = format!("combat.{}_distant_depleted", definition.id);
    let key = definition
        .trigger
        .deplete_distant_message
        .as_deref()
        .unwrap_or(&default_key);
    content.render_message(key, &[("room", room_title), ("item", item_label)])
}
