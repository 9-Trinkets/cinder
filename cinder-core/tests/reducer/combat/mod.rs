//! Combat tests grouped by driver: periodic room hazards, hostile strikes,
//! and attack-command driven fights (drops, xp, resistance).

mod commands;
mod periodic;
mod strikes;

use super::common::*;
use cinder_core::content::types::{ActionDefinition, CommandEffect, CommandTargetMode, ContentPack};
use cinder_core::engine::narrative::NarrativeLine;

/// Pushes the canonical "attack" action the command-driven tests use.
fn attack_action(pack: &mut ContentPack) {
    pack.actions.push(ActionDefinition {
        id: "attack".to_string(),
        command: "attack".to_string(),
        target_mode: CommandTargetMode::Actor,
        effects: vec![CommandEffect::AttackTarget],
        event_text: "{actor_name} strikes {target_actor_name}.".to_string(),
        ..ActionDefinition::default()
    });
}

/// Binds the attack command to the player actor in the lounge.
fn attack_input(target_actor_id: Option<&'static str>, target_actor_name: Option<&'static str>) -> ActorCommandInput<'static> {
    ActorCommandInput {
        actor_id: ACTOR_A_ID,
        actor_name: ACTOR_A_NAME,
        room_id: LOUNGE_ID,
        target_room_id: None,
        target_actor_id,
        target_actor_name,
        context_label: None,
        feature_id: None,
        consumable_id: None,
        freeform_text: None,
    }
}

/// Joins narration line texts the way the previous in-test transcript did.
fn transcript(lines: &[NarrativeLine]) -> String {
    lines
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}