use crate::content::types::{
    ActCastMember, ActorDefinition, AffordancesDefinition, BeatsDefinition, CommandsDefinition,
    ContentPack, ContentSettingsDefinition, ItemDefinition, MovementConfigDefinition,
    OpeningDefinition, OpeningMenuDefinition, OpeningMovieDefinition, PresentationDefinition,
    RoomDefinition, RuleBundleProgressRef, RuleBundlesDefinition, SpeechConfigDefinition,
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

pub fn available_packs() -> Vec<String> {
    let dir = content_dir();
    let mut packs: Vec<String> = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let settings_path = path.join("settings.json");
                if settings_path.exists() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        packs.push(name.to_string());
                    }
                }
            }
        }
    }
    packs.sort();
    packs
}

pub fn load_pack_settings(pack_id: &str) -> Result<ContentSettingsDefinition, Box<dyn Error>> {
    Ok(
        read_optional_path::<ContentSettingsDefinition>(&pack_dir(pack_id).join("settings.json"))?
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
    let settings = read_optional_path::<ContentSettingsDefinition>(&path.join("settings.json"))?
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
    let system_text = paths.read_required::<SystemTextDefinition>("system.json")?;
    let opening = paths
        .read_required_with_fallback::<OpeningDefinition>("opening.json", Some("scenario.json"))?;
    let beats = paths
        .read_optional_with_fallback::<BeatsDefinition>("beats.json", Some("objective_flow.json"))?
        .unwrap_or_default();
    let menus = paths
        .read_optional::<Vec<OpeningMenuDefinition>>("menus.json")?
        .unwrap_or_default();
    let mut movies = paths
        .read_optional_with_fallback::<Vec<OpeningMovieDefinition>>(
            "movies.json",
            Some("projector_sequences.json"),
        )?
        .unwrap_or_default();
    let presentation = paths
        .read_optional::<PresentationDefinition>("presentation.json")?
        .unwrap_or_default();
    for movie in &mut movies {
        for frame in &mut movie.frames {
            if !frame.text_path.is_empty() {
                frame.text = fs::read_to_string(path.join(&frame.text_path))?;
            }
        }
    }
    let rooms = paths.read_required::<Vec<RoomDefinition>>("rooms.json")?;
    let actors = paths.read_required::<Vec<ActorDefinition>>("actors.json")?;
    let act_cast = collect_act_cast(&actors);
    let stats = read_optional_json::<StatsDefinition>(path, "stats.json")?.unwrap_or_default();
    let commands =
        read_optional_json::<CommandsDefinition>(path, "commands.json")?.unwrap_or_default();
    validate_player_commands(&commands)?;
    let affordances =
        read_optional_json::<AffordancesDefinition>(path, "affordances.json")?.unwrap_or_default();
    let movement =
        read_optional_json::<MovementConfigDefinition>(path, "movement.json")?.unwrap_or_default();
    let speech =
        read_optional_json::<SpeechConfigDefinition>(path, "speech.json")?.unwrap_or_default();
    let rule_bundles = read_json::<RuleBundlesDefinition>(path, "rule_bundles.json")?;
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

    let room_index = build_index(&rooms, |room| &room.id);
    let actor_index = build_index(&actors, |actor| &actor.id);
    let command_index = build_index(&commands.actions, |command| &command.id);
    let affordance_index = build_index(&affordances.actions, |affordance| &affordance.id);
    let room_ids = rooms
        .iter()
        .map(|room| room.id.as_str())
        .collect::<Vec<_>>();
    let actor_ids = actors
        .iter()
        .map(|actor| actor.id.as_str())
        .collect::<Vec<_>>();

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
        if let Some(config) = &stage.stage_assignment {
            if !config.initiator_actor_id.trim().is_empty() {
                require_known_id(
                    &config.initiator_actor_id,
                    &actor_ids,
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
                    &room_ids,
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
                    &room_ids,
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
                &stage_ids,
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
    for action in &affordances.actions {
        require_known_command_id(
            &action.command_id,
            &format!("affordance '{}'", action.id),
            &command_index,
        )?;
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
    for actor_id in movement.actors.keys() {
        require_known_id(
            actor_id,
            &actor_ids,
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
                    &room_ids,
                    &format!("{context} target_room_id '{}'", rule.target_room_id),
                    "rooms",
                )?;
            }
            for stage_id in &rule.any_active_stage_ids {
                require_known_id(
                    stage_id,
                    &stage_ids,
                    &format!("{context} any_active_stage_ids entry '{stage_id}'"),
                    "beats.stages",
                )?;
            }
        }
    }
    for stage_id in &movement.stage_locks {
        require_known_id(
            stage_id,
            &stage_ids,
            &format!("movement.json stage_locks entry '{stage_id}'"),
            "beats.stages",
        )?;
    }
    for bundle in &rule_bundles.bundles {
        if bundle.id.trim().is_empty() {
            return Err("rule_bundles.json bundles entries require non-empty id".into());
        }
        let bundle_stage_ids = bundle
            .stage_ids
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        if bundle_stage_ids.is_empty() {
            return Err(format!(
                "rule_bundles.json bundle '{}' requires at least one stage id",
                bundle.id
            )
            .into());
        }
        for stage_id in bundle_stage_ids {
            require_known_id(
                stage_id,
                &stage_ids,
                &format!(
                    "rule_bundles.json bundle '{}' stage id '{}'",
                    bundle.id, stage_id
                ),
                "beats.stages",
            )?;
        }
        let mut seen_progress_keys = std::collections::BTreeSet::new();
        for progress in &bundle.progress.keys {
            if progress.key.trim().is_empty() {
                return Err(format!(
                    "rule_bundles.json bundle '{}' has progress entry with empty key",
                    bundle.id
                )
                .into());
            }
            if !seen_progress_keys.insert(progress.key.clone()) {
                return Err(format!(
                    "rule_bundles.json bundle '{}' has duplicate progress key '{}'",
                    bundle.id, progress.key
                )
                .into());
            }
        }
        for priority in &bundle.guidance.prioritize {
            if priority.command_id.trim().is_empty() {
                return Err(format!(
                    "rule_bundles.json bundle '{}' has prioritize entry with empty command_id",
                    bundle.id
                )
                .into());
            }
            require_known_command_id(
                &priority.command_id,
                &format!("rule_bundles.json bundle '{}' prioritize", bundle.id),
                &command_index,
            )?;
        }
        for (index, conditional) in bundle.guidance.conditional.iter().enumerate() {
            for priority in &conditional.prioritize {
                if priority.command_id.trim().is_empty() {
                    return Err(format!(
                        "rule_bundles.json bundle '{}' conditional guidance #{} has prioritize entry with empty command_id",
                        bundle.id,
                        index + 1
                    )
                    .into());
                }
                require_known_command_id(
                    &priority.command_id,
                    &format!(
                        "rule_bundles.json bundle '{}' conditional guidance #{} prioritize",
                        bundle.id,
                        index + 1
                    ),
                    &command_index,
                )?;
            }
        }
    }
    let bundle_progress_keys = rule_bundles
        .bundles
        .iter()
        .map(|bundle| {
            (
                bundle.id.as_str(),
                bundle
                    .progress
                    .keys
                    .iter()
                    .map(|progress| progress.key.as_str())
                    .collect::<std::collections::BTreeSet<_>>(),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    for command in &commands.actions {
        for stage_id in &command.available_during {
            require_known_id(
                stage_id,
                &stage_ids,
                &format!(
                    "command '{}' available_during stage_id '{}'",
                    command.id, stage_id
                ),
                "beats.stages",
            )?;
        }
        validate_bundle_progress_refs(
            &format!("command '{}' bundle progress", command.id),
            command
                .required_bundle_progress
                .iter()
                .chain(command.blocked_by_bundle_progress.iter())
                .chain(command.sets_bundle_progress.iter())
                .chain(command.clears_bundle_progress.iter()),
            &bundle_progress_keys,
        )?;
        for bundle in &rule_bundles.bundles {
            for conditional in &bundle.guidance.conditional {
                validate_bundle_progress_refs(
                    &format!(
                        "rule_bundles.json bundle '{}' conditional guidance bundle progress",
                        bundle.id
                    ),
                    conditional
                        .required_bundle_progress
                        .iter()
                        .chain(conditional.blocked_by_bundle_progress.iter()),
                    &bundle_progress_keys,
                )?;
            }
        }
    }
    for room_id in &movement.unreachable_rooms {
        require_known_id(
            room_id,
            &room_ids,
            &format!("movement.json unreachable_rooms entry '{room_id}'"),
            "rooms",
        )?;
    }
    if !act_cast.is_empty() {
        let mut seen_member_ids = std::collections::BTreeSet::new();
        let mut seen_member_actor_ids = std::collections::BTreeSet::new();
        for member in &act_cast {
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
                &actor_ids,
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
        act_cast,
        stats,
        commands,
        affordances,
        movement,
        speech,
        rule_bundles,
        hooks,
        speech_intents,
        items,
        variables,
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

fn read_optional_json<T: DeserializeOwned>(
    path: &Path,
    file_name: &str,
) -> Result<Option<T>, Box<dyn Error>> {
    read_optional_path(&path.join(file_name))
}

fn read_json<T: DeserializeOwned>(path: &Path, file_name: &str) -> Result<T, Box<dyn Error>> {
    read_required_path(&path.join(file_name))
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

fn read_optional_path<T: DeserializeOwned>(path: &Path) -> Result<Option<T>, Box<dyn Error>> {
    match fs::read_to_string(path) {
        Ok(contents) => Ok(Some(serde_json::from_str(&contents)?)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn read_required_path<T: DeserializeOwned>(path: &Path) -> Result<T, Box<dyn Error>> {
    let contents = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&contents)?)
}

fn collect_act_cast(actors: &[ActorDefinition]) -> Vec<ActCastMember> {
    actors
        .iter()
        .filter_map(|actor| {
            let act_cast = actor.act_cast.as_ref()?;
            Some(ActCastMember {
                id: actor.id.clone(),
                name: actor.name.clone(),
                actor_id: actor.id.clone(),
                inspect_blurb: act_cast.inspect_blurb.clone(),
                intro_blurb: act_cast.intro_blurb.clone(),
                return_blurb: act_cast.return_blurb.clone(),
                metadata: act_cast.metadata.clone(),
                actor_stats: actor.initial_stats.clone(),
            })
        })
        .collect()
}

fn build_index<T, F>(items: &[T], id: F) -> HashMap<String, usize>
where
    F: Fn(&T) -> &str,
{
    items
        .iter()
        .enumerate()
        .map(|(index, item)| (id(item).to_string(), index))
        .collect()
}

fn require_known_command_id(
    command_id: &str,
    subject: &str,
    command_index: &HashMap<String, usize>,
) -> Result<(), Box<dyn Error>> {
    if command_index.contains_key(command_id) {
        Ok(())
    } else {
        Err(format!("{subject} command_id '{command_id}' not found in commands").into())
    }
}

fn validate_bundle_progress_refs<'a>(
    owner: &str,
    refs: impl IntoIterator<Item = &'a RuleBundleProgressRef>,
    bundle_progress_keys: &BTreeMap<&str, std::collections::BTreeSet<&str>>,
) -> Result<(), Box<dyn Error>> {
    for progress in refs {
        if progress.bundle_id.trim().is_empty() || progress.key.trim().is_empty() {
            return Err(format!("{owner} refs require non-empty bundle_id and key").into());
        }
        let Some(keys) = bundle_progress_keys.get(progress.bundle_id.as_str()) else {
            return Err(format!(
                "{owner} bundle_id '{}' not found in rule_bundles",
                progress.bundle_id
            )
            .into());
        };
        if !keys.contains(progress.key.as_str()) {
            return Err(format!(
                "{owner} key '{}' not found in rule bundle '{}'",
                progress.key, progress.bundle_id
            )
            .into());
        }
    }
    Ok(())
}

struct LocalizedPaths<'a> {
    root: &'a Path,
    locale: &'a str,
}

impl<'a> LocalizedPaths<'a> {
    fn new(root: &'a Path, locale: &'a str) -> Self {
        Self { root, locale }
    }

    fn read_optional<T: DeserializeOwned>(
        &self,
        file_name: &str,
    ) -> Result<Option<T>, Box<dyn Error>> {
        read_optional_path(&localized_file_path(self.root, self.locale, file_name))
    }

    fn read_required<T: DeserializeOwned>(&self, file_name: &str) -> Result<T, Box<dyn Error>> {
        read_required_path(&localized_file_path(self.root, self.locale, file_name))
    }

    fn read_optional_with_fallback<T: DeserializeOwned>(
        &self,
        file_name: &str,
        fallback_file_name: Option<&str>,
    ) -> Result<Option<T>, Box<dyn Error>> {
        read_optional_path(&localized_file_path_with_fallback(
            self.root,
            self.locale,
            file_name,
            fallback_file_name,
        ))
    }

    fn read_required_with_fallback<T: DeserializeOwned>(
        &self,
        file_name: &str,
        fallback_file_name: Option<&str>,
    ) -> Result<T, Box<dyn Error>> {
        read_required_path(&localized_file_path_with_fallback(
            self.root,
            self.locale,
            file_name,
            fallback_file_name,
        ))
    }
}
