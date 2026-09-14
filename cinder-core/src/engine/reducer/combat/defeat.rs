use crate::content::types::{
    ContentPack, DropSpec, ItemStorageTarget, XpDistributionMode, XpRecipientMode,
};
use crate::engine::hook_ids;
use crate::engine::hooks::apply_narrating_world_hook_effects;
use crate::engine::narrative::NarrativeLines;
use crate::engine::reducer::observation::render_story_text;
use crate::engine::reducer::summaries::summarize_actor_names;
use crate::engine::state::{ActorRelationship, GamePhase, WorldState, display_actor_name};
use crate::engine::turn_policies::story_var_is_truthy;
use rand::Rng;
use rand::seq::SliceRandom;
use serde_json::json;
use std::collections::BTreeMap;

use super::actor_display_name;
use crate::engine::reducer::handlers::push_message;

/// Runs the shared defeat sequence for an actor whose health reached zero.
pub(in crate::engine::reducer) fn defeat_actor(
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
pub(in crate::engine::reducer) fn award_defeat_xp(
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
            // Apply growth while the new level (and thus the new natural max
            // for health) is already recorded, so the health clamp cannot eat
            // the level's own +hp growth.
            *state.actor_level.entry(target.clone()).or_insert(1) = level;
            for (stat, delta) in &definition.stat_changes {
                if let Err(error) = state.adjust_actor_stat(content, &target, stat, *delta) {
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
    let mut follower_levels = BTreeMap::<u32, Vec<String>>::new();
    for (actor_id, new_level) in leveled {
        let name = content
            .actor(&actor_id)
            .map(|actor| display_actor_name(state, actor))
            .unwrap_or_else(|| actor_id.clone());
        if actor_id == *player_actor_id {
            let level = new_level.to_string();
            push_message(
                lines,
                content,
                "combat.level_up",
                &[("actor_name", name.as_str()), ("level", level.as_str())],
            );
        } else {
            follower_levels.entry(new_level).or_default().push(name);
        }
    }
    for (new_level, actor_names) in follower_levels {
        let count = actor_names.len();
        let key = if count >= 4 && content.messages.contains_key("combat.party_level_up_large") {
            Some("combat.party_level_up_large")
        } else if count > 1 && content.messages.contains_key("combat.party_level_up_group") {
            Some("combat.party_level_up_group")
        } else {
            None
        };
        if let Some(key) = key {
            let actors = summarize_actor_names(&actor_names).unwrap_or_default();
            let count = count.to_string();
            let level = new_level.to_string();
            push_message(
                lines,
                content,
                key,
                &[
                    ("actors", actors.as_str()),
                    ("count", count.as_str()),
                    ("level", level.as_str()),
                ],
            );
        } else {
            for actor_name in actor_names {
                let level = new_level.to_string();
                push_message(
                    lines,
                    content,
                    "combat.level_up",
                    &[
                        ("actor_name", actor_name.as_str()),
                        ("level", level.as_str()),
                    ],
                );
            }
        }
    }
}

pub(in crate::engine::reducer) fn spawn_defeat_drops(
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
    let mut rng = rand::thread_rng();
    // Item id → count resolved across specs, so a weighted pool can never
    // scatter the same suit twice and mixed specs collapse instead of stacking.
    let mut resolved: Vec<(String, u32)> = Vec::new();
    for (key, spec) in &actor.drops {
        match spec {
            DropSpec::Always(count) if *count > 0 => {
                resolved.push((key.clone(), *count));
            }
            DropSpec::Conditional(conditional) => {
                let skipped = !conditional.skip_when_story_var.is_empty()
                    && story_var_is_truthy(state, &conditional.skip_when_story_var);
                if !skipped && conditional.count > 0 {
                    resolved.push((key.clone(), conditional.count));
                }
            }
            DropSpec::Chance(chance) => {
                let roll = rng.gen_range(0..100);
                if roll < chance.chance_percent && chance.count > 0 {
                    resolved.push((key.clone(), chance.count));
                }
            }
            DropSpec::Weighted(pool) => {
                if pool.rolls == 0 || pool.entries.is_empty() {
                    continue;
                }
                for _ in 0..pool.rolls {
                    let Ok(entry) = pool.entries.choose_weighted(&mut rng, |entry| entry.weight)
                    else {
                        continue;
                    };
                    if entry.count == 0 {
                        continue;
                    }
                    if let Some((_, existing)) = resolved
                        .iter_mut()
                        .find(|(item_id, _)| item_id == &entry.item_id)
                    {
                        *existing += entry.count;
                    } else {
                        resolved.push((entry.item_id.clone(), entry.count));
                    }
                }
            }
            _ => {}
        }
    }
    if resolved.is_empty() {
        return;
    }
    let mut dropped_labels = Vec::new();
    for (item_id, count) in &resolved {
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

pub(in crate::engine::reducer) fn defeat_player_if_dead(
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
    lines.narration(render_story_text(&combat.player_defeat_text, state));
    state.phase = GamePhase::GameEnded;
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
