//! Coverage check for pack files that are localized through overlays
//! (`items.json`, `skills.json`, `actions.json`, `settings.json`) rather than by
//! duplicating the whole file. Every prose string in the pack-root file must
//! have a differing counterpart in `locales/<code>/<file>`.

use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

/// Pack-root files whose player-facing text lives in a locale overlay.
const OVERLAY_FILES: &[&str] = &["items.json", "skills.json", "actions.json", "settings.json"];

/// Keys whose values are prose the player reads.
const PROSE_KEYS: &[&str] = &[
    "label",
    "description",
    "look_description",
    "title",
    "prompt",
    "summary_template",
    "zero_summary",
    "event_text",
    "usage",
    "example",
    "tagline",
];

/// Flattens prose leaves into `path -> text`, addressing array entries by their
/// `id` so a base file and its overlay line up regardless of order.
fn flatten(value: &Value, path: &str, out: &mut BTreeMap<String, String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let next = format!("{path}/{key}");
                match child {
                    Value::String(text)
                        if PROSE_KEYS.contains(&key.as_str()) && !text.is_empty() =>
                    {
                        out.insert(next, text.clone());
                    }
                    Value::Array(items) if key == "tags" && items.iter().all(Value::is_string) => {
                        let joined = items
                            .iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join("|");
                        out.insert(next, joined);
                    }
                    _ => flatten(child, &next, out),
                }
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                let segment = child
                    .get("id")
                    .and_then(Value::as_str)
                    .map_or_else(|| index.to_string(), str::to_string);
                flatten(child, &format!("{path}/{segment}"), out);
            }
        }
        _ => {}
    }
}

/// Prints per-file overlay coverage and returns the number of pending strings.
pub fn check_overlays(pack_dir: &Path, locale: &str) -> Result<usize, Box<dyn std::error::Error>> {
    let mut pending_all = 0usize;
    let mut printed_header = false;
    for file in OVERLAY_FILES {
        let base_path = pack_dir.join(file);
        if !base_path.is_file() {
            continue;
        }
        let base: Value = serde_json::from_str(&std::fs::read_to_string(&base_path)?)
            .map_err(|e| format!("{}: {e}", base_path.display()))?;
        let mut base_entries = BTreeMap::new();
        flatten(&base, "", &mut base_entries);
        if base_entries.is_empty() {
            continue;
        }

        let overlay_path = pack_dir.join("locales").join(locale).join(file);
        let mut overlay_entries = BTreeMap::new();
        if overlay_path.is_file() {
            let overlay: Value = serde_json::from_str(&std::fs::read_to_string(&overlay_path)?)
                .map_err(|e| format!("{}: {e}", overlay_path.display()))?;
            flatten(&overlay, "", &mut overlay_entries);
        }

        let mut pending: Vec<&String> = Vec::new();
        for (path, source) in &base_entries {
            match overlay_entries.get(path) {
                Some(text) if text != source => {}
                _ => pending.push(path),
            }
        }
        if !printed_header {
            println!("\nOverlay files (pack-root text localized per locale):");
            printed_header = true;
        }
        let filled = base_entries.len() - pending.len();
        let status = if pending.is_empty() { "ok" } else { "pending" };
        println!("{file:<20} {filled:>5}/{:<5} {status}", base_entries.len());
        for path in pending.iter().take(10) {
            println!("    missing: {path}");
        }
        pending_all += pending.len();
    }
    Ok(pending_all)
}
