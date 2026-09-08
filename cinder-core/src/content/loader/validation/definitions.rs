use super::require_known_id;
use crate::content::types::{
    ActionDefinition, CommandEffect, CommandTargetMode, ContentSettingsDefinition, ItemDefinition,
    MapDefinition, MapRevealCondition, PeriodicActorEffect, StatDefinition,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;

pub(crate) fn validate_maps(
    maps: &[MapDefinition],
    known_room_ids: &[&str],
    known_actor_ids: &[&str],
) -> Result<(), Box<dyn Error>> {
    let mut map_ids = BTreeSet::new();
    let mut mapped_room_ids = BTreeSet::new();
    for map in maps {
        if map.id.trim().is_empty() || !map_ids.insert(map.id.as_str()) {
            return Err(format!("map id '{}' is empty or duplicated", map.id).into());
        }
        if map.label.trim().is_empty() {
            return Err(format!("map '{}' must define a label", map.id).into());
        }
        if map.rooms.is_empty() {
            return Err(format!("map '{}' must contain at least one room", map.id).into());
        }
        let mut positions = BTreeSet::new();
        for room in &map.rooms {
            require_known_id(
                &room.room_id,
                known_room_ids,
                &format!("map '{}' room '{}'", map.id, room.room_id),
                "rooms",
            )?;
            if !mapped_room_ids.insert(room.room_id.as_str()) {
                return Err(format!("room '{}' appears in multiple maps", room.room_id).into());
            }
            if !positions.insert((room.x.to_bits(), room.y.to_bits())) {
                return Err(format!(
                    "map '{}' has multiple rooms at ({}, {})",
                    map.id, room.x, room.y
                )
                .into());
            }
        }
        for condition in &map.reveal_conditions {
            match condition {
                MapRevealCondition::ActorDefeated { actor_id } => {
                    require_known_id(
                        actor_id,
                        known_actor_ids,
                        &format!("map '{}' reveal actor '{}'", map.id, actor_id),
                        "actors",
                    )?;
                }
                MapRevealCondition::StoryVarTruthy { key } if key.trim().is_empty() => {
                    return Err(format!(
                        "map '{}' story_var_truthy reveal key must not be empty",
                        map.id
                    )
                    .into());
                }
                MapRevealCondition::StoryVarTruthy { .. } => {}
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_actions(
    actions: &[ActionDefinition],
    known_room_ids: &[&str],
    known_stage_ids: &[&str],
) -> Result<(), Box<dyn Error>> {
    for action in actions {
        if action.player_enabled && !action.phrases.is_empty() {
            let metadata = action.player_command.as_ref().ok_or_else(|| {
                format!(
                    "player-enabled action '{}' with phrases must define player_command metadata",
                    action.id
                )
            })?;
            if metadata.usage.trim().is_empty() {
                return Err(format!(
                    "player-enabled action '{}' must define player_command.usage",
                    action.id
                )
                .into());
            }
            if metadata.example.trim().is_empty() {
                return Err(format!(
                    "player-enabled action '{}' must define player_command.example",
                    action.id
                )
                .into());
            }
        }
        if action.npc.is_some() && action.command.trim().is_empty() {
            return Err(format!("NPC action '{}' must define a command field", action.id).into());
        }
        for room_id in &action.available.allowed_rooms {
            require_known_id(
                room_id,
                known_room_ids,
                &format!("action '{}' allowed_room '{}'", action.id, room_id),
                "rooms",
            )?;
        }
        for stage_id in &action.available.available_during {
            require_known_id(
                stage_id,
                known_stage_ids,
                &format!("action '{}' available_during '{}'", action.id, stage_id),
                "beats.stages",
            )?;
        }
        let has_move_effect = action.has_effect(CommandEffect::MoveActor);
        let has_observe_room_effect = action.has_effect(CommandEffect::ObserveRoom);
        if has_move_effect && action.target_mode != CommandTargetMode::Room {
            return Err(format!("move action '{}' must use target_mode room", action.id).into());
        }
        if has_observe_room_effect && action.target_mode != CommandTargetMode::None {
            return Err(format!(
                "observe-room action '{}' must use target_mode none",
                action.id
            )
            .into());
        }
        if let Some(item_creation) = &action.item_creation {
            let template = item_creation.creates_item_target_template.trim();
            if !template.is_empty() {
                if action.target_mode != CommandTargetMode::Actor {
                    return Err(format!(
                        "action '{}' with item_creation.creates_item_target_template must use \
                         target_mode actor",
                        action.id
                    )
                    .into());
                }
                if !template.contains("{target_actor_id}") {
                    return Err(format!(
                        "action '{}' item_creation.creates_item_target_template must contain \
                         '{{target_actor_id}}'",
                        action.id
                    )
                    .into());
                }
                let remainder = template.replace("{target_actor_id}", "");
                if remainder.contains('{') || remainder.contains('}') {
                    return Err(format!(
                        "action '{}' item_creation.creates_item_target_template contains an \
                         unsupported placeholder",
                        action.id
                    )
                    .into());
                }
            }
        }
    }
    Ok(())
}

/// Equipment references resolve against the pack: slots must be declared in
/// `settings.equipment_slots`, bonus keys must be declared stats, and use
/// hooks must exist in hooks.json.
pub(crate) fn validate_items(
    items: &[ItemDefinition],
    settings: &ContentSettingsDefinition,
    known_stats: &BTreeMap<String, StatDefinition>,
    known_hooks: &BTreeMap<String, Value>,
) -> Result<(), Box<dyn Error>> {
    for item in items {
        if item.equip_slot.trim().is_empty() {
            continue;
        }

        if !settings.equipment_slots.contains(&item.equip_slot) {
            return Err(format!(
                "item '{}' equip_slot '{}' not declared in settings.equipment_slots",
                item.id, item.equip_slot
            )
            .into());
        }
        for stat_id in item.stat_bonuses.keys() {
            if !known_stats.contains_key(stat_id) {
                return Err(format!(
                    "item '{}' stat_bonuses key '{}' not declared in stats.actor",
                    item.id, stat_id
                )
                .into());
            }
        }
        for (field, hook_id) in [
            ("use_hook", &item.use_hook),
            ("equip_hook", &item.equip_hook),
        ] {
            if !hook_id.trim().is_empty() && !known_hooks.contains_key(hook_id.as_str()) {
                return Err(format!(
                    "item '{}' {field} '{hook_id}' not found in hooks.json",
                    item.id
                )
                .into());
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_combat_settings(
    settings: &ContentSettingsDefinition,
) -> Result<(), Box<dyn Error>> {
    let ally_attack = &settings.combat.ally_attack;
    if ally_attack.contribution_percent > 1_000 {
        return Err("combat.ally_attack.contribution_percent must be at most 1000".into());
    }
    if ally_attack
        .maximum_per_ally
        .is_some_and(|maximum| maximum < 0)
    {
        return Err("combat.ally_attack.maximum_per_ally must not be negative".into());
    }
    Ok(())
}

pub(crate) fn validate_periodic_actor_effects(
    settings: &ContentSettingsDefinition,
    items: &[ItemDefinition],
    messages: &BTreeMap<String, String>,
) -> Result<(), Box<dyn Error>> {
    let item_ids = items
        .iter()
        .map(|item| item.id.as_str())
        .collect::<Vec<_>>();
    let mut effect_ids = BTreeSet::new();
    for (index, definition) in settings.periodic_actor_effects.iter().enumerate() {
        let id = definition.id.trim();
        if id.is_empty() {
            return Err(format!("periodic_actor_effects[{index}].id must not be empty").into());
        }
        if !effect_ids.insert(id) {
            return Err(format!("periodic_actor_effects id '{id}' is duplicated").into());
        }
        let room_item = definition.trigger.room_item.trim();
        if room_item.is_empty() {
            return Err(format!(
                "periodic_actor_effects '{id}' trigger.room_item must not be empty"
            )
            .into());
        }
        require_known_id(
            room_item,
            &item_ids,
            &format!("periodic_actor_effects '{id}' trigger.room_item '{room_item}'"),
            "items",
        )?;
        match definition.effect {
            PeriodicActorEffect::Damage { amount } if amount <= 0 => {
                return Err(format!(
                    "periodic_actor_effects '{id}' damage amount must be positive"
                )
                .into());
            }
            PeriodicActorEffect::Damage { .. } => {}
        }
        let message = definition.message.trim();
        if message.is_empty() {
            return Err(format!("periodic_actor_effects '{id}' message must not be empty").into());
        }
        if !messages.contains_key(message) {
            return Err(format!(
                "periodic_actor_effects '{id}' message '{message}' not found in locale messages"
            )
            .into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::types::{
        MapRoomDefinition, PeriodicActorEffectDefinition, PeriodicActorEffectTargets,
        PeriodicActorEffectTrigger,
    };

    fn valid_fixture() -> (
        ContentSettingsDefinition,
        Vec<ItemDefinition>,
        BTreeMap<String, String>,
    ) {
        let settings = ContentSettingsDefinition {
            periodic_actor_effects: vec![PeriodicActorEffectDefinition {
                id: "hazard".to_string(),
                trigger: PeriodicActorEffectTrigger {
                    room_item: "hazard-token".to_string(),
                    ..PeriodicActorEffectTrigger::default()
                },
                targets: PeriodicActorEffectTargets::HostileLiving,
                effect: PeriodicActorEffect::Damage { amount: 2 },
                message: "combat.hazard".to_string(),
            }],
            ..ContentSettingsDefinition::default()
        };
        let items = vec![ItemDefinition {
            id: "hazard-token".to_string(),
            ..ItemDefinition::default()
        }];
        let messages = BTreeMap::from([("combat.hazard".to_string(), "Ouch.".to_string())]);
        (settings, items, messages)
    }

    #[test]
    fn rejects_empty_map_story_var_reveal_key() {
        let maps = vec![MapDefinition {
            id: "floor".to_string(),
            label: "Floor".to_string(),
            rooms: vec![MapRoomDefinition {
                room_id: "room".to_string(),
                x: 0.0,
                y: 0.0,
            }],
            reveal_conditions: vec![MapRevealCondition::StoryVarTruthy {
                key: " ".to_string(),
            }],
        }];

        let error = validate_maps(&maps, &["room"], &[])
            .unwrap_err()
            .to_string();

        assert!(error.contains("reveal key must not be empty"), "{error}");
    }

    #[test]
    fn accepts_valid_periodic_actor_effects() {
        let (settings, items, messages) = valid_fixture();
        validate_periodic_actor_effects(&settings, &items, &messages).unwrap();
    }

    #[test]
    fn rejects_empty_and_duplicate_periodic_effect_ids() {
        let (mut settings, items, messages) = valid_fixture();
        settings.periodic_actor_effects[0].id = " ".to_string();
        let error = validate_periodic_actor_effects(&settings, &items, &messages)
            .unwrap_err()
            .to_string();
        assert!(error.contains("id must not be empty"), "{error}");

        let (mut settings, items, messages) = valid_fixture();
        settings
            .periodic_actor_effects
            .push(settings.periodic_actor_effects[0].clone());
        let error = validate_periodic_actor_effects(&settings, &items, &messages)
            .unwrap_err()
            .to_string();
        assert!(error.contains("id 'hazard' is duplicated"), "{error}");
    }

    #[test]
    fn rejects_unknown_room_item_and_non_positive_damage() {
        let (mut settings, items, messages) = valid_fixture();
        settings.periodic_actor_effects[0].trigger.room_item = "missing".to_string();
        let error = validate_periodic_actor_effects(&settings, &items, &messages)
            .unwrap_err()
            .to_string();
        assert!(error.contains("'missing' not found in items"), "{error}");

        let (mut settings, items, messages) = valid_fixture();
        settings.periodic_actor_effects[0].effect = PeriodicActorEffect::Damage { amount: 0 };
        let error = validate_periodic_actor_effects(&settings, &items, &messages)
            .unwrap_err()
            .to_string();
        assert!(error.contains("damage amount must be positive"), "{error}");
    }

    #[test]
    fn rejects_empty_or_unknown_periodic_effect_message() {
        let (mut settings, items, messages) = valid_fixture();
        settings.periodic_actor_effects[0].message = String::new();
        let error = validate_periodic_actor_effects(&settings, &items, &messages)
            .unwrap_err()
            .to_string();
        assert!(error.contains("message must not be empty"), "{error}");

        let (mut settings, items, messages) = valid_fixture();
        settings.periodic_actor_effects[0].message = "combat.missing".to_string();
        let error = validate_periodic_actor_effects(&settings, &items, &messages)
            .unwrap_err()
            .to_string();
        assert!(error.contains("not found in locale messages"), "{error}");
    }
}
