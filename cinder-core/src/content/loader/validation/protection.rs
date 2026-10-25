use super::{ContentValidator, Diagnostics};
use crate::content::types::{
    AdvanceEffect, BeatDefinition, ContentPack, ProtectionRuleDefinition, SurvivorRewardDefinition,
};

/// Validates beat-stage transitions, spawn schedules, on-advance/completion
/// rewards, and room protection rules.
pub(super) struct ProtectionRulesValidator;

impl ContentValidator for ProtectionRulesValidator {
    fn validate(&self, pack: &ContentPack, diagnostics: &mut Diagnostics) {
        for stage in &pack.beats.stages {
            validate_stage_transitions(pack, stage, diagnostics);
            validate_stage_effects(pack, &stage.id, &stage.on_advance_effects, diagnostics);
            validate_stage_effects(pack, &stage.id, &stage.completion_effects, diagnostics);
            validate_actor_spawn_schedule(pack, stage, diagnostics);
            if let Some(rule) = &stage.protection_rule {
                validate_protection_rule(pack, stage, rule, diagnostics);
            }
        }
    }
}

fn validate_stage_transitions(
    pack: &ContentPack,
    stage: &BeatDefinition,
    diagnostics: &mut Diagnostics,
) {
    for next_stage_id in stage.next_stage_ids.iter().chain(
        stage
            .advance_signals
            .iter()
            .filter_map(|signal| signal.next_stage_ids())
            .flatten(),
    ) {
        if !pack
            .beats
            .stages
            .iter()
            .any(|candidate| candidate.id == *next_stage_id)
        {
            diagnostics.push(format!(
                "stage '{}' references unknown next stage '{}'",
                stage.id, next_stage_id
            ));
        }
    }
}

fn validate_actor_spawn_schedule(
    pack: &ContentPack,
    stage: &BeatDefinition,
    diagnostics: &mut Diagnostics,
) {
    let Some(schedule) = &stage.actor_spawn_schedule else {
        return;
    };
    if schedule.rout_on_stop && schedule.stop_story_var.is_empty() {
        diagnostics.push(format!(
            "stage '{}' routed spawn schedule requires stop_story_var",
            stage.id
        ));
    }
    if !schedule.rout_message.is_empty() && !pack.messages.contains_key(&schedule.rout_message) {
        diagnostics.push(format!(
            "stage '{}' spawn schedule references unknown rout message '{}'",
            stage.id, schedule.rout_message
        ));
    }
}

fn validate_protection_rule(
    pack: &ContentPack,
    stage: &BeatDefinition,
    rule: &ProtectionRuleDefinition,
    diagnostics: &mut Diagnostics,
) {
    if pack.room(&rule.room_id).is_none() {
        diagnostics.push(format!(
            "stage '{}' protection rule references unknown room '{}'",
            stage.id, rule.room_id
        ));
    }
    if rule.hostile_tag.trim().is_empty() {
        diagnostics.push(format!(
            "stage '{}' protection rule requires a hostile_tag",
            stage.id
        ));
    }
    if rule.protected_actor_ids.is_empty() {
        diagnostics.push(format!(
            "stage '{}' protection rule requires protected_actor_ids",
            stage.id
        ));
    }
    for actor_id in &rule.protected_actor_ids {
        if pack.actor(actor_id).is_none() {
            diagnostics.push(format!(
                "stage '{}' protection rule references unknown actor '{}'",
                stage.id, actor_id
            ));
        }
    }
    if rule.breach_minutes == 0 {
        diagnostics.push(format!(
            "stage '{}' protection rule requires breach_minutes greater than zero",
            stage.id
        ));
    }
    if rule.failure_death_count > rule.protected_actor_ids.len() {
        diagnostics.push(format!(
            "stage '{}' protection rule failure_death_count exceeds its protected actor count",
            stage.id
        ));
    }
    validate_protection_messages(pack, stage, rule, diagnostics);
}

fn validate_protection_messages(
    pack: &ContentPack,
    stage: &BeatDefinition,
    rule: &ProtectionRuleDefinition,
    diagnostics: &mut Diagnostics,
) {
    for message_key in [
        &rule.warning_message,
        &rule.cleared_message,
        &rule.death_message,
    ] {
        if !message_key.is_empty() && !pack.messages.contains_key(message_key) {
            diagnostics.push(format!(
                "stage '{}' protection rule references unknown message '{}'",
                stage.id, message_key
            ));
        }
    }
}

fn validate_stage_effects(
    pack: &ContentPack,
    stage_id: &str,
    effects: &[AdvanceEffect],
    diagnostics: &mut Diagnostics,
) {
    for effect in effects {
        match effect {
            AdvanceEffect::GrantItem {
                item_id, message, ..
            }
            | AdvanceEffect::RemoveItem {
                item_id, message, ..
            } => {
                validate_reward_reference(pack, stage_id, item_id, message, diagnostics);
            }
            AdvanceEffect::ResolveSurvivorRewards {
                rewards,
                all_survived_message,
                ..
            } => {
                validate_survivor_rewards(
                    pack,
                    stage_id,
                    rewards,
                    all_survived_message,
                    diagnostics,
                );
            }
            AdvanceEffect::ConsumeItem { item_id } => {
                validate_reward_reference(pack, stage_id, item_id, "", diagnostics);
            }
            AdvanceEffect::AdjustActorStat { .. }
            | AdvanceEffect::AdjustPairStat { .. }
            | AdvanceEffect::RunHook { .. }
            | AdvanceEffect::EmitSignal { .. }
            | AdvanceEffect::SetStoryVar { .. }
            | AdvanceEffect::SetFollowsPlayer { .. }
            | AdvanceEffect::SetFeatureState { .. }
            | AdvanceEffect::QueueSequence { .. } => {}
        }
    }
}

fn validate_survivor_rewards(
    pack: &ContentPack,
    stage_id: &str,
    rewards: &[SurvivorRewardDefinition],
    all_survived_message: &str,
    diagnostics: &mut Diagnostics,
) {
    if !all_survived_message.is_empty() && !pack.messages.contains_key(all_survived_message) {
        diagnostics.push(format!(
            "stage '{}' survivor rewards reference unknown message '{}'",
            stage_id, all_survived_message
        ));
    }
    for reward in rewards {
        if pack.actor(&reward.actor_id).is_none() {
            diagnostics.push(format!(
                "stage '{}' survivor rewards reference unknown actor '{}'",
                stage_id, reward.actor_id
            ));
        }
        validate_reward_reference(
            pack,
            stage_id,
            &reward.item_id,
            &reward.message,
            diagnostics,
        );
    }
}

fn validate_reward_reference(
    pack: &ContentPack,
    stage_id: &str,
    item_id: &str,
    message: &str,
    diagnostics: &mut Diagnostics,
) {
    if pack.item(item_id).is_none() {
        diagnostics.push(format!(
            "stage '{}' reward references unknown item '{}'",
            stage_id, item_id
        ));
    }
    if !message.is_empty() && !pack.messages.contains_key(message) {
        diagnostics.push(format!(
            "stage '{}' reward references unknown message '{}'",
            stage_id, message
        ));
    }
}
