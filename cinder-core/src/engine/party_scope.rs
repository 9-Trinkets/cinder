use crate::content::types::{
    ContentPack, PartyCandidatePriority, PartyCombatDecisionRule, PartyDecisionCondition,
};
use crate::engine::combat_targets::party_actor_is_in_room;
use crate::engine::state::{PLAYER_PARTY_ID, WorldState};
use std::cmp::Ordering;

/// The command group a reactive behavior draws its rules and roster from.
pub(crate) struct PartyScope {
    pub(crate) party_id: String,
    pub(crate) leader_id: String,
    pub(crate) members: Vec<String>,
}

impl PartyScope {
    /// The protected actor plus every member, deduplicated, in a stable order.
    pub(crate) fn allies(&self) -> Vec<&str> {
        let mut ids = vec![self.leader_id.as_str()];
        for member in &self.members {
            if member != &self.leader_id {
                ids.push(member.as_str());
            }
        }
        ids
    }
}

/// The party a protective reaction belongs to: the player party for the player
/// actor, otherwise the (possibly AI-driven) party the protected actor is in.
pub(crate) fn scope_for_protected(
    content: &ContentPack,
    state: &WorldState,
    protected_id: &str,
) -> Option<PartyScope> {
    if content.is_player_actor(protected_id) {
        let party = state.player_party()?;
        return Some(PartyScope {
            party_id: party.id.clone(),
            leader_id: party
                .leader
                .clone()
                .unwrap_or_else(|| content.settings.combat.player_actor_id.clone()),
            members: party.members.iter().cloned().collect(),
        });
    }
    let party = state.party_of_actor(protected_id)?;
    let leader_id = party
        .leader
        .clone()
        .or_else(|| {
            party
                .is_player_controlled()
                .then(|| content.settings.combat.player_actor_id.clone())
        })
        .unwrap_or_else(|| protected_id.to_string());
    Some(PartyScope {
        party_id: party.id.clone(),
        leader_id,
        members: party.members.iter().cloned().collect(),
    })
}

pub(crate) fn party_rules<'a>(
    content: &'a ContentPack,
    party_id: &str,
) -> &'a [PartyCombatDecisionRule] {
    content
        .settings
        .parties
        .iter()
        .find(|definition| {
            let id = if definition.id.is_empty() {
                PLAYER_PARTY_ID
            } else {
                definition.id.as_str()
            };
            id == party_id
        })
        .map(|definition| definition.combat_rules.as_slice())
        .unwrap_or(&[])
}

pub(crate) fn actor_is_reaction_eligible(
    content: &ContentPack,
    state: &WorldState,
    actor_id: &str,
    room_id: &str,
) -> bool {
    actor_id != content.settings.combat.player_actor_id
        && state.actor_is_in_room(content, actor_id, room_id)
        && !state.actor_is_defeated(actor_id, &content.settings.combat.health_stat_id)
        && state
            .relations
            .party_reaction_ready_at
            .get(actor_id)
            .is_none_or(|ready_at| *ready_at <= state.clock.current_time_minutes)
}

/// Whether an actor may perform a reactive behavior.
pub(crate) fn actor_may_use_reaction(state: &WorldState, actor_id: &str, skill_id: &str) -> bool {
    if skill_id.is_empty() {
        return true;
    }
    state.actor_has_skill(actor_id, skill_id)
}

pub(crate) fn condition_matches(
    content: &ContentPack,
    state: &WorldState,
    scope: &PartyScope,
    actor_id: &str,
    room_id: &str,
    condition: &PartyDecisionCondition,
) -> bool {
    match condition {
        PartyDecisionCondition::OrderIs { orders } => state
            .party_order(content, actor_id)
            .is_some_and(|order| orders.contains(&order)),
        PartyDecisionCondition::ActorHealthAtMostPercent { percent } => {
            health_percent_at_most(content, state, actor_id, *percent)
        }
        PartyDecisionCondition::ActorHealthAtLeastPercent { percent } => {
            !health_percent_at_most(content, state, actor_id, percent.saturating_sub(1))
        }
        PartyDecisionCondition::PlayerHealthAtMostPercent { percent } => health_percent_at_most(
            content,
            state,
            &content.settings.combat.player_actor_id,
            *percent,
        ),
        PartyDecisionCondition::LeaderHealthAtMostPercent { percent } => {
            health_percent_at_most(content, state, &scope.leader_id, *percent)
        }
        PartyDecisionCondition::ActorHasTag { tag } => state
            .actor(content, actor_id)
            .is_some_and(|actor| actor.tags.contains(tag)),
        PartyDecisionCondition::AnyAllyWounded => scope
            .allies()
            .into_iter()
            .filter(|other_id| party_actor_is_in_room(content, state, other_id, room_id))
            .any(|other_id| {
                let (current, maximum) = health_values(content, state, other_id);
                current < maximum
            }),
        PartyDecisionCondition::AnyAllyHealthAtMostPercent { percent } => scope
            .allies()
            .into_iter()
            .filter(|other_id| party_actor_is_in_room(content, state, other_id, room_id))
            .any(|other_id| health_percent_at_most(content, state, other_id, *percent)),
    }
}

fn health_percent_at_most(
    content: &ContentPack,
    state: &WorldState,
    actor_id: &str,
    percent: u8,
) -> bool {
    let (current, maximum) = health_values(content, state, actor_id);
    i64::from(current) * 100 <= i64::from(maximum) * i64::from(percent)
}

fn health_values(content: &ContentPack, state: &WorldState, actor_id: &str) -> (i32, i32) {
    let health_stat = &content.settings.combat.health_stat_id;
    let current = state.actor_stat(actor_id, health_stat).max(0);
    let maximum = state
        .actor_stat_maximum(content, actor_id, health_stat)
        .max(1);
    (current, maximum)
}

pub(crate) fn lowest_health_ally(
    content: &ContentPack,
    state: &WorldState,
    actor_id: &str,
    room_id: &str,
) -> Option<String> {
    let scope = scope_for_protected(content, state, actor_id)?;
    scope
        .allies()
        .into_iter()
        .filter(|other_id| {
            party_actor_is_in_room(content, state, other_id, room_id)
                && !state.actor_is_defeated(other_id, &content.settings.combat.health_stat_id)
        })
        .min_by(|left, right| {
            let (left_current, left_max) = health_values(content, state, left);
            let (right_current, right_max) = health_values(content, state, right);
            (i64::from(left_current) * i64::from(right_max))
                .cmp(&(i64::from(right_current) * i64::from(left_max)))
        })
        .map(str::to_string)
}

#[derive(Debug)]
pub(crate) struct Candidate<'a> {
    pub(crate) actor_id: &'a str,
    pub(crate) content_index: usize,
}

pub(crate) fn compare_candidates(
    content: &ContentPack,
    state: &WorldState,
    rule: &PartyCombatDecisionRule,
    left: &Candidate<'_>,
    right: &Candidate<'_>,
) -> Ordering {
    for priority in &rule.candidate_priority {
        let ordering = match priority {
            PartyCandidatePriority::HighestHealthPercent => {
                let (left_current, left_max) = health_values(content, state, left.actor_id);
                let (right_current, right_max) = health_values(content, state, right.actor_id);
                (i64::from(right_current) * i64::from(left_max))
                    .cmp(&(i64::from(left_current) * i64::from(right_max)))
            }
            PartyCandidatePriority::HighestDefense => state
                .actor_combat_defense(content, right.actor_id)
                .cmp(&state.actor_combat_defense(content, left.actor_id)),
            PartyCandidatePriority::ContentOrder => left.content_index.cmp(&right.content_index),
        };
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    left.content_index.cmp(&right.content_index)
}
