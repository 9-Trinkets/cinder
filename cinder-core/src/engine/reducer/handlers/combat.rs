use crate::content::types::ContentPack;
use crate::engine::narrative::NarrativeLines;
use crate::engine::party_policy::{consume_party_reaction, select_defensive_reaction};
use crate::engine::reducer::combat::resisted_damage;
use crate::engine::reducer::command_effects::{actor_display_name, defeat_player_if_dead};
use crate::engine::state::{ActorStance, GamePhase, WorldState};
fn legacy_guard_in_room(
    state: &WorldState,
    content: &ContentPack,
    room_id: &str,
) -> Option<String> {
    content
        .actors
        .iter()
        .filter(|actor| actor.guard && state.relationship(&actor.id).follows_player)
        .filter(|actor| {
            state.actor_is_in_room(content, &actor.id, room_id)
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
    let attack_kind = content
        .actor(actor_id)
        .map(|actor| actor.attack_kind())
        .unwrap_or("physical");
    let actor_name = actor_display_name(content, actor_id);
    let player_name = actor_display_name(content, &combat.player_actor_id);
    // A guarding follower intercepts the blow aimed at the player. The guard
    // soaks the attacker's raw strike against its own defense, floored by the
    // minimum-damage rule just like a direct hit on the player.
    let defensive_reaction = select_defensive_reaction(content, state);
    let guard_id = defensive_reaction
        .as_ref()
        .map(|decision| decision.actor_id.clone())
        .or_else(|| {
            content
                .settings
                .party
                .combat_rules
                .is_empty()
                .then(|| legacy_guard_in_room(state, content, state.current_room_id.as_str()))
                .flatten()
        });
    if let Some(guard_id) = guard_id {
        let guard_defense = state
            .effective_actor_stat(content, &guard_id, &combat.defense_stat_id)
            .max(0);
        let raw_guard_takes = (state.actor_stat(actor_id, &combat.attack_stat_id) - guard_defense)
            .max(combat.minimum_damage);
        let guard_takes = resisted_damage(content, &guard_id, attack_kind, raw_guard_takes);
        let guard_name = actor_display_name(content, &guard_id);
        if raw_guard_takes > 0 && guard_takes == 0 {
            if let Some(line) = content.render_message(
                "combat.no_effect",
                &[("actor", guard_name.as_str()), ("kind", attack_kind)],
            ) {
                lines.narration(line);
            }
        } else {
            state
                .adjust_actor_stat(&guard_id, &combat.health_stat_id, -guard_takes)
                .unwrap_or_else(|error| eprintln!("[cinder] combat stat error: {error}"));
            let message = defensive_reaction
                .as_ref()
                .map(|decision| decision.message.as_str())
                .filter(|message| !message.is_empty())
                .unwrap_or("combat.guard_intercepts");
            if let Some(line) = content.render_message(
                message,
                &[
                    ("actor", actor_name.as_str()),
                    ("guard", guard_name.as_str()),
                    ("damage", guard_takes.to_string().as_str()),
                ],
            ) {
                lines.narration(line);
            }
        }
        let remaining = state.effective_actor_stat(content, &guard_id, &combat.health_stat_id);
        if remaining <= 0
            && guard_takes > 0
            && let Some(line) =
                content.render_message("combat.guard_falls", &[("guard", guard_name.as_str())])
        {
            lines.narration(line);
        }
        if let Some(decision) = defensive_reaction.as_ref() {
            consume_party_reaction(content, state, decision);
        }
    } else {
        let damage = resisted_damage(content, &combat.player_actor_id, attack_kind, raw_damage);
        if raw_damage > 0 && damage == 0 {
            if let Some(line) = content.render_message(
                "combat.no_effect",
                &[("actor", player_name.as_str()), ("kind", attack_kind)],
            ) {
                lines.narration(line);
            }
        } else {
            state
                .adjust_actor_stat(&combat.player_actor_id, &combat.health_stat_id, -damage)
                .unwrap_or_else(|error| eprintln!("[cinder] combat stat error: {error}"));
            let remaining = state.effective_actor_stat(
                content,
                &combat.player_actor_id,
                &combat.health_stat_id,
            );
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
