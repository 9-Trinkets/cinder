use crate::content::types::{
    AllyAttackMode, AllyAttackParticipants, ContentPack, DropSpec, ItemStorageTarget,
    PeriodicActorEffect, XpDistributionMode, XpRecipientMode,
};
use crate::engine::hook_ids;
use crate::engine::hooks::{
    apply_narrating_world_hook_effects, apply_world_hook_effects,
};
use crate::engine::narrative::NarrativeLines;
use crate::engine::state::{
    ActorRelationship, ActorStance, GamePhase, WorldState, display_actor_name,
};
use crate::engine::turn_policies::story_var_is_truthy;
use serde_json::json;

static VEC_EMPTY_TAGS: Vec<String> = Vec::new();

pub(super) fn handle_periodic_actor_effect_applied(
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
    let default_room_id = content
        .actor(actor_id)
        .map(|actor| actor.room_id.as_str())
        .unwrap_or_default();
    let room_id = state.actor_room_id(actor_id, default_room_id).to_string();
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
    // A trigger with fixed activations burns one charge per actual application
    // and, when the last charge goes, spends the room item itself. The planner
    // keeps scheduling events while the item is present; once it fades the
    // `has_item_in_storage` guard above stops further applications in the same
    // batch.
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
                let item_label = content
                    .item(&definition.trigger.room_item)
                    .map(|item| item.label.as_str())
                    .unwrap_or(&definition.trigger.room_item);
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

pub(super) fn apply_attack_target(
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
        for ally_id in &allied_participants {
            let ally_damage = ally_attack_contribution(state, content, ally_id);
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

/// Applies a target actor's content-declared resistance to `damage` dealt in
/// `kind`. Each point of resistance removes that much damage; the result is
/// floored at zero so a fully-resisted hit yields true immunity (no minimum
/// damage floor survives resistance). Negative resistance amplifies damage.
pub(super) fn resisted_damage(content: &ContentPack, target_actor_id: &str, kind: &str, damage: i32) -> i32 {
    let resistance = content
        .actor(target_actor_id)
        .map(|actor| actor.resistances.get(kind).copied().unwrap_or(0))
        .unwrap_or(0);
    (damage - resistance).max(0)
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
    let targets = xp_recipients(state, content);
    let awards = distribute_xp(
        xp,
        targets.len(),
        content.settings.combat.xp_distribution.mode,
    );
    let mut leveled: Vec<(String, u32)> = Vec::new();
    for (target, award) in targets.into_iter().zip(awards) {
        if award == 0 {
            continue;
        }
        let mut accrued = state
            .actor_xp
            .get(&target)
            .copied()
            .unwrap_or(0)
            .saturating_add(award);
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
    for (item_id, spec) in &actor.drops {
        let count = match spec {
            DropSpec::Always(count) => *count,
            DropSpec::Conditional(conditional) => {
                if !conditional.skip_when_story_var.is_empty()
                    && story_var_is_truthy(state, &conditional.skip_when_story_var)
                {
                    0
                } else {
                    conditional.count
                }
            }
        };
        if count == 0 {
            continue;
        }
        for _ in 0..count {
            state.add_item_to_storage(item_id, ItemStorageTarget::CurrentRoom, room_id);
        }
        if let Some(item) = content.item(item_id) {
            dropped_labels.push(if count > 1 {
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
                && state.actor_room_id(
                    actor_id,
                    content
                        .actor(actor_id)
                        .map(|actor| actor.room_id.as_str())
                        .unwrap_or_default(),
                ) == room_id
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

fn xp_recipients(state: &WorldState, content: &ContentPack) -> Vec<String> {
    let player_actor_id = &content.settings.combat.player_actor_id;
    let mut recipients = vec![player_actor_id.clone()];
    if content.settings.combat.xp_distribution.recipients == XpRecipientMode::PlayerAndFollowers {
        let mut followers = state
            .relationships
            .iter()
            .filter(|(actor_id, relationship)| {
                actor_id.as_str() != player_actor_id && relationship.follows_player
            })
            .map(|(actor_id, _)| actor_id.clone())
            .collect::<Vec<_>>();
        followers.sort();
        recipients.extend(followers);
    }
    recipients
}

fn distribute_xp(xp: u32, recipient_count: usize, mode: XpDistributionMode) -> Vec<u32> {
    if recipient_count == 0 {
        return Vec::new();
    }
    match mode {
        XpDistributionMode::FullEach => vec![xp; recipient_count],
        XpDistributionMode::SplitEvenly => {
            let count = recipient_count as u32;
            let base = xp / count;
            let remainder = xp % count;
            (0..recipient_count)
                .map(|index| base + u32::from((index as u32) < remainder))
                .collect()
        }
    }
}
