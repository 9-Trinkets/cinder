use crate::content::types::{AllyAttackMode, AllyAttackParticipants, ContentPack};
use crate::engine::hook_ids;
use crate::engine::hooks::apply_world_hook_effects;
use crate::engine::narrative::NarrativeLines;
use crate::engine::reducer::handlers::push_message;
use crate::engine::reducer::summaries::summarize_actor_names;
use crate::engine::state::{ActorStance, WorldState};
use serde_json::json;

use super::{VEC_EMPTY_TAGS, actor_display_name, adjust_actor_stat, defeat_actor, resisted_damage};

pub(in crate::engine::reducer) fn apply_attack_target(
    state: &mut WorldState,
    content: &ContentPack,
    attacker_actor_id: &str,
    target_actor_id: &str,
    room_id: &str,
    lines: &mut NarrativeLines,
) {
    let combat = &content.settings.combat;
    // Mechanism backstop: allies are never valid targets, even if a pack
    // omits the planning-side message.
    if state.stance(target_actor_id) == ActorStance::Allied {
        return;
    }
    let target_name = actor_display_name(content, target_actor_id);
    // `actor.attacked` fires for this pack only when the player themselves
    // issues the attack action (follower damage boosts and periodic effects
    // never reach this point). Packs use it to record run-rule flags, e.g. a
    // clean-floor reward that is forfeited by attacking ordinary mobs.
    if attacker_actor_id == content.settings.combat.player_actor_id {
        let target_tags = content
            .actor(target_actor_id)
            .map(|actor| &actor.tags)
            .unwrap_or(&VEC_EMPTY_TAGS);
        apply_world_hook_effects(
            state,
            content,
            hook_ids::ACTOR_ATTACKED,
            json!({
                "actor_id": target_actor_id,
                "actor_name": target_name,
                "attacker_id": attacker_actor_id,
                "tags": target_tags,
            }),
        )
        .unwrap_or_else(|error| eprintln!("[cinder] hook warning (actor.attacked): {error}"));
    }
    let player_attack =
        state.effective_actor_stat(content, &combat.player_actor_id, &combat.attack_stat_id);
    let target_defense =
        state.effective_actor_stat(content, target_actor_id, &combat.defense_stat_id);
    let base_damage = (player_attack - target_defense).max(combat.minimum_damage);
    let allied_participants = participating_allies(state, content, room_id);
    let ally_damage = allied_participants
        .iter()
        .map(|actor_id| ally_attack_contribution(state, content, actor_id))
        .fold(0, i32::saturating_add);
    let raw_damage = base_damage + ally_damage;
    let attack_kind = content
        .actor(&combat.player_actor_id)
        .map(|actor| actor.attack_kind())
        .unwrap_or("physical");
    let total_damage = resisted_damage(content, target_actor_id, attack_kind, raw_damage);
    if raw_damage > 0 && total_damage == 0 {
        if let Some(line) = content.render_message(
            "combat.no_effect",
            &[("actor", target_name.as_str()), ("kind", attack_kind)],
        ) {
            lines.narration(line);
        }
    } else {
        let remaining = adjust_actor_stat(
            state,
            content,
            target_actor_id,
            &combat.health_stat_id,
            -total_damage,
        );
        if let Some(line) = content.render_message(
            "combat.attack_hit",
            &[
                ("actor", target_name.as_str()),
                ("damage", total_damage.to_string().as_str()),
                ("remaining", remaining.to_string().as_str()),
            ],
        ) {
            lines.narration(line);
        }
        render_ally_attack_contribution(state, content, &allied_participants, ally_damage, lines);
        if remaining <= 0 {
            defeat_actor(state, content, target_actor_id, room_id, lines);
            return;
        }

        fn render_ally_attack_contribution(
            state: &WorldState,
            content: &ContentPack,
            allied_participants: &[String],
            total_damage: i32,
            lines: &mut NarrativeLines,
        ) {
            let count = allied_participants.len();
            if count == 0 {
                return;
            }
            let actor_names = allied_participants
                .iter()
                .map(|actor_id| actor_display_name(content, actor_id))
                .collect::<Vec<_>>();
            let group_key =
                if count >= 4 && content.messages.contains_key("combat.party_joins_attack") {
                    Some("combat.party_joins_attack")
                } else if count > 1 && content.messages.contains_key("combat.allies_join_attack") {
                    Some("combat.allies_join_attack")
                } else {
                    None
                };
            if let Some(key) = group_key
                && let Some(actors) = summarize_actor_names(&actor_names)
            {
                let count = count.to_string();
                let damage = total_damage.to_string();
                push_message(
                    lines,
                    content,
                    key,
                    &[
                        ("actors", actors.as_str()),
                        ("count", count.as_str()),
                        ("damage", damage.as_str()),
                    ],
                );
                return;
            }
            for (ally_id, actor_name) in allied_participants.iter().zip(actor_names) {
                let damage = ally_attack_contribution(state, content, ally_id).to_string();
                push_message(
                    lines,
                    content,
                    "combat.ally_joins_attack",
                    &[("actor", actor_name.as_str()), ("damage", damage.as_str())],
                );
            }
        }
    }
    let mut relationship = state.relationship(target_actor_id);
    if relationship.stance != ActorStance::Allied && relationship.stance != ActorStance::Hostile {
        relationship.stance = ActorStance::Hostile;
        state.set_relationship(target_actor_id, relationship);
        let interval = content
            .actor(target_actor_id)
            .map(|actor| actor.attack_interval_minutes(combat.default_attack_interval_minutes))
            .unwrap_or(combat.default_attack_interval_minutes);
        state.next_hostile_strike_at.insert(
            target_actor_id.to_string(),
            state.current_time_minutes + interval,
        );
        if let Some(line) = content.render_message(
            "combat.actor_wakes_hostile",
            &[("actor", target_name.as_str())],
        ) {
            lines.narration(line);
        }
    }
}

fn participating_allies(state: &WorldState, content: &ContentPack, room_id: &str) -> Vec<String> {
    let policy = &content.settings.combat.ally_attack;
    if policy.participants == AllyAttackParticipants::Disabled {
        return Vec::new();
    }
    let mut allies = state
        .relationships
        .iter()
        .filter(|(actor_id, relationship)| {
            !content.is_player_actor(actor_id)
                && relationship.stance == ActorStance::Allied
                && (policy.participants != AllyAttackParticipants::FollowersOnly
                    || relationship.follows_player)
        })
        .map(|(actor_id, _)| actor_id.clone())
        .filter(|actor_id| {
            !state.actor_is_defeated(actor_id, &content.settings.combat.health_stat_id)
                && state.actor_is_in_room(content, actor_id, room_id)
        })
        .collect::<Vec<_>>();
    allies.sort();
    allies
}

fn ally_attack_contribution(state: &WorldState, content: &ContentPack, actor_id: &str) -> i32 {
    let policy = &content.settings.combat.ally_attack;
    let raw = match policy.mode {
        AllyAttackMode::AttackStat => state
            .actor_stat(actor_id, &content.settings.combat.attack_stat_id)
            .max(0),
    };
    let scaled = (i64::from(raw) * i64::from(policy.contribution_percent) / 100)
        .min(i64::from(i32::MAX)) as i32;
    policy
        .maximum_per_ally
        .map_or(scaled, |maximum| scaled.min(maximum))
}
