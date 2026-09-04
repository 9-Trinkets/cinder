use crate::content::loader::fs::{localized_file_path, read_optional_path, read_required_path, LocalizedPaths};
use crate::content::types::SystemTextDefinition;
use serde_json::Value;
use std::collections::BTreeMap;
use std::error::Error;

/// Reads a pack's `messages.json`, layering it over the engine's bundled
/// `messages_defaults.json`. Packs override the engine's default narration
/// keys they define; anything else falls back to the bundled value. This keeps
/// the engine's default player-facing narration in a JSON file rather than
/// hardcoded in Rust.
pub fn read_messages(paths: &LocalizedPaths<'_>) -> Result<BTreeMap<String, String>, Box<dyn Error>> {
    let defaults: BTreeMap<String, String> =
        serde_json::from_str(include_str!("../messages_defaults.json"))
            .expect("invalid bundled messages_defaults.json");
    let mut merged = defaults;
    let pack_messages = read_optional_path::<BTreeMap<String, String>>(&localized_file_path(
        paths.root,
        paths.locale,
        "messages.json",
    ))?
    .unwrap_or_default();
    merged.extend(pack_messages);
    Ok(merged)
}

/// Reads a pack's `system.json`, layering it over the engine's bundled
/// `system_defaults.json`. Required fields must still be declared by the pack;
/// defaulted fields fall back to the bundled values unless the pack overrides
/// them. This keeps the engine's prompt/label defaults in a JSON file rather
/// than hardcoded in Rust.
pub fn read_system_text(paths: &LocalizedPaths<'_>) -> Result<SystemTextDefinition, Box<dyn Error>> {
    let defaults: Value = serde_json::from_str(include_str!("../system_defaults.json"))
        .expect("invalid bundled system_defaults.json");
    let mut merged = defaults
        .as_object()
        .cloned()
        .unwrap_or_else(serde_json::Map::new);
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
