mod bundled;
mod fs;
mod index;
mod validation;

use crate::content::loader::bundled::{read_messages, read_system_text};
use crate::content::loader::fs::{
    read_json, read_optional_json, read_optional_json_raw, LocalizedPaths,
};
use crate::content::loader::index::{build_index, collect_act_cast};
use crate::content::loader::validation::{
    PackContext, require_known_id, validate_actions, validate_combat_settings, validate_contents,
    validate_items, validate_periodic_actor_effects,
};
use crate::content::types::{
    ActionsDefinition, ActorDefinition, BehaviorDefinition, BeatObjectivesDefinition,
    BeatsDefinition, ContentPack, ContentSettingsDefinition, ItemDefinition, LevelingDefinition,
    MovementConfigDefinition, OpeningDefinition, OpeningMenuDefinition, OpeningMovieDefinition,
    PresentationDefinition, RoomDefinition, SpeechConfigDefinition, SpeechIntentsConfig,
    StatsDefinition, UiTextDefinition,
};
use serde_json::Value;
use std::collections::BTreeMap;
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
    PathBuf::from(env!("CINDER_PROJECT_DIR")).join("content")
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
    let settings = read_optional_json::<ContentSettingsDefinition>(path, "settings.json")?
        .unwrap_or_default();
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
    let menus = paths
        .read_optional::<Vec<OpeningMenuDefinition>>("menus.json")?
        .unwrap_or_default();
    let mut movies = paths
        .read_optional::<Vec<OpeningMovieDefinition>>("movies.json")?
        .unwrap_or_default();
    let presentation = paths
        .read_optional::<PresentationDefinition>("presentation.json")?
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
    let beat_objectives = read_json::<BeatObjectivesDefinition>(path, "beat_objectives.json")?;
    let hooks =
        read_optional_json::<BTreeMap<String, Value>>(path, "hooks.json")?.unwrap_or_default();
    let speech_intents: SpeechIntentsConfig =
        read_optional_json::<SpeechIntentsConfig>(path, "intents.json")?.unwrap_or_default();
    let items: Vec<ItemDefinition> =
        read_optional_json::<Vec<ItemDefinition>>(path, "items.json")?.unwrap_or_default();
    let item_ids = items
        .iter()
        .map(|item| item.id.as_str())
        .collect::<Vec<_>>();
    for item_id in settings.starting_items.keys() {
        require_known_id(
            item_id,
            &item_ids,
            &format!("starting_items '{item_id}'"),
            "items",
        )?;
    }
    validate_combat_settings(&settings)?;
    validate_periodic_actor_effects(&settings, &items, &messages)?;
    validate_items(&items, &settings, &stats.actor, &hooks)?;
    let variables: BTreeMap<String, crate::engine::state::VariableDeclaration> =
        read_optional_json::<BTreeMap<String, crate::engine::state::VariableDeclaration>>(
            path,
            "variables.json",
        )?
        .unwrap_or_default();
    let levels = read_optional_json::<LevelingDefinition>(path, "levels.json")?.unwrap_or_default();

    let room_index = build_index(&rooms, |room| &room.id);
    let actor_index = build_index(&actors, |actor| &actor.id);
    let action_index = build_index(&actions, |action| &action.id);
    let room_ids = rooms
        .iter()
        .map(|room| room.id.as_str())
        .collect::<Vec<_>>();
    let actor_ids = actors
        .iter()
        .map(|actor| actor.id.as_str())
        .collect::<Vec<_>>();

    let stage_ids: Vec<&str> = beats.stages.iter().map(|s| s.id.as_str()).collect();
    validate_actions(&actions, &room_ids, &stage_ids)?;
    validate_contents(&PackContext {
        levels: &levels,
        beats: &beats,
        actors: &actors,
        movement: &movement,
        actions: &actions,
        beat_objectives: &beat_objectives,
        act_cast: &act_cast,
        actor_ids: &actor_ids,
        room_ids: &room_ids,
        stage_ids: &stage_ids,
        item_ids: &item_ids,
        room_index: &room_index,
        action_index: &action_index,
    })?;

    Ok(ContentPack {
        locale: effective_locale,
        settings,
        ui_text,
        system_text,
        opening,
        beats,
        menus,
        movies,
        presentation,
        rooms,
        actors,
        act_cast,
        stats,
        actions,
        movement,
        behavior,
        speech,
        beat_objectives,
        hooks,
        speech_intents,
        items,
        variables,
        levels,
        messages,
        room_index,
        actor_index,
        action_index,
    })
}

pub fn available_locales(path: &Path) -> Result<Vec<LocaleOption>, Box<dyn Error>> {    let locales_dir = path.join("locales");
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
            if pack == "layla" {
                assert_eq!(loaded.settings.periodic_actor_effects.len(), 1);
                assert_eq!(loaded.settings.periodic_actor_effects[0].id, "drain_sigil");
                assert_eq!(
                    loaded.actor("fire-elemental").unwrap().drops,
                    BTreeMap::from([("spawn-scroll".to_string(), 1)])
                );
                assert_eq!(
                    loaded
                        .item("spawn-scroll")
                        .map(|item| item.use_hook.as_str()),
                    Some("item.spawn_scroll_read")
                );
                assert_eq!(
                    loaded
                        .action("trace")
                        .and_then(|action| action.item_creation.as_ref())
                        .and_then(|creation| creation.craftable_item_gates.get("spawn-sigil"))
                        .map(String::as_str),
                    Some("knows_spawn")
                );
            }
        }
    }
}
