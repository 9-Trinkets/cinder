mod bundled;
mod fs;
mod index;
mod validation;

use crate::content::loader::bundled::{read_messages, read_system_text};
use crate::content::loader::fs::{
    LocalizedPaths, read_json, read_optional_json, read_optional_json_raw,
};
use crate::content::loader::index::{build_index, collect_act_cast};
use crate::content::types::{
    ActionsDefinition, ActorDefinition, BeatObjectivesDefinition, BeatsDefinition,
    BehaviorDefinition, ContentPack, ContentSettingsDefinition, ItemDefinition, LevelingDefinition,
    MapDefinition, MovementConfigDefinition, OpeningDefinition, OpeningMenuDefinition,
    OpeningMovieDefinition, PartyDefinition, PresentationDefinition, RoomDefinition,
    SequencesDefinition, SkillsDefinition, SpeechConfigDefinition, SpeechIntentsConfig,
    StatsDefinition, TeleportNetworkDefinition, UiTextDefinition,
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
    let mut settings =
        read_optional_json::<ContentSettingsDefinition>(&pack_dir(pack_id), "settings.json")?
            .unwrap_or_default();
    if let Some(parties) =
        read_optional_json::<Vec<PartyDefinition>>(&pack_dir(pack_id), "parties.json")?
    {
        settings.parties = parties;
    }
    Ok(settings)
}

pub fn load_pack_from_dir(path: &Path) -> Result<ContentPack, Box<dyn Error>> {
    load_pack_from_dir_with_locale(path, None)
}

pub fn load_pack_from_dir_with_locale(
    path: &Path,
    locale: Option<&str>,
) -> Result<ContentPack, Box<dyn Error>> {
    let mut settings =
        read_optional_json::<ContentSettingsDefinition>(path, "settings.json")?.unwrap_or_default();
    if let Some(parties) = read_optional_json::<Vec<PartyDefinition>>(path, "parties.json")? {
        settings.parties = parties;
    }
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

    let mut actions = if let Some(actions_json) = read_optional_json_raw(path, "actions.json")? {
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
    install_skill_behaviors(&skills, &mut actions, &mut settings)?;

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
    validation::validate(&pack)?;
    Ok(pack)
}

fn install_skill_behaviors(
    skills: &SkillsDefinition,
    actions: &mut Vec<crate::content::types::ActionDefinition>,
    settings: &mut ContentSettingsDefinition,
) -> Result<(), Box<dyn Error>> {
    if let Some(action) = actions.iter().find(|action| !action.skill_id.is_empty()) {
        return Err(format!(
            "skills require skill action '{}' to be authored in skills.json",
            action.id
        )
        .into());
    }
    if let Some(rule) = settings
        .parties
        .iter()
        .filter(|party| party.controller == crate::content::types::PartyController::Player)
        .flat_map(|party| &party.combat_rules)
        .next()
    {
        return Err(format!(
            "skills require player party reaction '{}' to be authored in skills.json",
            rule.id
        )
        .into());
    }

    let mut reactions = Vec::new();
    let mut reaction_ids = settings
        .parties
        .iter()
        .flat_map(|party| &party.combat_rules)
        .map(|rule| rule.id.clone())
        .collect::<BTreeSet<_>>();
    for (skill_index, skill) in skills.skills.iter().enumerate() {
        if let Some(mut action) = skill.player_action.clone() {
            if !action.skill_id.is_empty() && action.skill_id != skill.id {
                return Err(format!(
                    "skill '{}' player action '{}' binds a different skill '{}'",
                    skill.id, action.id, action.skill_id
                )
                .into());
            }
            if actions.iter().any(|existing| existing.id == action.id) {
                return Err(format!(
                    "skill '{}' player action duplicates action id '{}'",
                    skill.id, action.id
                )
                .into());
            }
            action.skill_id = skill.id.clone();
            actions.push(action);
        }
        for (rule_index, authored_rule) in skill.reactions.iter().enumerate() {
            if !authored_rule.skill_id.is_empty() && authored_rule.skill_id != skill.id {
                return Err(format!(
                    "skill '{}' reaction '{}' binds a different skill '{}'",
                    skill.id, authored_rule.id, authored_rule.skill_id
                )
                .into());
            }
            if !reaction_ids.insert(authored_rule.id.clone()) {
                return Err(format!(
                    "skill '{}' reaction duplicates rule id '{}'",
                    skill.id, authored_rule.id
                )
                .into());
            }
            let mut rule = authored_rule.clone();
            rule.skill_id = skill.id.clone();
            reactions.push((skill.reaction_priority, skill_index, rule_index, rule));
        }
    }
    reactions.sort_by_key(|(priority, skill_index, rule_index, _)| {
        (*priority, *skill_index, *rule_index)
    });
    settings
        .ensure_player_party()
        .combat_rules
        .extend(reactions.into_iter().map(|(_, _, _, rule)| rule));
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
                loaded.behavior.defaults.hold.is_some(),
                "{pack}: hold default absent"
            );
            assert!(!loaded.rooms.is_empty(), "{pack}: rooms absent");
            assert!(!loaded.actors.is_empty(), "{pack}: actors absent");
        }
    }
}
