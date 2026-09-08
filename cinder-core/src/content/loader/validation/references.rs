use super::require_known_id;
use crate::content::types::{
    ActCastMember, ActionDefinition, ActorDefinition, BeatDefinition, BeatObjectiveProgressRef,
    BeatObjectivesDefinition, BeatsDefinition, ChannelPrivacy, LevelingDefinition, MessagingChannel,
    MovementConfigDefinition, LOCAL_CHANNEL_ID,
};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;

/// Everything the content loader has loaded that cross-file validation needs.
/// Bundled into one context so validators stay small and readable.
pub(crate) struct PackContext<'a> {
    pub levels: &'a LevelingDefinition,
    pub beats: &'a BeatsDefinition,
    pub actors: &'a [ActorDefinition],
    pub movement: &'a MovementConfigDefinition,
    pub actions: &'a [ActionDefinition],
    pub beat_objectives: &'a BeatObjectivesDefinition,
    pub act_cast: &'a [ActCastMember],
    pub channels: &'a [MessagingChannel],
    pub actor_ids: &'a [&'a str],
    pub room_ids: &'a [&'a str],
    pub stage_ids: &'a [&'a str],
    pub item_ids: &'a [&'a str],
    pub room_index: &'a std::collections::HashMap<String, usize>,
    pub action_index: &'a std::collections::HashMap<String, usize>,
}

/// Runs all content-reference validation for a loaded pack, in a fixed order.
///
/// Mirrors the original inline checks in the loader: cross-file id references
/// (rooms, actors, stages, actions, items), movement rules, beat objectives,
/// and the act cast. Behavior is identical to the previous in-function loops.
pub(crate) fn validate_contents(ctx: &PackContext<'_>) -> Result<(), Box<dyn Error>> {
    for actor_id in ctx.levels.actors.keys() {
        require_known_id(
            actor_id,
            ctx.actor_ids,
            &format!("levels.actors '{actor_id}'"),
            "actors",
        )?;
    }

    for id in &ctx.beats.initial_stage_ids {
        require_known_id(
            id,
            ctx.stage_ids,
            &format!("initial_stage_id '{id}'"),
            "beats.stages",
        )?;
    }

    validate_beat_stages(
        &ctx.beats.stages,
        ctx.actor_ids,
        ctx.room_ids,
        ctx.stage_ids,
    )?;
    validate_actors(ctx.actors, ctx.room_index, ctx.item_ids)?;
    validate_movement(ctx.movement, ctx.actor_ids, ctx.room_ids, ctx.stage_ids)?;

    let action_index_keys: Vec<&str> = ctx.action_index.keys().map(|k| k.as_str()).collect();
    validate_beat_objectives(ctx.beat_objectives, ctx.stage_ids, &action_index_keys)?;

    let objective_progress_keys = objective_progress_keys(ctx.beat_objectives);
    validate_action_objective_progress(ctx.actions, ctx.stage_ids, &objective_progress_keys)?;
    validate_conditional_guidance_progress(ctx.beat_objectives, &objective_progress_keys)?;

    for room_id in &ctx.movement.unreachable_rooms {
        require_known_id(
            room_id,
            ctx.room_ids,
            &format!("movement.json unreachable_rooms entry '{room_id}'"),
            "rooms",
        )?;
    }
    validate_act_cast(ctx.act_cast, ctx.actor_ids)?;
    validate_channels(ctx.channels, ctx.actor_ids)?;

    Ok(())
}

fn validate_beat_stages(
    stages: &[BeatDefinition],
    actor_ids: &[&str],
    room_ids: &[&str],
    stage_ids: &[&str],
) -> Result<(), Box<dyn Error>> {
    let valid_operators = [
        "equal",
        "greater_than",
        "less_than",
        "gte",
        "lte",
        "not_equal",
        "array_contains",
    ];
    for stage in stages {
        if let Some(config) = &stage.stage_assignment {
            if !config.initiator_actor_id.trim().is_empty() {
                require_known_id(
                    &config.initiator_actor_id,
                    actor_ids,
                    &format!(
                        "beat '{}' stage_assignment initiator_actor_id '{}'",
                        stage.id, config.initiator_actor_id
                    ),
                    "actors",
                )?;
            }
            if !config.selected_room_id.trim().is_empty() {
                require_known_id(
                    &config.selected_room_id,
                    room_ids,
                    &format!(
                        "beat '{}' stage_assignment selected_room_id '{}'",
                        stage.id, config.selected_room_id
                    ),
                    "rooms",
                )?;
            }
            if !config.remaining_room_id.trim().is_empty() {
                require_known_id(
                    &config.remaining_room_id,
                    room_ids,
                    &format!(
                        "beat '{}' stage_assignment remaining_room_id '{}'",
                        stage.id, config.remaining_room_id
                    ),
                    "rooms",
                )?;
            }
        }
        for id in &stage.next_stage_ids {
            require_known_id(
                id,
                stage_ids,
                &format!("beat '{}' next_stage_ids contains '{id}'", stage.id),
                "beats.stages",
            )?;
        }
        for signal in &stage.advance_signals {
            for cond in signal.conditions() {
                if !valid_operators.contains(&cond.operator.as_str()) {
                    return Err(format!(
                        "beat '{}' advance_signal '{}' has unknown operator '{}'",
                        stage.id,
                        signal.signal(),
                        cond.operator
                    )
                    .into());
                }
            }
        }
    }
    Ok(())
}

fn validate_actors(
    actors: &[ActorDefinition],
    room_index: &std::collections::HashMap<String, usize>,
    item_ids: &[&str],
) -> Result<(), Box<dyn Error>> {
    for actor in actors {
        if !actor.is_offstage() && !room_index.contains_key(&actor.room_id) {
            return Err(format!(
                "actor '{}' room_id '{}' not found in rooms",
                actor.id, actor.room_id
            )
            .into());
        }
        for item_id in actor.drops.keys() {
            require_known_id(
                item_id,
                item_ids,
                &format!("actor '{}' drops '{item_id}'", actor.id),
                "items",
            )?;
        }
    }
    Ok(())
}

fn validate_channels(
    channels: &[MessagingChannel],
    actor_ids: &[&str],
) -> Result<(), Box<dyn Error>> {
    let mut seen_ids = BTreeSet::new();
    for channel in channels {
        if channel.id == LOCAL_CHANNEL_ID {
            return Err(format!(
                "channel id '{}' collides with the implicit local speech channel",
                channel.id
            )
            .into());
        }
        if !seen_ids.insert(channel.id.as_str()) {
            return Err(format!("duplicate channel id '{}'", channel.id).into());
        }
        if channel.privacy == ChannelPrivacy::Private {
            if channel.participants.is_empty() {
                return Err(format!(
                    "private channel '{}' declares no participants",
                    channel.id
                )
                .into());
            }
            for participant in &channel.participants {
                require_known_id(
                    participant,
                    actor_ids,
                    &format!("channel '{}' participant '{participant}'", channel.id),
                    "actors",
                )?;
            }
        }
    }
    Ok(())
}

fn validate_movement(
    movement: &MovementConfigDefinition,
    actor_ids: &[&str],
    room_ids: &[&str],
    stage_ids: &[&str],
) -> Result<(), Box<dyn Error>> {
    for actor_id in movement.actors.keys() {
        require_known_id(
            actor_id,
            actor_ids,
            &format!("movement.json actors key '{actor_id}'"),
            "actors",
        )?;
    }
    for (actor_id, rules) in &movement.actors {
        for (index, rule) in rules.target_rules.iter().enumerate() {
            let context = format!("movement.json actor '{actor_id}' target_rules[{index}]");
            if rule.target_room_id.trim().is_empty() && rule.target_from_story_var.trim().is_empty()
            {
                return Err(
                    format!("{context} must set target_room_id or target_from_story_var").into(),
                );
            }
            if rule.target_behavior.is_none() {
                return Err(
                    format!("{context} must set target_behavior to 'move' or 'stay'").into(),
                );
            }
            if !rule.target_room_id.trim().is_empty() {
                require_known_id(
                    &rule.target_room_id,
                    room_ids,
                    &format!("{context} target_room_id '{}'", rule.target_room_id),
                    "rooms",
                )?;
            }
            for stage_id in &rule.any_active_stage_ids {
                require_known_id(
                    stage_id,
                    stage_ids,
                    &format!("{context} any_active_stage_ids entry '{stage_id}'"),
                    "beats.stages",
                )?;
            }
        }
    }
    for stage_id in &movement.stage_locks {
        require_known_id(
            stage_id,
            stage_ids,
            &format!("movement.json stage_locks entry '{stage_id}'"),
            "beats.stages",
        )?;
    }
    Ok(())
}

fn validate_beat_objectives(
    beat_objectives: &BeatObjectivesDefinition,
    stage_ids: &[&str],
    action_index_keys: &[&str],
) -> Result<(), Box<dyn Error>> {
    for objective in &beat_objectives.objectives {
        if objective.id.trim().is_empty() {
            return Err("beat_objectives.json objectives entries require non-empty id".into());
        }
        let objective_stage_ids = objective
            .stage_ids
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        if objective_stage_ids.is_empty() {
            return Err(format!(
                "beat_objectives.json objective '{}' requires at least one stage id",
                objective.id
            )
            .into());
        }
        for stage_id in objective_stage_ids {
            require_known_id(
                stage_id,
                stage_ids,
                &format!(
                    "beat_objectives.json objective '{}' stage id '{}'",
                    objective.id, stage_id
                ),
                "beats.stages",
            )?;
        }
        let mut seen_progress_keys = BTreeSet::new();
        for progress in &objective.progress.keys {
            if progress.key.trim().is_empty() {
                return Err(format!(
                    "beat_objectives.json objective '{}' has progress entry with empty key",
                    objective.id
                )
                .into());
            }
            if !seen_progress_keys.insert(progress.key.clone()) {
                return Err(format!(
                    "beat_objectives.json objective '{}' has duplicate progress key '{}'",
                    objective.id, progress.key
                )
                .into());
            }
        }
        for priority in &objective.guidance.prioritize {
            if priority.command_id.trim().is_empty() {
                return Err(format!(
                    "beat_objectives.json objective '{}' has prioritize entry with empty command_id",
                    objective.id
                )
                .into());
            }
            require_known_id(
                &priority.command_id,
                action_index_keys,
                &format!(
                    "beat_objectives.json objective '{}' prioritize",
                    objective.id
                ),
                "actions",
            )?;
        }
        for (index, conditional) in objective.guidance.conditional.iter().enumerate() {
            for priority in &conditional.prioritize {
                if priority.command_id.trim().is_empty() {
                    return Err(format!(
                        "beat_objectives.json objective '{}' conditional guidance #{} has prioritize entry with empty command_id",
                        objective.id,
                        index + 1
                    )
                    .into());
                }
                require_known_id(
                    &priority.command_id,
                    action_index_keys,
                    &format!(
                        "beat_objectives.json objective '{}' conditional guidance #{} prioritize",
                        objective.id,
                        index + 1
                    ),
                    "actions",
                )?;
            }
        }
    }
    Ok(())
}

fn objective_progress_keys(
    beat_objectives: &BeatObjectivesDefinition,
) -> BTreeMap<&str, BTreeSet<&str>> {
    beat_objectives
        .objectives
        .iter()
        .map(|objective| {
            (
                objective.id.as_str(),
                objective
                    .progress
                    .keys
                    .iter()
                    .map(|progress| progress.key.as_str())
                    .collect::<BTreeSet<_>>(),
            )
        })
        .collect()
}

fn validate_action_objective_progress(
    actions: &[ActionDefinition],
    stage_ids: &[&str],
    objective_progress_keys: &BTreeMap<&str, BTreeSet<&str>>,
) -> Result<(), Box<dyn Error>> {
    for action in actions {
        for stage_id in &action.available.available_during {
            require_known_id(
                stage_id,
                stage_ids,
                &format!(
                    "action '{}' available_during stage_id '{}'",
                    action.id, stage_id
                ),
                "beats.stages",
            )?;
        }
        validate_objective_progress_refs(
            &format!("action '{}' objective progress", action.id),
            action
                .available
                .required_objective_progress
                .iter()
                .chain(action.available.blocked_by_objective_progress.iter())
                .chain(action.sets_objective_progress.iter())
                .chain(action.clears_objective_progress.iter()),
            objective_progress_keys,
        )?;
    }
    Ok(())
}

fn validate_conditional_guidance_progress(
    beat_objectives: &BeatObjectivesDefinition,
    objective_progress_keys: &BTreeMap<&str, BTreeSet<&str>>,
) -> Result<(), Box<dyn Error>> {
    for objective in &beat_objectives.objectives {
        for conditional in &objective.guidance.conditional {
            validate_objective_progress_refs(
                &format!(
                    "beat_objectives.json objective '{}' conditional guidance objective progress",
                    objective.id
                ),
                conditional
                    .required_objective_progress
                    .iter()
                    .chain(conditional.blocked_by_objective_progress.iter()),
                objective_progress_keys,
            )?;
        }
    }
    Ok(())
}

fn validate_act_cast(act_cast: &[ActCastMember], actor_ids: &[&str]) -> Result<(), Box<dyn Error>> {
    if act_cast.is_empty() {
        return Ok(());
    }
    let mut seen_member_ids = BTreeSet::new();
    let mut seen_member_actor_ids = BTreeSet::new();
    for member in act_cast {
        if member.id.trim().is_empty() {
            return Err("act_cast member definition is missing id".into());
        }
        if !seen_member_ids.insert(member.id.clone()) {
            return Err(format!("duplicate act_cast member id '{}'", member.id).into());
        }
        if member.actor_id.trim().is_empty() {
            return Err(format!("act_cast member '{}' is missing actor_id", member.id).into());
        }
        require_known_id(
            &member.actor_id,
            actor_ids,
            &format!(
                "act_cast member '{}' actor_id '{}'",
                member.id, member.actor_id
            ),
            "actors",
        )?;
        if !seen_member_actor_ids.insert(member.actor_id.clone()) {
            return Err(format!(
                "act_cast members must not reuse actor_id '{}'",
                member.actor_id
            )
            .into());
        }
    }
    Ok(())
}

pub(crate) fn validate_objective_progress_refs<'a>(
    owner: &str,
    refs: impl IntoIterator<Item = &'a BeatObjectiveProgressRef>,
    objective_progress_keys: &BTreeMap<&str, BTreeSet<&str>>,
) -> Result<(), Box<dyn Error>> {
    for progress in refs {
        if progress.objective_id.trim().is_empty() || progress.key.trim().is_empty() {
            return Err(format!("{owner} refs require non-empty objective_id and key").into());
        }
        let Some(keys) = objective_progress_keys.get(progress.objective_id.as_str()) else {
            return Err(format!(
                "{owner} objective_id '{}' not found in beat_objectives",
                progress.objective_id
            )
            .into());
        };
        if !keys.contains(progress.key.as_str()) {
            return Err(format!(
                "{owner} key '{}' not found in rule objective '{}'",
                progress.key, progress.objective_id
            )
            .into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::types::ActorPromptContext;
    use std::collections::HashMap;

    fn actor(id: &str, room_id: &str) -> ActorDefinition {
        ActorDefinition {
            id: id.to_string(),
            name: id.to_string(),
            room_id: room_id.to_string(),
            level: 1,
            initial_stats: BTreeMap::new(),
            initial_pair_stats: BTreeMap::new(),
            aliases: Vec::new(),
            tags: Vec::new(),
            inspect_text: String::new(),
            required_consumable_tags: Vec::new(),
            attackable: false,
            guard: false,
            drops: BTreeMap::new(),
            xp_drop: 0,
            attack_interval_minutes: None,
            initial_hostile: false,
            initial_relationship: None,
            attack_kind: String::new(),
            resistances: BTreeMap::new(),
            prompt_context: ActorPromptContext {
                character_notes: Vec::new(),
                subtext_notes: Vec::new(),
                response_notes: Vec::new(),
                behavior_examples: Vec::new(),
            },
            act_cast: None,
            game_data: BTreeMap::new(),
        }
    }

    #[test]
    fn onstage_actor_requires_an_existing_room() {
        let room_index = HashMap::from([("lounge".to_string(), 0)]);
        let error = validate_actors(
            &[actor("alex", "missing")],
            &room_index,
            &[],
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("room_id 'missing' not found"), "{error}");
    }

    #[test]
    fn offstage_actor_skips_the_room_existence_check() {
        let room_index = HashMap::from([("lounge".to_string(), 0)]);
        let result = validate_actors(
            &[actor("handler", "")],
            &room_index,
            &[],
        );
        assert!(result.is_ok(), "{result:?}");
    }

    fn channel(id: &str, participants: &[&str]) -> MessagingChannel {
        MessagingChannel {
            id: id.to_string(),
            kind: crate::content::types::ChannelKind::Direct,
            privacy: ChannelPrivacy::Private,
            availability: crate::content::types::ChannelAvailability::Always,
            participants: participants.iter().map(|id| id.to_string()).collect(),
            label: None,
        }
    }

    #[test]
    fn private_channel_participants_must_resolve_to_actors() {
        let error = validate_channels(&[channel("comms", &["player", "nobody"])], &["player", "blair"])
            .unwrap_err()
            .to_string();
        assert!(error.contains("participant 'nobody'"), "{error}");
    }

    #[test]
    fn private_channel_must_declare_participants() {
        let error = validate_channels(&[channel("comms", &[])], &["player"])
            .unwrap_err()
            .to_string();
        assert!(error.contains("declares no participants"), "{error}");
    }

    #[test]
    fn a_pack_cannot_declare_the_implicit_local_channel() {
        let error =
            validate_channels(&[channel(LOCAL_CHANNEL_ID, &["player"])], &["player"])
                .unwrap_err()
                .to_string();
        assert!(error.contains("implicit local speech channel"), "{error}");
    }

    #[test]
    fn duplicate_channel_ids_are_rejected() {
        let error = validate_channels(
            &[channel("comms", &["player"]), channel("comms", &["blair"])],
            &["player", "blair"],
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("duplicate channel id 'comms'"), "{error}");
    }

    #[test]
    fn well_formed_private_channel_passes() {
        let result = validate_channels(&[channel("comms", &["player", "handler"])], &["player", "handler"]);
        assert!(result.is_ok(), "{result:?}");
    }
}
