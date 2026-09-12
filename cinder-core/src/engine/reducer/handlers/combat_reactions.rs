use crate::content::types::{ContentPack, PartyReactionAction, PartySupportEffect};
use crate::engine::narrative::NarrativeLines;
use crate::engine::party_policy::{
    PartyReactionDecision, consume_party_reaction, resolve_party_reaction_target,
    select_post_damage_reactions,
};
use crate::engine::reducer::combat::{defeat_actor, resisted_damage};
use crate::engine::reducer::command_effects::actor_display_name;
use crate::engine::reducer::summaries::summarize_actor_names;
use crate::engine::state::{ActorStance, WorldState};

use super::feedback::push_message;

enum PartyReactionOutcome {
    Counterattack {
        actor: String,
        target_id: String,
        target: String,
        damage: i32,
        remaining: i32,
        kind: String,
        message: String,
        no_effect: bool,
    },
    Support {
        actor: String,
        target_id: String,
        target: String,
        amount: i32,
        remaining: i32,
        stat: String,
        message: String,
    },
    Hold {
        actor: String,
        message: String,
    },
}

impl PartyReactionOutcome {
    fn is_compatible_with(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Counterattack {
                    target_id,
                    message,
                    no_effect,
                    kind,
                    ..
                },
                Self::Counterattack {
                    target_id: other_target,
                    message: other_message,
                    no_effect: other_no_effect,
                    kind: other_kind,
                    ..
                },
            ) => {
                target_id == other_target
                    && message == other_message
                    && no_effect == other_no_effect
                    && (!no_effect || kind == other_kind)
            }
            (
                Self::Support {
                    target_id,
                    stat,
                    message,
                    ..
                },
                Self::Support {
                    target_id: other_target,
                    stat: other_stat,
                    message: other_message,
                    ..
                },
            ) => target_id == other_target && stat == other_stat && message == other_message,
            (
                Self::Hold { message, .. },
                Self::Hold {
                    message: other_message,
                    ..
                },
            ) => message == other_message,
            _ => false,
        }
    }

    fn actor(&self) -> &str {
        match self {
            Self::Counterattack { actor, .. }
            | Self::Support { actor, .. }
            | Self::Hold { actor, .. } => actor,
        }
    }
}

pub(super) fn resolve_post_damage_reactions(
    state: &mut WorldState,
    content: &ContentPack,
    attacker_id: &str,
    lines: &mut NarrativeLines,
) {
    let mut outcomes = Vec::new();
    for decision in select_post_damage_reactions(content, state) {
        if state.actor_is_defeated(attacker_id, &content.settings.combat.health_stat_id) {
            break;
        }
        let outcome = match decision.action {
            PartyReactionAction::Counterattack => {
                resolve_counterattack(state, content, attacker_id, &decision)
            }
            PartyReactionAction::Support => resolve_support(state, content, attacker_id, &decision),
            PartyReactionAction::Hold => Some(PartyReactionOutcome::Hold {
                actor: actor_display_name(content, &decision.actor_id),
                message: decision.message.clone(),
            }),
            PartyReactionAction::Intercept => None,
        };
        if let Some(outcome) = outcome {
            consume_party_reaction(content, state, &decision);
            let defeated_target = match &outcome {
                PartyReactionOutcome::Counterattack {
                    target_id,
                    remaining,
                    no_effect,
                    ..
                } if !no_effect && *remaining <= 0 => Some(target_id.clone()),
                _ => None,
            };
            outcomes.push(outcome);
            if let Some(target_id) = defeated_target {
                render(content, &outcomes, lines);
                outcomes.clear();
                let room_id = state.current_room_id.clone();
                defeat_actor(state, content, &target_id, &room_id, lines);
            }
        }
    }
    render(content, &outcomes, lines);
}

fn resolve_counterattack(
    state: &mut WorldState,
    content: &ContentPack,
    attacker_id: &str,
    decision: &PartyReactionDecision,
) -> Option<PartyReactionOutcome> {
    let target_id = resolve_party_reaction_target(content, state, decision, attacker_id)?;
    if state.actor_is_defeated(&target_id, &content.settings.combat.health_stat_id) {
        return None;
    }
    let combat = &content.settings.combat;
    let raw_damage =
        (state.effective_actor_stat(content, &decision.actor_id, &combat.attack_stat_id)
            - state.effective_actor_stat(content, &target_id, &combat.defense_stat_id))
        .max(combat.minimum_damage);
    let attack_kind = content
        .actor(&decision.actor_id)
        .map(|actor| actor.attack_kind())
        .unwrap_or("physical");
    let damage = resisted_damage(content, &target_id, attack_kind, raw_damage);
    let actor_name = actor_display_name(content, &decision.actor_id);
    let target_name = actor_display_name(content, &target_id);
    if raw_damage > 0 && damage == 0 {
        let remaining = state.actor_stat(&target_id, &combat.health_stat_id);
        Some(PartyReactionOutcome::Counterattack {
            actor: actor_name,
            target_id,
            target: target_name,
            damage,
            remaining,
            kind: attack_kind.to_string(),
            message: decision.message.clone(),
            no_effect: true,
        })
    } else {
        state
            .adjust_actor_stat(&target_id, &combat.health_stat_id, -damage)
            .unwrap_or_else(|error| eprintln!("[cinder] combat stat error: {error}"));
        let remaining = state.effective_actor_stat(content, &target_id, &combat.health_stat_id);
        Some(PartyReactionOutcome::Counterattack {
            actor: actor_name,
            target_id,
            target: target_name,
            damage,
            remaining,
            kind: attack_kind.to_string(),
            message: decision.message.clone(),
            no_effect: false,
        })
    }
}

fn resolve_support(
    state: &mut WorldState,
    content: &ContentPack,
    attacker_id: &str,
    decision: &PartyReactionDecision,
) -> Option<PartyReactionOutcome> {
    let target_id = resolve_party_reaction_target(content, state, decision, attacker_id)?;
    let player_id = content.settings.combat.player_actor_id.as_str();
    if (target_id != player_id && state.stance(&target_id) != ActorStance::Allied)
        || (target_id != player_id
            && !state.actor_is_in_room(content, &target_id, &state.current_room_id))
        || state.actor_is_defeated(&target_id, &content.settings.combat.health_stat_id)
    {
        return None;
    }
    let Some(PartySupportEffect::AdjustActorStat { stat, delta }) =
        decision.support_effect.as_ref()
    else {
        return None;
    };
    let before = state.actor_stat(&target_id, stat);
    state
        .adjust_actor_stat(&target_id, stat, *delta)
        .unwrap_or_else(|error| eprintln!("[cinder] party support stat error: {error}"));
    let remaining = state.actor_stat(&target_id, stat);
    let target_name = actor_display_name(content, &target_id);
    Some(PartyReactionOutcome::Support {
        actor: actor_display_name(content, &decision.actor_id),
        target_id,
        target: target_name,
        amount: remaining.saturating_sub(before).abs(),
        remaining,
        stat: stat.clone(),
        message: decision.message.clone(),
    })
}

fn render(content: &ContentPack, outcomes: &[PartyReactionOutcome], lines: &mut NarrativeLines) {
    let mut start = 0;
    while start < outcomes.len() {
        let mut end = start + 1;
        while end < outcomes.len() && outcomes[start].is_compatible_with(&outcomes[end]) {
            end += 1;
        }
        render_group(content, &outcomes[start..end], lines);
        start = end;
    }
}

fn render_group(content: &ContentPack, group: &[PartyReactionOutcome], lines: &mut NarrativeLines) {
    match &group[0] {
        PartyReactionOutcome::Counterattack {
            target,
            kind,
            message,
            no_effect,
            ..
        } => {
            if message.is_empty() && !no_effect {
                return;
            }
            let keys = if *no_effect {
                (
                    "combat.party_counterattack_no_effect_group",
                    "combat.party_counterattack_no_effect_large",
                )
            } else {
                (
                    "combat.party_counterattack_group",
                    "combat.party_counterattack_large",
                )
            };
            if let Some(key) = group_key(content, group.len(), keys) {
                let damage = group
                    .iter()
                    .map(|outcome| match outcome {
                        PartyReactionOutcome::Counterattack { damage, .. } => *damage,
                        _ => unreachable!(),
                    })
                    .sum::<i32>()
                    .to_string();
                render_aggregate(
                    content,
                    group,
                    lines,
                    key,
                    &[
                        ("target", target),
                        ("damage", damage.as_str()),
                        ("remaining", counter_remaining(group).as_str()),
                        ("kind", kind),
                    ],
                );
            } else {
                for outcome in group {
                    render_counterattack(content, outcome, lines);
                }
            }
        }
        PartyReactionOutcome::Support {
            target,
            stat,
            message,
            ..
        } => {
            if message.is_empty() {
                return;
            }
            if let Some(key) = group_key(
                content,
                group.len(),
                ("combat.party_support_group", "combat.party_support_large"),
            ) {
                let amount = group
                    .iter()
                    .map(|outcome| match outcome {
                        PartyReactionOutcome::Support { amount, .. } => *amount,
                        _ => unreachable!(),
                    })
                    .sum::<i32>()
                    .to_string();
                render_aggregate(
                    content,
                    group,
                    lines,
                    key,
                    &[
                        ("target", target),
                        ("amount", amount.as_str()),
                        ("remaining", support_remaining(group).as_str()),
                        ("stat", stat),
                    ],
                );
            } else {
                for outcome in group {
                    render_support(content, outcome, lines);
                }
            }
        }
        PartyReactionOutcome::Hold { message, .. } => {
            if message.is_empty() {
                return;
            }
            if let Some(key) = group_key(
                content,
                group.len(),
                ("combat.party_holds_group", "combat.party_holds_large"),
            ) {
                render_aggregate(content, group, lines, key, &[]);
            } else {
                for outcome in group {
                    push_message(lines, content, message, &[("actor", outcome.actor())]);
                }
            }
        }
    }
}

fn render_aggregate(
    content: &ContentPack,
    group: &[PartyReactionOutcome],
    lines: &mut NarrativeLines,
    key: &str,
    values: &[(&str, &str)],
) {
    let actor_names = group
        .iter()
        .map(|outcome| outcome.actor().to_string())
        .collect::<Vec<_>>();
    let actors = summarize_actor_names(&actor_names).unwrap_or_default();
    let count = group.len().to_string();
    let mut values = values.to_vec();
    values.extend([("actors", actors.as_str()), ("count", count.as_str())]);
    push_message(lines, content, key, &values);
}

fn counter_remaining(group: &[PartyReactionOutcome]) -> String {
    match group.last().unwrap() {
        PartyReactionOutcome::Counterattack { remaining, .. } => remaining.to_string(),
        _ => unreachable!(),
    }
}

fn support_remaining(group: &[PartyReactionOutcome]) -> String {
    match group.last().unwrap() {
        PartyReactionOutcome::Support { remaining, .. } => remaining.to_string(),
        _ => unreachable!(),
    }
}

fn render_counterattack(
    content: &ContentPack,
    outcome: &PartyReactionOutcome,
    lines: &mut NarrativeLines,
) {
    let PartyReactionOutcome::Counterattack {
        actor,
        target,
        damage,
        remaining,
        kind,
        message,
        no_effect,
        ..
    } = outcome
    else {
        unreachable!();
    };
    let damage = damage.to_string();
    let remaining = remaining.to_string();
    push_message(
        lines,
        content,
        if *no_effect {
            "combat.no_effect"
        } else {
            message
        },
        &[
            ("actor", if *no_effect { target } else { actor }),
            ("target", target),
            ("damage", damage.as_str()),
            ("remaining", remaining.as_str()),
            ("kind", kind),
        ],
    );
}

fn render_support(
    content: &ContentPack,
    outcome: &PartyReactionOutcome,
    lines: &mut NarrativeLines,
) {
    let PartyReactionOutcome::Support {
        actor,
        target,
        amount,
        remaining,
        stat,
        message,
        ..
    } = outcome
    else {
        unreachable!();
    };
    let amount = amount.to_string();
    let remaining = remaining.to_string();
    push_message(
        lines,
        content,
        message,
        &[
            ("actor", actor),
            ("target", target),
            ("amount", amount.as_str()),
            ("remaining", remaining.as_str()),
            ("stat", stat),
        ],
    );
}

fn group_key<'a>(content: &ContentPack, count: usize, keys: (&'a str, &'a str)) -> Option<&'a str> {
    if count >= 4 && content.messages.contains_key(keys.1) {
        Some(keys.1)
    } else if count > 1 && content.messages.contains_key(keys.0) {
        Some(keys.0)
    } else {
        None
    }
}
