mod bundled;
mod fs;
mod index;

use crate::content::loader::bundled::{read_messages, read_system_text};
use crate::content::loader::fs::{
    LocalizedPaths, read_json, read_optional_json, read_optional_json_raw,
};
use crate::content::loader::index::{build_index, collect_act_cast};
use crate::content::types::{
    ActionsDefinition, ActorDefinition, BeatObjectivesDefinition, BeatsDefinition,
    BehaviorDefinition, ContentPack, ContentSettingsDefinition, ItemDefinition, LevelingDefinition,
    MapDefinition, MovementConfigDefinition, OpeningDefinition, OpeningMenuDefinition,
    OpeningMovieDefinition, PresentationDefinition, RoomDefinition, SequencesDefinition,
    SkillsDefinition, SpeechConfigDefinition, SpeechIntentsConfig, StatsDefinition,
    TeleportNetworkDefinition, UiTextDefinition,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs as std_fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_LOCALE: &str = "en";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocaleOption {
    pub code: String,
    pub label: String,
}

pub fn load_named_pack(pack_id: &str, locale: Option<&str>) -> Result<ContentPack, Box<dyn Error>> {
    load_pack_from_dir_with_locale(&pack_dir(pack_id), locale)
}

pub fn content_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("CINDER_CONTENT_DIR") {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    crate::project_dir().join("content")
}

pub fn pack_dir(pack_id: &str) -> PathBuf {
    content_dir().join(pack_id)
}

pub fn available_packs() -> Vec<String> {
    let dir = content_dir();
    let mut packs: Vec<String> = Vec::new();
    if let Ok(entries) = std_fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let settings_path = path.join("settings.json");
                if settings_path.exists()
                    && let Some(name) = path.file_name().and_then(|n| n.to_str())
                {
                    packs.push(name.to_string());
                }
            }
        }
    }
    packs.sort();
    packs
}

pub fn load_pack_settings(pack_id: &str) -> Result<ContentSettingsDefinition, Box<dyn Error>> {
    Ok(
        read_optional_json::<ContentSettingsDefinition>(&pack_dir(pack_id), "settings.json")?
            .unwrap_or_default(),
    )
}

pub fn load_pack_from_dir(path: &Path) -> Result<ContentPack, Box<dyn Error>> {
    load_pack_from_dir_with_locale(path, None)
}

pub fn load_pack_from_dir_with_locale(
    path: &Path,
    locale: Option<&str>,
) -> Result<ContentPack, Box<dyn Error>> {
    let settings =
        read_optional_json::<ContentSettingsDefinition>(path, "settings.json")?.unwrap_or_default();
    let effective_locale = match locale {
        Some(locale) if !locale.trim().is_empty() => locale.to_string(),
        _ if !settings.default_language.trim().is_empty() => settings.default_language.clone(),
        _ => DEFAULT_LOCALE.to_string(),
    };
    let paths = LocalizedPaths::new(path, &effective_locale);
    let ui_text = paths
        .read_optional::<UiTextDefinition>("ui.json")?
        .unwrap_or_default();
    let system_text = read_system_text(&paths)?;
    let opening = paths.read_required::<OpeningDefinition>("opening.json")?;
    let beats = paths
        .read_optional::<BeatsDefinition>("beats.json")?
        .unwrap_or_default();
    let sequences = paths
        .read_optional::<SequencesDefinition>("sequences.json")?
        .unwrap_or_default();
    let menus = paths
        .read_optional::<Vec<OpeningMenuDefinition>>("menus.json")?
        .unwrap_or_default();
    let mut movies = paths
        .read_optional::<Vec<OpeningMovieDefinition>>("movies.json")?
        .unwrap_or_default();
    let presentation = paths
        .read_optional::<PresentationDefinition>("presentation.json")?
        .unwrap_or_default();
    let maps = paths
        .read_optional::<Vec<MapDefinition>>("maps.json")?
        .unwrap_or_default();
    let messages = read_messages(&paths)?;
    for movie in &mut movies {
        for frame in &mut movie.frames {
            if !frame.text_path.is_empty() {
                frame.text = std_fs::read_to_string(path.join(&frame.text_path))?;
            }
        }
    }
    let rooms = paths.read_required::<Vec<RoomDefinition>>("rooms.json")?;
    let actors = paths.read_required::<Vec<ActorDefinition>>("actors.json")?;
    let act_cast = collect_act_cast(&actors);
    let stats = read_optional_json::<StatsDefinition>(path, "stats.json")?.unwrap_or_default();

    let actions = if let Some(actions_json) = read_optional_json_raw(path, "actions.json")? {
        let actions_def: ActionsDefinition =
            serde_json::from_str(&actions_json).map_err(|e| format!("actions.json: {e}"))?;
        actions_def.actions
    } else {
        Vec::new()
    };
    let movement =
        read_optional_json::<MovementConfigDefinition>(path, "movement.json")?.unwrap_or_default();
    let behavior =
        read_optional_json::<BehaviorDefinition>(path, "behavior.json")?.unwrap_or_default();
    let speech =
        read_optional_json::<SpeechConfigDefinition>(path, "speech.json")?.unwrap_or_default();
    let teleports = paths
        .read_optional::<TeleportNetworkDefinition>("teleports.json")?
        .unwrap_or_default();
    let beat_objectives = read_json::<BeatObjectivesDefinition>(path, "beat_objectives.json")?;
    let hooks =
        read_optional_json::<BTreeMap<String, Value>>(path, "hooks.json")?.unwrap_or_default();
    let speech_intents: SpeechIntentsConfig =
        read_optional_json::<SpeechIntentsConfig>(path, "intents.json")?.unwrap_or_default();
    let items: Vec<ItemDefinition> =
        read_optional_json::<Vec<ItemDefinition>>(path, "items.json")?.unwrap_or_default();
    let variables: BTreeMap<String, crate::engine::state::VariableDeclaration> =
        read_optional_json::<BTreeMap<String, crate::engine::state::VariableDeclaration>>(
            path,
            "variables.json",
        )?
        .unwrap_or_default();
    let levels = read_optional_json::<LevelingDefinition>(path, "levels.json")?.unwrap_or_default();
    let skills = read_optional_json::<SkillsDefinition>(path, "skills.json")?.unwrap_or_default();

    let room_index = build_index(&rooms, |room| &room.id);
    let actor_index = build_index(&actors, |actor| &actor.id);
    let action_index = build_index(&actions, |action| &action.id);
    let skill_index = build_index(&skills.skills, |skill| &skill.id);

    let pack = ContentPack {
        locale: effective_locale,
        settings,
        ui_text,
        system_text,
        opening,
        beats,
        sequences,
        menus,
        movies,
        presentation,
        maps,
        rooms,
        actors,
        act_cast,
        stats,
        actions,
        movement,
        behavior,
        speech,
        teleports,
        beat_objectives,
        hooks,
        speech_intents,
        items,
        variables,
        levels,
        messages,
        skills,
        room_index,
        actor_index,
        action_index,
        skill_index,
    };
    validate_skills(&pack)?;
    Ok(pack)
}

fn validate_skills(pack: &ContentPack) -> Result<(), Box<dyn Error>> {
    let mut declared = BTreeSet::new();
    for skill in &pack.skills.skills {
        if skill.id.trim().is_empty() {
            return Err("skills.json contains a skill with an empty id".into());
        }
        if !declared.insert(skill.id.as_str()) {
            return Err(format!("skills.json declares duplicate skill '{}'", skill.id).into());
        }
    }

    for actor in &pack.actors {
        let mut assigned = BTreeSet::new();
        for assignment in &actor.skills {
            let skill_id = assignment.id();
            let Some(skill) = pack.skill(skill_id) else {
                return Err(
                    format!("actor '{}' declares unknown skill '{skill_id}'", actor.id).into(),
                );
            };
            if !assigned.insert(skill_id) {
                return Err(format!(
                    "actor '{}' declares skill '{skill_id}' more than once",
                    actor.id
                )
                .into());
            }
            if pack.skills.strict
                && skill.kind == Some(crate::content::types::SkillKind::Heal)
                && assignment.power().is_none()
            {
                return Err(format!(
                    "strict skills require actor '{}' to configure power for heal skill '{}'",
                    actor.id, skill_id
                )
                .into());
            }
        }
    }

    for action in &pack.actions {
        if !action.skill_id.is_empty() && pack.skill(&action.skill_id).is_none() {
            return Err(format!(
                "action '{}' references unknown skill '{}'",
                action.id, action.skill_id
            )
            .into());
        }
    }
    for rule in &pack.settings.party.combat_rules {
        if !rule.skill_id.is_empty() && pack.skill(&rule.skill_id).is_none() {
            return Err(format!(
                "party combat rule '{}' references unknown skill '{}'",
                rule.id, rule.skill_id
            )
            .into());
        }
    }
    for level in pack
        .levels
        .default
        .iter()
        .chain(pack.levels.actors.values().flatten())
    {
        for skill_id in &level.unlocks {
            if pack.skill(skill_id).is_none() {
                return Err(format!("levels.json unlocks unknown skill '{skill_id}'").into());
            }
        }
    }
    for (hook_id, hook) in &pack.hooks {
        validate_hook_skill_grants(pack, hook_id, hook)?;
    }

    if !pack.skills.strict {
        return Ok(());
    }

    for action in &pack.actions {
        if matches!(action.command.as_str(), "ATTACK" | "TRACE" | "TELEPORT")
            && action.skill_id.is_empty()
        {
            return Err(format!(
                "strict skills require action '{}' to name a skill",
                action.id
            )
            .into());
        }
    }
    for rule in &pack.settings.party.combat_rules {
        if rule.skill_id.is_empty() {
            return Err(format!(
                "strict skills require party combat rule '{}' to name a skill",
                rule.id
            )
            .into());
        }
    }
    for actor in &pack.actors {
        let behavior = pack
            .behavior
            .actors
            .get(&actor.id)
            .cloned()
            .unwrap_or_default()
            .resolved_with_default(&pack.behavior.defaults);
        if behavior.strike.is_some() && behavior.strike_skill_id.is_empty() {
            return Err(format!(
                "strict skills require actor '{}' strike behavior to name a skill",
                actor.id
            )
            .into());
        }
        if (actor.attackable || actor.initial_hostile)
            && behavior.strike.is_some()
            && actor.skill(&behavior.strike_skill_id).is_none()
        {
            return Err(format!(
                "combat actor '{}' lacks required strike skill '{}'",
                actor.id, behavior.strike_skill_id
            )
            .into());
        }
    }
    Ok(())
}

fn validate_hook_skill_grants(
    pack: &ContentPack,
    hook_id: &str,
    value: &Value,
) -> Result<(), Box<dyn Error>> {
    match value {
        Value::Array(values) => {
            for value in values {
                validate_hook_skill_grants(pack, hook_id, value)?;
            }
        }
        Value::Object(object) => {
            if object.get("kind").and_then(Value::as_str) == Some("grant_skill") {
                let skill_id = object
                    .get("skill_id")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if pack.skill(skill_id).is_none() {
                    return Err(
                        format!("hook '{hook_id}' grants unknown skill '{skill_id}'").into(),
                    );
                }
                if let Some(actor_id) = object.get("actor_id").and_then(Value::as_str)
                    && pack.actor(actor_id).is_none()
                {
                    return Err(format!(
                        "hook '{hook_id}' grants a skill to unknown actor '{actor_id}'"
                    )
                    .into());
                }
            }
            for value in object.values() {
                validate_hook_skill_grants(pack, hook_id, value)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub fn available_locales(path: &Path) -> Result<Vec<LocaleOption>, Box<dyn Error>> {
    let locales_dir = path.join("locales");
    let mut locales = Vec::new();
    if locales_dir.exists() {
        for entry in std_fs::read_dir(&locales_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let code = entry.file_name().to_string_lossy().to_string();
            let ui_path = entry.path().join("ui.json");
            let label = match std_fs::read_to_string(ui_path) {
                Ok(contents) => serde_json::from_str::<UiTextDefinition>(&contents)
                    .map(|ui_text| ui_text.language_name)
                    .unwrap_or_else(|_| code.clone()),
                Err(_) => code.clone(),
            };
            locales.push(LocaleOption { code, label });
        }
    }
    locales.sort_by(
        |left, right| match (left.code.as_str(), right.code.as_str()) {
            (DEFAULT_LOCALE, DEFAULT_LOCALE) => std::cmp::Ordering::Equal,
            (DEFAULT_LOCALE, _) => std::cmp::Ordering::Less,
            (_, DEFAULT_LOCALE) => std::cmp::Ordering::Greater,
            _ => left.code.cmp(&right.code),
        },
    );
    if locales.is_empty() {
        locales.push(LocaleOption {
            code: DEFAULT_LOCALE.to_string(),
            label: UiTextDefinition::default().language_name,
        });
    }
    Ok(locales)
}

#[cfg(test)]
mod shipped_pack_load_tests {
    use super::*;

    #[test]
    fn shipped_packs_load_behavior_and_movement() {
        for pack in ["aera", "ella", "isla", "layla"] {
            let dir = pack_dir(pack);
            let loaded = load_pack_from_dir_with_locale(&dir, Some("en"))
                .unwrap_or_else(|e| panic!("pack {pack} failed to load: {e}"));
            assert!(
                loaded.behavior.defaults.strike.is_some(),
                "{pack}: strike default absent"
            );
            assert!(
                loaded.behavior.defaults.hold.is_some(),
                "{pack}: hold default absent"
            );
            assert!(!loaded.rooms.is_empty(), "{pack}: rooms absent");
            assert!(!loaded.actors.is_empty(), "{pack}: actors absent");
        }
    }
}
