use crate::content::loader::fs::{
    LocalizedPaths, localized_file_path, read_optional_path, read_required_path,
};
use crate::content::types::{PackMessage, SystemTextDefinition, UiTextDefinition};
use serde_json::Value;
use std::collections::BTreeMap;
use std::error::Error;

/// Reads a pack's `messages.json`, layering it over the engine's bundled
/// `messages_defaults.json`. Packs override the engine's default narration
/// keys they define; anything else falls back to the bundled value. This keeps
/// the engine's default player-facing narration in a JSON file rather than
/// hardcoded in Rust. Entries may be plain strings (world narration) or
/// explicitly system- or handler-voiced messages (see [`PackMessage`]).
///
/// The bundled defaults are localized: the file for the requested locale is
/// used when one ships (`messages_defaults.<locale>.json`), otherwise English.
pub fn read_messages(
    paths: &LocalizedPaths<'_>,
) -> Result<BTreeMap<String, PackMessage>, Box<dyn Error>> {
    let defaults: BTreeMap<String, PackMessage> =
        serde_json::from_str(bundled_messages(paths.locale))
            .expect("invalid bundled messages_defaults.json");
    let mut merged = defaults;
    let pack_messages = read_optional_path::<BTreeMap<String, PackMessage>>(&localized_file_path(
        paths.root,
        paths.locale,
        "messages.json",
    ))?
    .unwrap_or_default();
    merged.extend(pack_messages);
    Ok(merged)
}

/// Selects the bundled default messages for a locale, falling back to English.
fn bundled_messages(locale: &str) -> &'static str {
    match locale {
        "zh-TW" => include_str!("../messages_defaults.zh-TW.json"),
        _ => include_str!("../messages_defaults.json"),
    }
}

/// Selects the bundled locale layer of `system_defaults` (the prompts and
/// labels the engine ships), if the locale has one. English is the base file.
fn bundled_system_defaults(locale: &str) -> Option<&'static str> {
    match locale {
        "zh-TW" => Some(include_str!("../system_defaults.zh-TW.json")),
        _ => None,
    }
}

/// Selects the bundled UI text defaults for a locale, if it ships any. Without
/// one, the English field defaults on `UiTextDefinition` apply.
fn bundled_ui_defaults(locale: &str) -> Option<&'static str> {
    match locale {
        "zh-TW" => Some(include_str!("../ui_defaults.zh-TW.json")),
        _ => None,
    }
}

fn parse_object(source: &str, what: &str) -> serde_json::Map<String, Value> {
    serde_json::from_str::<Value>(source)
        .unwrap_or_else(|e| panic!("invalid bundled {what}: {e}"))
        .as_object()
        .cloned()
        .unwrap_or_default()
}

/// Reads a pack's `system.json`, layering it over the engine's bundled
/// `system_defaults.json` (and the locale's bundled layer, when one ships).
/// Required fields must still be declared by the pack; defaulted fields fall
/// back to the bundled values unless the pack overrides them. This keeps the
/// engine's prompt/label defaults in JSON files rather than hardcoded in Rust.
pub fn read_system_text(
    paths: &LocalizedPaths<'_>,
) -> Result<SystemTextDefinition, Box<dyn Error>> {
    let mut merged = parse_object(
        include_str!("../system_defaults.json"),
        "system_defaults.json",
    );
    if let Some(locale_defaults) = bundled_system_defaults(paths.locale) {
        merged.extend(parse_object(
            locale_defaults,
            "system_defaults locale layer",
        ));
    }
    let pack_system = read_required_path::<Value>(&localized_file_path(
        paths.root,
        paths.locale,
        "system.json",
    ))?;
    if let Value::Object(pack_object) = pack_system {
        for (key, value) in pack_object {
            merged.insert(key, value);
        }
    }
    Ok(serde_json::from_value(Value::Object(merged))?)
}

/// Reads a pack's `ui.json`, layering it over the locale's bundled UI text
/// defaults. Keys neither layer defines fall back to the English field
/// defaults on `UiTextDefinition`, so a locale only needs a bundled file to
/// avoid English leaking through keys a pack does not override.
pub fn read_ui_text(paths: &LocalizedPaths<'_>) -> Result<UiTextDefinition, Box<dyn Error>> {
    let mut merged = bundled_ui_defaults(paths.locale)
        .map(|source| parse_object(source, "ui_defaults locale layer"))
        .unwrap_or_default();
    let pack_ui =
        read_optional_path::<Value>(&localized_file_path(paths.root, paths.locale, "ui.json"))?;
    if let Some(Value::Object(pack_object)) = pack_ui {
        for (key, value) in pack_object {
            merged.insert(key, value);
        }
    }
    Ok(serde_json::from_value(Value::Object(merged))?)
}
