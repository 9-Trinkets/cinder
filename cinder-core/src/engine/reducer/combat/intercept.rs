use crate::content::types::ContentPack;
use crate::engine::narrative::NarrativeLines;
use crate::engine::party_policy::{consume_party_reaction, select_defensive_reaction};
use crate::engine::state::WorldState;

use super::{actor_display_name, adjust_actor_stat, defeat_actor, resisted_damage};

/// Resolves a guarding party member stepping in front of a blow aimed at its
/// leader. Returns `true` when a guard absorbed the strike for `target_actor_id`
/// (the caller must then skip damaging the target directly).
///
/// Shared by hostile-initiated strikes (where the player party's guards shield
/// the player) and player-initiated attacks (where an AI party's guards shield
/// their leader, e.g. the elf army protecting its king).
pub fn apply_leader_intercept(
    state: &mut WorldState,
    content: &ContentPack,
    attacker_actor_id: &str,
    incoming_attack: i32,
    attack_kind: &str,
    target_actor_id: &str,
    room_id: &str,
    lines: &mut NarrativeLines,
) -> bool {
    let combat = &content.settings.combat;
    let Some(decision) = select_defensive_reaction(content, state, target_actor_id) else {
        return false;
    };
    let guard_id = decision.actor_id.clone();
    let guard_intercept_level = state.actor_skill_level(&guard_id, "intercept");
    let soaked = content
        .skill("intercept")
        .and_then(|skill| skill.param_value("damage_soaked", guard_intercept_level))
        .unwrap_or_else(|| {
            crate::engine::skills::intercept_damage_soaked(guard_intercept_level) as i64
        }) as i32;
    let guard_defense = state.actor_combat_defense(content, &guard_id).max(0);
    let raw_guard_takes = (incoming_attack - guard_defense - soaked).max(combat.minimum_damage);
    let guard_takes = resisted_damage(content, &guard_id, attack_kind, raw_guard_takes);
    let attacker_name = actor_display_name(state, content, attacker_actor_id);
    let guard_name = actor_display_name(state, content, &guard_id);
    crate::engine::reducer::award_actor_skill_sp(state, content, &guard_id, "intercept", 3, lines);
    if raw_guard_takes > 0 && guard_takes == 0 {
        if let Some(line) = content.render_message(
            "combat.no_effect",
            &[("actor", guard_name.as_str()), ("kind", attack_kind)],
        ) {
            lines.narration(line);
        }
    } else {
        adjust_actor_stat(
            state,
            content,
            &guard_id,
            &combat.health_stat_id,
            -guard_takes,
        );
        if room_id == state.current_room_id {
            let message = if decision.message.is_empty() {
                "combat.guard_intercepts"
            } else {
                decision.message.as_str()
            };
            if let Some(line) = content.render_message(
                message,
                &[
                    ("actor", attacker_name.as_str()),
                    ("guard", guard_name.as_str()),
                    ("damage", guard_takes.to_string().as_str()),
                ],
            ) {
                lines.narration(line);
            }
        }
        let remaining = state.effective_actor_stat(content, &guard_id, &combat.health_stat_id);
        if remaining <= 0 && guard_takes > 0 {
            if room_id == state.current_room_id {
                if let Some(line) =
                    content.render_message("combat.guard_falls", &[("guard", guard_name.as_str())])
                {
                    lines.narration(line);
                }
            } else {
                let offscreen_state = state
                    .offscreen_combat_states
                    .entry(room_id.to_string())
                    .or_default();
                if !offscreen_state.reported_fallen_allies.contains(&guard_id) {
                    offscreen_state
                        .reported_fallen_allies
                        .insert(guard_id.clone());
                    offscreen_state.newly_fallen_allies.push(guard_name.clone());
                }
            }
            defeat_actor(state, content, &guard_id, room_id, lines);
        }
    }
    consume_party_reaction(content, state, &decision);
    true
}
