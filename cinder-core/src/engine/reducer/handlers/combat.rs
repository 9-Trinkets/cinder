use crate::engine::reducer::command_effects::{actor_display_name, defeat_player_if_dead};
use crate::content::types::ContentPack;
use crate::engine::narrative::NarrativeLines;
use crate::engine::state::{ActorStance, GamePhase, WorldState};
fn living_guard_in_room(
    state: &WorldState,
    content: &ContentPack,
    room_id: &str,
) -> Option<String> {
    content
        .actors
        .iter()
        .filter(|actor| actor.guard && state.relationship(&actor.id).follows_player)
        .filter(|actor| {
            state.actor_room_id(&actor.id, &actor.room_id) == room_id
                && !state.actor_is_defeated(&actor.id, &content.settings.combat.health_stat_id)
        })
        .map(|actor| actor.id.clone())
        .next()
}

pub(crate) fn handle_hostile_strike(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    lines: &mut NarrativeLines,
) {
    let combat = &content.settings.combat;
    if state.phase != GamePhase::Active
        || state.stance(actor_id) != ActorStance::Hostile
        || state.actor_stat(actor_id, &combat.health_stat_id) <= 0
    {
        return;
    }
    if state.current_time_minutes < *state.next_hostile_strike_at.get(actor_id).unwrap_or(&0) {
        return;
    }
    let default_room_id = content
        .actor(actor_id)
        .map(|actor| actor.room_id.clone())
        .unwrap_or_default();
    if state.actor_room_id(actor_id, &default_room_id) != state.current_room_id {
        return;
    }
    let raw_damage = (state.actor_stat(actor_id, &combat.attack_stat_id)
        - state.effective_actor_stat(content, &combat.player_actor_id, &combat.defense_stat_id))
    .max(combat.minimum_damage);
    let actor_name = actor_display_name(content, actor_id);
    // A guarding follower intercepts the blow aimed at the player.
    if let Some(guard_id) = living_guard_in_room(state, content, state.current_room_id.as_str()) {
        let guard_defense = state
            .effective_actor_stat(content, &guard_id, &combat.defense_stat_id)
            .max(0);
        let guard_takes = (raw_damage - guard_defense).max(0);
        state
            .adjust_actor_stat(&guard_id, &combat.health_stat_id, -guard_takes)
            .unwrap_or_else(|error| eprintln!("[cinder] combat stat error: {error}"));
        let guard_name = actor_display_name(content, &guard_id);
        if let Some(line) = content.render_message(
            "combat.guard_intercepts",
            &[
                ("actor", actor_name.as_str()),
                ("guard", guard_name.as_str()),
                ("damage", guard_takes.to_string().as_str()),
            ],
        ) {
            lines.narration(line);
        }
        let remaining = state.effective_actor_stat(content, &guard_id, &combat.health_stat_id);
        if remaining <= 0
            && let Some(line) =
                content.render_message("combat.guard_falls", &[("guard", guard_name.as_str())])
        {
            lines.narration(line);
        }
    } else {
        let damage = raw_damage;
        state
            .adjust_actor_stat(&combat.player_actor_id, &combat.health_stat_id, -damage)
            .unwrap_or_else(|error| eprintln!("[cinder] combat stat error: {error}"));
        let remaining =
            state.effective_actor_stat(content, &combat.player_actor_id, &combat.health_stat_id);
        if let Some(line) = content.render_message(
            "combat.hostile_strike",
            &[
                ("actor", actor_name.as_str()),
                ("damage", damage.to_string().as_str()),
                ("remaining", remaining.to_string().as_str()),
            ],
        ) {
            lines.narration(line);
        }
    }
    let interval = content
        .actor(actor_id)
        .map(|actor| actor.attack_interval_minutes(combat.default_attack_interval_minutes))
        .unwrap_or(combat.default_attack_interval_minutes);
    state
        .next_hostile_strike_at
        .insert(actor_id.to_string(), state.current_time_minutes + interval);
    defeat_player_if_dead(state, content, lines);
}

pub(crate) fn handle_pair_stat_adjusted(
    state: &mut WorldState,
    participant_a_id: &str,
    participant_b_id: &str,
    stat: &str,
    delta: i32,
) {
    let _ = state.adjust_pair_stat(participant_a_id, participant_b_id, stat, delta);
}
