use super::{ContentValidator, Diagnostics};
use crate::content::types::{
    ActorDefinition, ContentPack, PartyController, SkillAutonomousAction, SkillAutonomousTarget,
    SkillAutonomousUse, SkillDefinition, SkillKind,
};
use serde_json::{Map, Value};
use std::collections::BTreeSet;

/// Validates the shape of `skills.json` itself: pack-level and per-skill SP
/// thresholds, scaling parameters, and autonomous-use declarations.
pub(super) struct SkillDeclarationsValidator;

impl ContentValidator for SkillDeclarationsValidator {
    fn validate(&self, pack: &ContentPack, diagnostics: &mut Diagnostics) {
        if let Some(thresholds) = &pack.skills.sp_thresholds {
            validate_sp_thresholds("skills.json", thresholds, diagnostics);
        }
        let mut declared = BTreeSet::new();
        for skill in &pack.skills.skills {
            if skill.id.trim().is_empty() {
                diagnostics.push("skills.json contains a skill with an empty id");
                continue;
            }
            if !declared.insert(skill.id.as_str()) {
                diagnostics.push(format!(
                    "skills.json declares duplicate skill '{}'",
                    skill.id
                ));
            }
            validate_skill_definition(skill, diagnostics);
        }
    }
}

fn validate_sp_thresholds(context: &str, thresholds: &[u32], diagnostics: &mut Diagnostics) {
    if thresholds.is_empty() {
        diagnostics.push(format!("{context} sp_thresholds cannot be empty"));
    } else if !thresholds.windows(2).all(|w| w[0] < w[1]) {
        diagnostics.push(format!(
            "{context} sp_thresholds must be strictly increasing"
        ));
    }
}

fn validate_skill_definition(skill: &SkillDefinition, diagnostics: &mut Diagnostics) {
    if let Some(thresholds) = &skill.sp_thresholds {
        validate_sp_thresholds(&format!("skill '{}'", skill.id), thresholds, diagnostics);
    }
    validate_scaling_parameters(skill, diagnostics);
    if let Some(action) = &skill.player_action
        && action.id.trim().is_empty()
    {
        diagnostics.push(format!(
            "skill '{}' has a player action with an empty id",
            skill.id
        ));
    }
    let mut autonomous_ids = BTreeSet::new();
    for autonomous in &skill.autonomous {
        if autonomous.id.trim().is_empty() {
            diagnostics.push(format!(
                "skill '{}' has an autonomous use with an empty id",
                skill.id
            ));
            continue;
        }
        if !autonomous_ids.insert(autonomous.id.as_str()) {
            diagnostics.push(format!(
                "skill '{}' declares autonomous use '{}' more than once",
                skill.id, autonomous.id
            ));
            continue;
        }
        validate_autonomous_use(skill, autonomous, diagnostics);
    }
}

fn validate_scaling_parameters(skill: &SkillDefinition, diagnostics: &mut Diagnostics) {
    let Some(scaling) = &skill.scaling else {
        return;
    };
    for (param_name, param) in &scaling.parameters {
        if param.step_levels == Some(0) {
            diagnostics.push(format!(
                "skill '{}' parameter '{}' step_levels must be greater than 0",
                skill.id, param_name
            ));
        }
    }
}

fn validate_autonomous_use(
    skill: &SkillDefinition,
    use_: &SkillAutonomousUse,
    diagnostics: &mut Diagnostics,
) {
    let valid_kind = matches!(
        (skill.kind, use_.action),
        (Some(SkillKind::Attack), SkillAutonomousAction::Strike)
            | (Some(SkillKind::Heal), SkillAutonomousAction::Heal)
    );
    if !valid_kind {
        diagnostics.push(format!(
            "skill '{}' autonomous use '{}' is incompatible with its skill kind",
            skill.id, use_.id
        ));
    }
    let valid_target = matches!(
        (use_.action, use_.target),
        (SkillAutonomousAction::Strike, SkillAutonomousTarget::Player)
            | (
                SkillAutonomousAction::Heal,
                SkillAutonomousTarget::LowestHealthHostileAlly
            )
    );
    if !valid_target {
        diagnostics.push(format!(
            "skill '{}' autonomous use '{}' has an incompatible target",
            skill.id, use_.id
        ));
    }
}

/// Validates each actor's skill assignments against the declared skills.
pub(super) struct ActorSkillAssignmentsValidator;

impl ContentValidator for ActorSkillAssignmentsValidator {
    fn validate(&self, pack: &ContentPack, diagnostics: &mut Diagnostics) {
        for actor in &pack.actors {
            validate_actor_skill_assignments(pack, actor, diagnostics);
        }
    }
}

fn validate_actor_skill_assignments(
    pack: &ContentPack,
    actor: &ActorDefinition,
    diagnostics: &mut Diagnostics,
) {
    let mut assigned = BTreeSet::new();
    for assignment in &actor.skills {
        let skill_id = assignment.id();
        let Some(skill) = pack.skill(skill_id) else {
            diagnostics.push(format!(
                "actor '{}' declares unknown skill '{skill_id}'",
                actor.id
            ));
            continue;
        };
        if !assigned.insert(skill_id) {
            diagnostics.push(format!(
                "actor '{}' declares skill '{skill_id}' more than once",
                actor.id
            ));
        }
        if skill.kind == Some(SkillKind::Heal) && assignment.power().is_none() {
            diagnostics.push(format!(
                "skills require actor '{}' to configure power for heal skill '{}'",
                actor.id, skill_id
            ));
        }
    }
}

/// Validates every cross-reference that points *at* a skill: actions, party
/// combat rules, level unlocks, and hook effects.
pub(super) struct SkillReferencesValidator;

impl ContentValidator for SkillReferencesValidator {
    fn validate(&self, pack: &ContentPack, diagnostics: &mut Diagnostics) {
        validate_action_skill_references(pack, diagnostics);
        validate_party_skill_references(pack, diagnostics);
        validate_level_skill_unlocks(pack, diagnostics);
        validate_hook_skill_references(pack, diagnostics);
        validate_action_skill_bindings(pack, diagnostics);
    }
}

fn validate_action_skill_references(pack: &ContentPack, diagnostics: &mut Diagnostics) {
    for action in &pack.actions {
        if !action.skill_id.is_empty() && pack.skill(&action.skill_id).is_none() {
            diagnostics.push(format!(
                "action '{}' references unknown skill '{}'",
                action.id, action.skill_id
            ));
        }
    }
}

fn validate_party_skill_references(pack: &ContentPack, diagnostics: &mut Diagnostics) {
    for party in &pack.settings.parties {
        let player_controlled = party.controller == PartyController::Player;
        for rule in &party.combat_rules {
            if player_controlled && rule.skill_id.is_empty() {
                diagnostics.push(format!(
                    "skills require player party combat rule '{}' to name a skill",
                    rule.id
                ));
            }
            if !rule.skill_id.is_empty() && pack.skill(&rule.skill_id).is_none() {
                diagnostics.push(format!(
                    "party combat rule '{}' references unknown skill '{}'",
                    rule.id, rule.skill_id
                ));
            }
        }
    }
}

fn validate_level_skill_unlocks(pack: &ContentPack, diagnostics: &mut Diagnostics) {
    for level in pack
        .levels
        .default
        .iter()
        .chain(pack.levels.actors.values().flatten())
    {
        for skill_id in &level.unlocks {
            if pack.skill(skill_id).is_none() {
                diagnostics.push(format!("levels.json unlocks unknown skill '{skill_id}'"));
            }
        }
    }
}

fn validate_hook_skill_references(pack: &ContentPack, diagnostics: &mut Diagnostics) {
    for (hook_id, hook) in &pack.hooks {
        validate_hook_skill_grants(pack, hook_id, hook, diagnostics);
    }
}

fn validate_hook_skill_grants(
    pack: &ContentPack,
    hook_id: &str,
    value: &Value,
    diagnostics: &mut Diagnostics,
) {
    match value {
        Value::Array(values) => {
            for value in values {
                validate_hook_skill_grants(pack, hook_id, value, diagnostics);
            }
        }
        Value::Object(object) => {
            if object.get("kind").and_then(Value::as_str) == Some("grant_skill") {
                validate_hook_skill_grant(pack, hook_id, object, diagnostics);
            }
            for value in object.values() {
                validate_hook_skill_grants(pack, hook_id, value, diagnostics);
            }
        }
        _ => {}
    }
}

fn validate_hook_skill_grant(
    pack: &ContentPack,
    hook_id: &str,
    object: &Map<String, Value>,
    diagnostics: &mut Diagnostics,
) {
    let skill_id = object
        .get("skill_id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if pack.skill(skill_id).is_none() {
        diagnostics.push(format!(
            "hook '{hook_id}' grants unknown skill '{skill_id}'"
        ));
    }
    if let Some(actor_id) = object.get("actor_id").and_then(Value::as_str)
        && pack.actor(actor_id).is_none()
    {
        diagnostics.push(format!(
            "hook '{hook_id}' grants a skill to unknown actor '{actor_id}'"
        ));
    }
}

fn validate_action_skill_bindings(pack: &ContentPack, diagnostics: &mut Diagnostics) {
    for action in &pack.actions {
        if matches!(action.command.as_str(), "ATTACK" | "TRACE" | "TELEPORT")
            && action.skill_id.is_empty()
        {
            diagnostics.push(format!(
                "skills require action '{}' to name a skill",
                action.id
            ));
        }
    }
}

/// Movement-only behavior is allowed: hold rules are permitted and attacks are
/// authored in `skills.json`. Combat actors, however, must own a skill with an
/// autonomous strike.
pub(super) struct CombatActorSkillsValidator;

impl ContentValidator for CombatActorSkillsValidator {
    fn validate(&self, pack: &ContentPack, diagnostics: &mut Diagnostics) {
        for actor in &pack.actors {
            if actor_needs_strike(actor) && !actor_has_autonomous_strike(pack, actor) {
                diagnostics.push(format!(
                    "combat actor '{}' lacks a skill with autonomous strike behavior",
                    actor.id
                ));
            }
        }
    }
}

fn actor_needs_strike(actor: &ActorDefinition) -> bool {
    actor.attackable || actor.initial_hostile
}

fn actor_has_autonomous_strike(pack: &ContentPack, actor: &ActorDefinition) -> bool {
    actor.skills.iter().any(|assignment| {
        pack.skill(assignment.id()).is_some_and(|skill| {
            skill
                .autonomous
                .iter()
                .any(|use_| use_.action == SkillAutonomousAction::Strike)
        })
    })
}
