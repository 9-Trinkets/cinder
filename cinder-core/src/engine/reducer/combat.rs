use crate::content::types::{ContentPack, ItemStorageTarget};
use crate::engine::hook_ids;
use crate::engine::hooks::apply_narrating_world_hook_effects;
use crate::engine::narrative::NarrativeLines;
use crate::engine::state::{
    ActorRelationship, ActorStance, GamePhase, WorldState, display_actor_name,
};
use serde_json::json;

pub(super) fn apply_attack_target(
    state: &mut WorldState,
    content: &ContentPack,
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
    let player_attack =
        state.effective_actor_stat(content, &combat.player_actor_id, &combat.attack_stat_id);
    let target_defense =
        state.effective_actor_stat(content, target_actor_id, &combat.defense_stat_id);
    let base_damage = (player_attack - target_defense).max(combat.minimum_damage);
    let allied_participants = room_allies(state, content, room_id);
    let ally_damage = allied_participants
        .iter()
        .map(|actor_id| state.actor_stat(actor_id, &combat.attack_stat_id).max(0))
        .sum::<i32>();
    let total_damage = base_damage + ally_damage;
    let remaining = adjust_actor_stat(
        state,
        target_actor_id,
        &combat.health_stat_id,
        -total_damage,
    );
    let target_name = actor_display_name(content, target_actor_id);
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
    for ally_id in &allied_participants {
        let ally_damage = state.actor_stat(ally_id, &combat.attack_stat_id).max(0);
        let ally_name = actor_display_name(content, ally_id);
        if let Some(line) = content.render_message(
            "combat.ally_joins_attack",
            &[
                ("actor", ally_name.as_str()),
                ("damage", ally_damage.to_string().as_str()),
            ],
        ) {
            lines.narration(line);
        }
    }
    if remaining <= 0 {
        defeat_actor(state, content, target_actor_id, room_id, lines);
        return;
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

pub(super) fn actor_display_name(content: &ContentPack, actor_id: &str) -> String {
    content
        .actor(actor_id)
        .map(|actor| actor.name.clone())
        .unwrap_or_else(|| actor_id.to_string())
}

/// Runs the shared defeat sequence for an actor whose health reached zero.
pub(super) fn defeat_actor(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    room_id: &str,
    lines: &mut NarrativeLines,
) {
    let actor_name = actor_display_name(content, actor_id);
    if let Some(line) =
        content.render_message("combat.actor_defeated", &[("actor", actor_name.as_str())])
    {
        lines.narration(line);
    }
    spawn_defeat_drops(state, content, actor_id, room_id, lines);
    apply_narrating_world_hook_effects(
        state,
        content,
        hook_ids::ACTOR_DEFEATED,
        json!({
            "actor_id": actor_id,
            "actor_name": actor_name,
            "room_id": room_id,
        }),
        lines,
    )
    .unwrap_or_else(|error| eprintln!("[cinder] hook warning (actor.defeated): {error}"));
    award_defeat_xp(state, content, actor_id, lines);
    state.set_relationship(actor_id, ActorRelationship::default());
}

/// Awards the defeated actor's full XP independently to each party member and
/// applies the recipient's own level table.
pub(super) fn award_defeat_xp(
    state: &mut WorldState,
    content: &ContentPack,
    defeated_actor_id: &str,
    lines: &mut NarrativeLines,
) {
    let Some(xp) = content.actor(defeated_actor_id).map(|actor| actor.xp_drop) else {
        return;
    };
    let player_actor_id = &content.settings.combat.player_actor_id;
    if xp == 0 || content.level_table(player_actor_id).is_empty() {
        return;
    }
    let mut targets = vec![player_actor_id.clone()];
    targets.extend(
        state
            .relationships
            .iter()
            .filter(|(_, relationship)| relationship.follows_player)
            .map(|(actor_id, _)| actor_id.clone()),
    );
    let mut leveled: Vec<(String, u32)> = Vec::new();
    for target in targets {
        let mut accrued = state.actor_xp.get(&target).copied().unwrap_or(0) + xp;
        let mut level = state.actor_level.get(&target).copied().unwrap_or(1).max(1);
        let mut gained = 0;
        while let Some(definition) = content.level_definition(&target, level) {
            if accrued < definition.exp_required {
                break;
            }
            accrued -= definition.exp_required;
            level += 1;
            gained += 1;
            for (stat, delta) in &definition.stat_changes {
                if let Err(error) = state.adjust_actor_stat(&target, stat, *delta) {
                    eprintln!("[cinder] level stat error ({target}/{stat}): {error}");
                }
            }
        }
        state.actor_xp.insert(target.clone(), accrued);
        *state.actor_level.entry(target.clone()).or_insert(1) = level;
        if gained > 0 {
            leveled.push((target, level));
        }
    }
    for (actor_id, new_level) in leveled {
        let name = content
            .actor(&actor_id)
            .map(|actor| display_actor_name(state, actor))
            .unwrap_or_else(|| actor_id.clone());
        if let Some(line) = content.render_message(
            "combat.level_up",
            &[
                ("actor_name", name.as_str()),
                ("level", new_level.to_string().as_str()),
            ],
        ) {
            lines.narration(line);
        }
    }
}

pub(super) fn spawn_defeat_drops(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    room_id: &str,
    lines: &mut NarrativeLines,
) {
    let Some(actor) = content.actor(actor_id) else {
        return;
    };
    if actor.drops.is_empty() {
        return;
    }
    let mut dropped_labels = Vec::new();
    for (item_id, count) in &actor.drops {
        for _ in 0..*count {
            state.add_item_to_storage(item_id, ItemStorageTarget::CurrentRoom, room_id);
        }
        if let Some(item) = content.item(item_id) {
            dropped_labels.push(if *count > 1 {
                format!("{} x{count}", item.label)
            } else {
                item.label.clone()
            });
        }
    }
    if dropped_labels.is_empty() {
        return;
    }
    let actor_name = actor_display_name(content, actor_id);
    if let Some(line) = content.render_message(
        "combat.defeat_drop",
        &[
            ("actor", actor_name.as_str()),
            ("items", dropped_labels.join(", ").as_str()),
            ("room", room_id),
        ],
    ) {
        lines.narration(line);
    }
}

fn adjust_actor_stat(state: &mut WorldState, actor_id: &str, stat: &str, delta: i32) -> i32 {
    state
        .adjust_actor_stat(actor_id, stat, delta)
        .unwrap_or_else(|error| eprintln!("[cinder] combat stat error: {error}"));
    state.actor_stat(actor_id, stat)
}

pub(super) fn defeat_player_if_dead(
    state: &mut WorldState,
    content: &ContentPack,
    lines: &mut NarrativeLines,
) {
    if state.phase != GamePhase::Active {
        return;
    }
    let combat = &content.settings.combat;
    if state.effective_actor_stat(content, &combat.player_actor_id, &combat.health_stat_id) > 0 {
        return;
    }
    lines.narration(super::observation::render_story_text(
        &combat.player_defeat_text,
        state,
    ));
    state.phase = GamePhase::GameEnded;
}

fn room_allies(state: &WorldState, content: &ContentPack, room_id: &str) -> Vec<String> {
    state
        .relationships
        .iter()
        .filter(|(actor_id, relationship)| {
            !content.is_player_actor(actor_id) && relationship.stance == ActorStance::Allied
        })
        .map(|(actor_id, _)| actor_id.clone())
        .filter(|actor_id| {
            !state.actor_is_defeated(actor_id, &content.settings.combat.health_stat_id)
                && state.actor_room_id(
                    actor_id,
                    content
                        .actor(actor_id)
                        .map(|actor| actor.room_id.as_str())
                        .unwrap_or_default(),
                ) == room_id
        })
        .collect()
}
