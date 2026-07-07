use crate::content::types::{
    ActorDefinition, AffordancesDefinition, AppointmentPatientDefinition, BeatsDefinition,
    CommandsDefinition, ContentPack, ContentSettingsDefinition, ItemDefinition, OpeningDefinition,
    OpeningMenuDefinition, OpeningMovieDefinition, PresentationDefinition, RoomDefinition,
    SpeechIntentsConfig, StatsDefinition, SystemTextDefinition, UiTextDefinition,
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_LOCALE: &str = "en";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocaleOption {
    pub code: String,
    pub label: String,
}

pub fn load_default_pack() -> Result<ContentPack, Box<dyn Error>> {
    load_default_pack_with_locale(None)
}

pub fn load_default_pack_with_locale(locale: Option<&str>) -> Result<ContentPack, Box<dyn Error>> {
    load_pack_from_dir_with_locale(&default_pack_dir(), locale)
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

pub fn default_pack_dir() -> PathBuf {
    pack_dir("ella")
}

pub fn load_pack_from_dir(path: &Path) -> Result<ContentPack, Box<dyn Error>> {
    load_pack_from_dir_with_locale(path, None)
}

pub fn load_pack_from_dir_with_locale(
    path: &Path,
    locale: Option<&str>,
) -> Result<ContentPack, Box<dyn Error>> {
    let settings_path = path.join("settings.json");
    let settings = match fs::read_to_string(&settings_path) {
        Ok(contents) => serde_json::from_str(&contents)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            ContentSettingsDefinition::default()
        }
        Err(error) => return Err(error.into()),
    };
    let effective_locale = match locale {
        Some(locale) if !locale.trim().is_empty() => locale.to_string(),
        _ if !settings.default_language.trim().is_empty() => settings.default_language.clone(),
        _ => DEFAULT_LOCALE.to_string(),
    };
    let ui_text_path = localized_file_path(path, &effective_locale, "ui.json");
    let ui_text = match fs::read_to_string(&ui_text_path) {
        Ok(contents) => serde_json::from_str(&contents)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => UiTextDefinition::default(),
        Err(error) => return Err(error.into()),
    };
    let system_text_path = localized_file_path(path, &effective_locale, "system.json");
    let system_text: SystemTextDefinition =
        serde_json::from_str(&fs::read_to_string(&system_text_path)?)?;
    let opening_path = localized_file_path_with_fallback(
        path,
        &effective_locale,
        "opening.json",
        Some("scenario.json"),
    );
    let opening: OpeningDefinition = serde_json::from_str(&fs::read_to_string(&opening_path)?)?;
    let beats: BeatsDefinition = read_optional_localized_json_with_fallback(
        path,
        &effective_locale,
        "beats.json",
        Some("objective_flow.json"),
    )?
    .unwrap_or_default();
    let menus = read_optional_localized_json::<Vec<OpeningMenuDefinition>>(
        path,
        &effective_locale,
        "menus.json",
    )?
    .unwrap_or_default();
    let mut movies = read_optional_localized_json_with_fallback::<Vec<OpeningMovieDefinition>>(
        path,
        &effective_locale,
        "movies.json",
        Some("projector_sequences.json"),
    )?
    .unwrap_or_default();
    let presentation = read_optional_localized_json::<PresentationDefinition>(
        path,
        &effective_locale,
        "presentation.json",
    )?
    .unwrap_or_default();
    for movie in &mut movies {
        for frame in &mut movie.frames {
            if !frame.text_path.is_empty() {
                frame.text = fs::read_to_string(path.join(&frame.text_path))?;
            }
        }
    }
    let rooms: Vec<RoomDefinition> = serde_json::from_str(&fs::read_to_string(
        localized_file_path(path, &effective_locale, "rooms.json"),
    )?)?;
    let actors: Vec<ActorDefinition> = serde_json::from_str(&fs::read_to_string(
        localized_file_path(path, &effective_locale, "actors.json"),
    )?)?;
    let appointment_patients = read_optional_localized_json::<Vec<AppointmentPatientDefinition>>(
        path,
        &effective_locale,
        "patients.json",
    )?
    .unwrap_or_default();
    let stats = read_optional_json::<StatsDefinition>(path, "stats.json")?.unwrap_or_default();
    let commands =
        read_optional_json::<CommandsDefinition>(path, "commands.json")?.unwrap_or_default();
    validate_player_commands(&commands)?;
    let affordances =
        read_optional_json::<AffordancesDefinition>(path, "affordances.json")?.unwrap_or_default();
    let hooks =
        read_optional_json::<BTreeMap<String, Value>>(path, "hooks.json")?.unwrap_or_default();
    let speech_intents: SpeechIntentsConfig =
        read_optional_json::<SpeechIntentsConfig>(path, "intents.json")?.unwrap_or_default();
    let items: Vec<ItemDefinition> =
        read_optional_json::<Vec<ItemDefinition>>(path, "items.json")?.unwrap_or_default();

    let room_index: HashMap<String, usize> = rooms
        .iter()
        .enumerate()
        .map(|(i, r)| (r.id.clone(), i))
        .collect();
    let actor_index: HashMap<String, usize> = actors
        .iter()
        .enumerate()
        .map(|(i, a)| (a.id.clone(), i))
        .collect();
    let command_index: HashMap<String, usize> = commands
        .actions
        .iter()
        .enumerate()
        .map(|(i, c)| (c.id.clone(), i))
        .collect();
    let affordance_index: HashMap<String, usize> = affordances
        .actions
        .iter()
        .enumerate()
        .map(|(i, a)| (a.id.clone(), i))
        .collect();

    let stage_ids: Vec<&str> = beats.stages.iter().map(|s| s.id.as_str()).collect();

    for id in &beats.initial_stage_ids {
        require_known_id(
            id,
            &stage_ids,
            &format!("initial_stage_id '{id}'"),
            "beats.stages",
        )?;
    }
    let valid_operators = [
        "equal",
        "greater_than",
        "less_than",
        "gte",
        "lte",
        "not_equal",
        "array_contains",
    ];
    for stage in &beats.stages {
        for id in stage.resolved_next_stage_ids() {
            require_known_id(
                &id,
                &stage_ids,
                &format!("beat '{}' next_stage_id '{id}'", stage.id),
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
    for action in &affordances.actions {
        if !command_index.contains_key(&action.command_id) {
            return Err(format!(
                "affordance '{}' command_id '{}' not found in commands",
                action.id, action.command_id
            )
            .into());
        }
    }
    for actor in &actors {
        if !room_index.contains_key(&actor.room_id) {
            return Err(format!(
                "actor '{}' room_id '{}' not found in rooms",
                actor.id, actor.room_id
            )
            .into());
        }
    }
    if settings.multi_appointment {
        if settings.appointment_patient_actor_id.trim().is_empty() {
            return Err(
                "multi_appointment content requires settings.appointment_patient_actor_id".into(),
            );
        }
        if appointment_patients.is_empty() {
            return Err("multi_appointment content requires localized patients.json".into());
        }
        let mut seen_patient_ids = std::collections::BTreeSet::new();
        for patient in &appointment_patients {
            if patient.id.trim().is_empty() {
                return Err("appointment patient definition is missing id".into());
            }
            if !seen_patient_ids.insert(patient.id.clone()) {
                return Err(format!("duplicate appointment patient id '{}'", patient.id).into());
            }
        }
    }

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
        appointment_patients,
        stats,
        commands,
        affordances,
        hooks,
        speech_intents,
        items,
        room_index,
        actor_index,
        command_index,
        affordance_index,
    })
}

use crate::content::loader_validation::{require_known_id, validate_player_commands};

pub fn available_locales(path: &Path) -> Result<Vec<LocaleOption>, Box<dyn Error>> {
    let locales_dir = path.join("locales");
    let mut locales = Vec::new();
    if locales_dir.exists() {
        for entry in fs::read_dir(&locales_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let code = entry.file_name().to_string_lossy().to_string();
            let ui_path = entry.path().join("ui.json");
            let label = match fs::read_to_string(ui_path) {
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

fn read_optional_localized_json<T: DeserializeOwned>(
    path: &Path,
    locale: &str,
    file_name: &str,
) -> Result<Option<T>, Box<dyn Error>> {
    let file_path = localized_file_path(path, locale, file_name);
    match fs::read_to_string(file_path) {
        Ok(contents) => Ok(Some(serde_json::from_str(&contents)?)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn read_optional_json<T: DeserializeOwned>(
    path: &Path,
    file_name: &str,
) -> Result<Option<T>, Box<dyn Error>> {
    match fs::read_to_string(path.join(file_name)) {
        Ok(contents) => Ok(Some(serde_json::from_str(&contents)?)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn read_optional_localized_json_with_fallback<T: DeserializeOwned>(
    path: &Path,
    locale: &str,
    file_name: &str,
    fallback_file_name: Option<&str>,
) -> Result<Option<T>, Box<dyn Error>> {
    let file_path = localized_file_path_with_fallback(path, locale, file_name, fallback_file_name);
    match fs::read_to_string(file_path) {
        Ok(contents) => Ok(Some(serde_json::from_str(&contents)?)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn localized_file_path_with_fallback(
    path: &Path,
    locale: &str,
    file_name: &str,
    fallback_file_name: Option<&str>,
) -> PathBuf {
    let localized = path.join("locales").join(locale).join(file_name);
    if localized.exists() {
        return localized;
    }
    let default_localized = path.join("locales").join(DEFAULT_LOCALE).join(file_name);
    if default_localized.exists() {
        return default_localized;
    }
    let direct = path.join(file_name);
    if direct.exists() {
        return direct;
    }
    if let Some(fallback_file_name) = fallback_file_name {
        return localized_file_path(path, locale, fallback_file_name);
    }
    direct
}

fn localized_file_path(path: &Path, locale: &str, file_name: &str) -> PathBuf {
    let localized = path.join("locales").join(locale).join(file_name);
    if localized.exists() {
        return localized;
    }
    let default_localized = path.join("locales").join(DEFAULT_LOCALE).join(file_name);
    if default_localized.exists() {
        return default_localized;
    }
    path.join(file_name)
}
