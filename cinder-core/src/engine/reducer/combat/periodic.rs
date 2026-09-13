use crate::content::types::{ContentPack, ItemStorageTarget, PeriodicActorEffect};
use crate::engine::narrative::NarrativeLines;
use crate::engine::state::{GamePhase, WorldState};

use super::{actor_display_name, defeat_actor, defeat_player_if_dead};

pub(in crate::engine::reducer) fn handle_periodic_actor_effect_applied(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    effect_id: &str,
    lines: &mut NarrativeLines,
) {
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
    if let Err(error) = state.adjust_actor_stat(actor_id, health_stat_id, -*amount) {
        eprintln!("[cinder] periodic effect stat error ({effect_id}, {actor_id}): {error}");
        return;
    }
    let remaining = state.actor_stat(actor_id, health_stat_id);
    let actor_name = actor_display_name(content, actor_id);
    if let Some(line) = content.render_message(
        &definition.message,
        &[
            ("actor", actor_name.as_str()),
            ("damage", amount.to_string().as_str()),
            ("remaining", remaining.to_string().as_str()),
            ("effect_id", effect_id),
        ],
    ) {
        lines.narration(line);
    }
    if remaining <= 0 {
        if actor_id == content.settings.combat.player_actor_id {
            defeat_player_if_dead(state, content, lines);
        } else {
            defeat_actor(state, content, actor_id, &room_id, lines);
        }
    }
    if let Some(max_activations) = definition.trigger.max_activations {
        let key = format!("{room_id}::{}", definition.trigger.room_item);
        let remaining_charges = state
            .room_item_charges
            .get(&key)
            .copied()
            .unwrap_or(max_activations);
        if remaining_charges <= 1 {
            state.remove_items_from_room(&room_id, &definition.trigger.room_item);
            if let Some(deplete_key) = definition.trigger.deplete_message.as_deref() {
                let item_label = content.item_label(&definition.trigger.room_item);
                if let Some(line) = content.render_message(
                    deplete_key,
                    &[("item", item_label), ("item_id", &definition.trigger.room_item)],
                ) {
                    lines.narration(line);
                }
            }
        } else {
            state.room_item_charges.insert(key, remaining_charges - 1);
        }
    }
}
