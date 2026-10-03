//! Hostile-action policy: decides *which* actors declare hostile strikes on a
//! background tick. The reducer resolves the mechanics of each declared
//! [`WorldEvent::HostileStrike`] generically; this module only selects.
//!
//! The selection policy itself is content-driven: each pack's `behavior.json`
//! `strike` rule decides per actor (see [`crate::engine::behavior`]).
//! Rust retains reducer eligibility backstops, cooldown scheduling, and damage
//! mechanics, but does not add another selection policy here.

use crate::content::types::{
    ContentPack, SkillAutonomousAction, SkillAutonomousTarget, SkillAutonomousUse, SkillKind,
};
use crate::engine::behavior;
use crate::engine::combat_targets::select_hostile_target;
use crate::engine::events::WorldEvent;
use crate::engine::state::{ActorStance, WorldState};

/// Rules mode: an actor declares a strike or heal when ready.
/// If an actor has healing capability and any wounded hostile ally (or self)
/// is present in the room, it prioritizes healing.
/// Otherwise, it declares a strike against the player.
pub(crate) fn plan_rules_hostile_actions(
    content: &ContentPack,
    state: &WorldState,
) -> Vec<WorldEvent> {
    if state.phase != crate::engine::state::GamePhase::Active {
        return Vec::new();
    }
    let actor_ids = state
        .onstage_actors(content)
        .map(|actor| actor.id.clone())
        .collect::<Vec<_>>();
    actor_ids
        .into_iter()
        .filter_map(|actor_id| {
            if let Some(event) = centralized_skill_event(content, state, &actor_id) {
                return Some(event);
            }
            behavior::strike_event(content, state, &actor_id)
        })
        .collect()
}

fn centralized_skill_event(
    content: &ContentPack,
    state: &WorldState,
    actor_id: &str,
) -> Option<WorldEvent> {
    let mut uses = content
        .skills
        .skills
        .iter()
        .enumerate()
        .filter(|(_, skill)| state.actor_has_skill(actor_id, &skill.id))
        .flat_map(|(skill_index, skill)| {
            skill
                .autonomous
                .iter()
                .enumerate()
                .map(move |(use_index, use_)| (use_.priority, skill_index, use_index, skill, use_))
        })
        .collect::<Vec<_>>();
    uses.sort_by_key(|(priority, skill_index, use_index, _, _)| {
        (*priority, *skill_index, *use_index)
    });

    uses.into_iter().find_map(|(_, _, _, skill, use_)| {
        if !behavior::skill_rule_emits(
            &use_.rule,
            use_.action.effect_kind(),
            content,
            state,
            actor_id,
        ) {
            return None;
        }
        match use_.action {
            SkillAutonomousAction::Strike => {
                select_hostile_target(content, state, actor_id)?;
                Some(WorldEvent::HostileStrike {
                    actor_id: actor_id.to_string(),
                })
            }
            SkillAutonomousAction::Heal => {
                let (amount, message) = hostile_healing_spec(content, state, actor_id, &skill.id)?;
                let target_id = select_autonomous_target(content, state, actor_id, use_)?;
                Some(WorldEvent::HostileHeal {
                    actor_id: actor_id.to_string(),
                    target_id,
                    amount,
                    message,
                })
            }
        }
    })
}

/// Healing available to a hostile through its assigned heal skill.
fn hostile_healing_spec(
    content: &ContentPack,
    state: &WorldState,
    actor_id: &str,
    skill_id: &str,
) -> Option<(i32, String)> {
    let skill = content.skill(skill_id)?;
    if skill.kind != Some(SkillKind::Heal) {
        return None;
    }
    let assignment = state
        .actor(content, actor_id)
        .and_then(|actor| actor.skill(skill_id));
    let amount = assignment.and_then(|assignment| assignment.power())?;
    let message = assignment
        .and_then(|assignment| assignment.narration_key())
        .map(str::to_string)
        .unwrap_or_default();
    Some((amount, message))
}

fn select_autonomous_target(
    content: &ContentPack,
    state: &WorldState,
    actor_id: &str,
    use_: &SkillAutonomousUse,
) -> Option<String> {
    match use_.target {
        SkillAutonomousTarget::Player => {
            return Some(content.settings.combat.player_actor_id.clone());
        }
        SkillAutonomousTarget::LowestHealthHostileAlly => {}
    }
    let health_stat = &content.settings.combat.health_stat_id;
    let default_room = content
        .actor(actor_id)
        .map(|a| a.room_id.as_str())
        .unwrap_or_default();
    let room_id = state.actor_room_id(actor_id, default_room);

    // Optional self-preservation threshold is authored with the skill use.
    let self_hp = state.actor_stat(actor_id, health_stat);
    let self_max = state
        .actor_stat_maximum(content, actor_id, health_stat)
        .max(1);
    if let Some(percent) = use_.prefer_self_below_percent
        && i64::from(self_hp) * 100 < i64::from(self_max) * i64::from(percent)
    {
        return Some(actor_id.to_string());
    }

    // Otherwise, collect all wounded living hostile allies in the same room (including self if below max).
    let mut wounded_allies = state
        .onstage_actors(content)
        .map(|actor| actor.id.clone())
        .filter(|other_id| {
            state.actor_is_in_room(content, other_id, &room_id)
                && state.stance(other_id) == ActorStance::Hostile
                && state.actor_stat(other_id, health_stat) > 0
                && state.actor_stat(other_id, health_stat)
                    < state.actor_stat_maximum(content, other_id, health_stat)
        })
        .collect::<Vec<_>>();

    if wounded_allies.is_empty() {
        return None;
    }

    // Select the wounded ally with the lowest health percentage.
    // If self is equally low, prioritize self.
    wounded_allies.sort_by(|left, right| {
        let (left_hp, left_max) = (
            state.actor_stat(left, health_stat),
            state.actor_stat_maximum(content, left, health_stat).max(1),
        );
        let (right_hp, right_max) = (
            state.actor_stat(right, health_stat),
            state.actor_stat_maximum(content, right, health_stat).max(1),
        );
        (i64::from(left_hp) * i64::from(right_max))
            .cmp(&(i64::from(right_hp) * i64::from(left_max)))
            .then_with(|| {
                if left == actor_id {
                    std::cmp::Ordering::Less
                } else if right == actor_id {
                    std::cmp::Ordering::Greater
                } else {
                    left.cmp(right)
                }
            })
    });

    wounded_allies.into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::state::{ActorStance, GamePhase};

    /// The strike decision re-declared as a neuron `effect_table` rule: strike
    /// when the actor is a living hostile sharing a room with a party target
    /// whose attack cooldown has elapsed.
    fn strike_default_rule() -> serde_json::Value {
        serde_json::json!({
            "rule": "effect_table",
            "rule_config": {
                "cases_path": "rules",
                "next_on_match": "continue",
                "next_on_default": "continue",
                "default_payload_template": { "effects": [] }
            },
            "input_overlay": {
                "rules": [{
                    "conditions": [
                        { "path": "actor.stance", "operator": "equal", "value": "hostile" },
                        { "path": "actor.alive", "operator": "equal", "value": true },
                        { "path": "world.cooldown_elapsed", "operator": "equal", "value": true },
                        { "path": "world.opposing_party_actor_in_room", "operator": "equal", "value": true }
                    ],
                    "payload_template": { "kind": "strike" }
                }]
            }
        })
    }

    /// Fixture reusing the synthetic pack's own actors: the first becomes the
    /// hostile creature sharing the player's room, the second waits elsewhere.
    /// Mutating definitions in place keeps the pack's id index consistent.
    fn hostile_fixture() -> (ContentPack, WorldState, String, String) {
        let mut content = crate::engine::test_fixtures::minimal_test_pack();
        assert!(content.actors.len() >= 2, "fixture needs two actors");
        content.skills.skills = vec![crate::content::types::SkillDefinition {
            id: "strike".to_string(),
            label: "Strike".to_string(),
            kind: Some(SkillKind::Attack),
            autonomous: vec![crate::content::types::SkillAutonomousUse {
                id: "hostile-strike".to_string(),
                priority: 20,
                action: SkillAutonomousAction::Strike,
                target: SkillAutonomousTarget::Player,
                rule: strike_default_rule(),
                prefer_self_below_percent: None,
            }],
            ..Default::default()
        }];
        content.skill_index.insert("strike".to_string(), 0);
        let brute_id = content.actors[0].id.clone();
        let bystander_id = content.actors[1].id.clone();
        for actor in &mut content.actors[..2] {
            actor.skills = vec![crate::content::types::ActorSkillAssignment::Id(
                "strike".to_string(),
            )];
        }
        content.actors[0].room_id = "hall".to_string();
        content.actors[0].attack_interval_minutes = Some(3);
        content.actors[1].room_id = "annex".to_string();
        let mut player = content.actors[1].clone();
        player.id = content.settings.combat.player_actor_id.clone();
        player.room_id = "hall".to_string();
        content.actors.push(player);
        crate::engine::test_fixtures::rebuild_test_pack_indexes(&mut content);
        let mut state = WorldState::new(&content);
        state.current_room_id = "hall".to_string();
        state
            .actor_stats
            .entry(brute_id.clone())
            .or_default()
            .insert("hp".to_string(), 6);
        state
            .actor_stats
            .entry(brute_id.clone())
            .or_default()
            .insert("strength".to_string(), 3);
        (content, state, brute_id, bystander_id)
    }

    #[test]
    fn rules_policy_selects_due_hostile_with_party_target_in_room() {
        let (content, mut state, brute_id, _) = hostile_fixture();
        state.set_stance(&brute_id, ActorStance::Hostile);

        let events = plan_rules_hostile_actions(&content, &state);

        assert_eq!(
            events,
            vec![WorldEvent::HostileStrike { actor_id: brute_id }]
        );
    }

    #[test]
    fn rules_policy_skips_future_cooldown_dead_and_distant_actors() {
        let (content, mut state, brute_id, bystander_id) = hostile_fixture();
        state.set_stance(&brute_id, ActorStance::Hostile);
        state
            .next_hostile_strike_at
            .insert(brute_id.clone(), state.current_time_minutes + 5);
        // Dead hostile in the same room.
        state.set_stance(&bystander_id, ActorStance::Hostile);
        state
            .actor_stats
            .entry(bystander_id.clone())
            .or_default()
            .insert("hp".to_string(), 0);

        let events = plan_rules_hostile_actions(&content, &state);

        assert!(events.is_empty(), "no actor was eligible, got {events:?}");
    }

    #[test]
    fn rules_policy_skips_hostile_in_another_room() {
        let (content, mut state, brute_id, _) = hostile_fixture();
        state.current_room_id = "annex".to_string();
        state.set_stance(&brute_id, ActorStance::Hostile);

        assert!(plan_rules_hostile_actions(&content, &state).is_empty());
    }

    #[test]
    fn rules_policy_yields_nothing_when_not_active() {
        let (content, mut state, brute_id, _) = hostile_fixture();
        state.phase = GamePhase::GameEnded;
        state.set_stance(&brute_id, ActorStance::Hostile);

        assert!(plan_rules_hostile_actions(&content, &state).is_empty());
    }

    /// Healing potency and narration stay on the actor's skill assignment.
    #[test]
    fn hostile_healing_keeps_each_actors_own_potency_and_line() {
        let mut content = crate::engine::test_fixtures::minimal_test_pack();
        assert!(content.actors.len() >= 2, "fixture needs two actors");
        content.skills.skills = vec![crate::content::types::SkillDefinition {
            id: "heal".to_string(),
            label: "Heal".to_string(),
            kind: Some(SkillKind::Heal),
            ..Default::default()
        }];
        content.skill_index.insert("heal".to_string(), 0);
        let healer_id = content.actors[0].id.clone();
        let bare_id = content.actors[1].id.clone();
        content.actors[0].skills = vec![crate::content::types::ActorSkillAssignment::Configured(
            crate::content::types::ActorSkillConfig {
                id: "heal".to_string(),
                power: Some(10),
                narration_key: Some("combat.sylvan_heal".to_string()),
            },
        )];
        content.actors[1].skills = vec![crate::content::types::ActorSkillAssignment::Configured(
            crate::content::types::ActorSkillConfig {
                id: "heal".to_string(),
                power: Some(4),
                narration_key: Some("combat.bishop_heal".to_string()),
            },
        )];
        let state = WorldState::new(&content);

        assert_eq!(
            hostile_healing_spec(&content, &state, &healer_id, "heal"),
            Some((10, "combat.sylvan_heal".to_string()))
        );
        assert_eq!(
            hostile_healing_spec(&content, &state, &bare_id, "heal"),
            Some((4, "combat.bishop_heal".to_string()))
        );
    }

    #[test]
    fn an_actor_without_the_heal_skill_cannot_heal() {
        let content = crate::engine::test_fixtures::minimal_test_pack();
        let state = WorldState::new(&content);
        assert_eq!(
            hostile_healing_spec(&content, &state, &content.actors[0].id, "heal"),
            None
        );
    }
}
