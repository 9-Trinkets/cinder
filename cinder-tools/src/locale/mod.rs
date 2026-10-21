//! Locale authoring helpers: scaffold a new locale from a source locale and
//! apply reviewed translations.
//!
//! A locale directory (`content/<pack>/locales/<code>/`) stores the full pack
//! content with text translated in place. Because the loader performs no
//! fallback between locales, a new locale must duplicate the full structure of
//! its source. [`scaffold`] copies that structure verbatim; translations are
//! then authored as compact per-file maps under `<locale>/translations/` and
//! written into the files by [`apply::apply`].

use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

pub mod apply;
pub mod overlay_check;
pub mod scaffold;

/// Subdirectory of a locale holding translation maps, one file per content file.
pub const TRANSLATIONS_DIR: &str = "translations";

/// Files whose every string leaf is player-facing prose.
const ALL_TEXT_FILES: &[&str] = &["ui.json", "system.json", "presentation.json"];

/// Keys whose string values are prose, in structurally-mixed files.
const PROSE_KEYS: &[&str] = &[
    "title",
    "label",
    "menu_label",
    "summary",
    "inspect_text",
    "intro_text",
    "help_text",
    "line",
    "name",
    "quest_title",
    "beat_note",
    "update_message",
    "text",
    "description",
    "character_notes",
    "subtext_notes",
    "response_notes",
    "behavior_examples",
    "setting_notes",
    "narrative_lines",
    "opening_narrative_lines",
    "fragment",
    "who",
];

pub(crate) fn escape_segment(seg: &str) -> String {
    seg.replace('~', "~0").replace('/', "~1")
}

fn collect_for_file(file: &str, value: &Value) -> Vec<(String, String)> {
    let mut raw = Vec::new();
    if file == "messages.json" {
        walk_messages(value, "", &mut raw);
    } else if ALL_TEXT_FILES.contains(&file) {
        walk_all_strings(value, "", &mut raw);
    } else {
        walk_prose(value, "", &mut raw);
    }
    raw.retain(|(_, source)| !source.is_empty());
    raw
}

/// Keys whose values are machine identifiers or enum tags, never translated —
/// even in `ALL_TEXT_FILES` where string leaves are otherwise prose.
fn is_structural_key(k: &str) -> bool {
    k == "id" || k.ends_with("_id") || k.ends_with("_ids")
}

fn walk_all_strings(value: &Value, pointer: &str, out: &mut Vec<(String, String)>) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                if is_structural_key(k) {
                    continue;
                }
                let child = format!("{pointer}/{}", escape_segment(k));
                walk_all_strings(v, &child, out);
            }
        }
        Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                let child = format!("{pointer}/{i}");
                walk_all_strings(v, &child, out);
            }
        }
        Value::String(s) => out.push((pointer.to_string(), s.clone())),
        _ => {}
    }
}

fn walk_messages(value: &Value, pointer: &str, out: &mut Vec<(String, String)>) {
    let Value::Object(map) = value else {
        return;
    };
    for (k, v) in map {
        let child = format!("{pointer}/{}", escape_segment(k));
        match v {
            Value::String(s) => out.push((child, s.clone())),
            Value::Object(o) => {
                if let Some(Value::String(t)) = o.get("text") {
                    out.push((format!("{child}/text"), t.clone()));
                }
            }
            _ => {}
        }
    }
}

fn walk_prose(value: &Value, pointer: &str, out: &mut Vec<(String, String)>) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                let child = format!("{pointer}/{}", escape_segment(k));
                match v {
                    Value::String(s) => {
                        if PROSE_KEYS.contains(&k.as_str()) {
                            out.push((child, s.clone()));
                        }
                    }
                    Value::Array(_) if PROSE_KEYS.contains(&k.as_str()) => {
                        collect_string_leaves(v, &child, out);
                    }
                    _ => walk_prose(v, &child, out),
                }
            }
        }
        Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                let child = format!("{pointer}/{i}");
                walk_prose(v, &child, out);
            }
        }
        _ => {}
    }
}

/// Collects every string leaf (recursing through arrays and objects), used for
/// prose keys whose value is a list of strings.
fn collect_string_leaves(value: &Value, pointer: &str, out: &mut Vec<(String, String)>) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                let child = format!("{pointer}/{}", escape_segment(k));
                collect_string_leaves(v, &child, out);
            }
        }
        Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                let child = format!("{pointer}/{i}");
                collect_string_leaves(v, &child, out);
            }
        }
        Value::String(s) => out.push((pointer.to_string(), s.clone())),
        _ => {}
    }
}

/// Reads every content JSON file at the top level of a locale directory.
pub(crate) fn read_locale_dir(
    dir: &Path,
) -> Result<BTreeMap<String, Value>, Box<dyn std::error::Error>> {
    if !dir.is_dir() {
        return Err(format!("locale directory not found: {}", dir.display()).into());
    }
    let mut names: Vec<String> = std::fs::read_dir(dir)?
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            if !path.is_file() {
                return None;
            }
            path.file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
        })
        .filter(|n| n.ends_with(".json") && !n.starts_with('_'))
        .collect();
    names.sort();

    let mut files = BTreeMap::new();
    for name in names {
        let path = dir.join(&name);
        let text = std::fs::read_to_string(&path)?;
        let value: Value =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        files.insert(name, value);
    }
    Ok(files)
}

/// Collects translatable `(pointer, source_text)` pairs per file in a locale.
pub fn collect_entries(
    dir: &Path,
) -> Result<BTreeMap<String, Vec<(String, String)>>, Box<dyn std::error::Error>> {
    let files = read_locale_dir(dir)?;
    let mut entries = BTreeMap::new();
    for (name, value) in &files {
        entries.insert(name.clone(), collect_for_file(name, value));
    }
    Ok(entries)
}

/// Loads and merges the per-file translation maps under a `translations/` dir.
pub(crate) fn load_maps(
    dir: &Path,
) -> Result<BTreeMap<String, BTreeMap<String, String>>, Box<dyn std::error::Error>> {
    let mut maps = BTreeMap::new();
    if !dir.is_dir() {
        return Ok(maps);
    }
    let mut names: Vec<String> = std::fs::read_dir(dir)?
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            if !path.is_file() {
                return None;
            }
            path.file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
        })
        .filter(|n| n.ends_with(".json"))
        .collect();
    names.sort();
    for name in names {
        let path = dir.join(&name);
        let text = std::fs::read_to_string(&path)?;
        let map: BTreeMap<String, String> =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        maps.insert(name, map);
    }
    Ok(maps)
}
