//! Locale overlays for pack files that hold both rules and player-facing text.
//!
//! Files such as `items.json`, `skills.json` and `actions.json` mix logic with
//! prose, so duplicating them per locale (as the prose-only files are) would
//! make every rules change a multi-locale edit. Instead a locale may ship a
//! partial file at `locales/<code>/<file>.json` that is deep-merged over the
//! pack-root file before it is deserialized.
//!
//! Merge rules:
//! - objects merge key by key;
//! - arrays of objects that carry an `id` merge by `id` (unmatched overlay
//!   entries are appended);
//! - string arrays under `phrases` or `aliases` are unioned, so a locale can add
//!   typed-command vocabulary without dropping the base phrases;
//! - everything else (strings, numbers, other arrays) is replaced.

use serde::de::DeserializeOwned;
use serde_json::Value;
use std::error::Error;
use std::fs;
use std::path::Path;

/// Keys whose string-array values are unioned instead of replaced.
const UNION_KEYS: &[&str] = &["phrases", "aliases"];

/// Reads `file_name` from the pack root and layers the locale's overlay (when
/// one exists) over it. Returns `Ok(None)` when the pack has no such file.
pub fn read_with_overlay<T: DeserializeOwned>(
    pack_dir: &Path,
    locale: &str,
    file_name: &str,
) -> Result<Option<T>, Box<dyn Error>> {
    let base_path = pack_dir.join(file_name);
    let mut value: Value = match fs::read_to_string(&base_path) {
        Ok(contents) => {
            serde_json::from_str(&contents).map_err(|e| format!("{}: {e}", base_path.display()))?
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };

    let overlay_path = pack_dir.join("locales").join(locale).join(file_name);
    match fs::read_to_string(&overlay_path) {
        Ok(contents) => {
            let overlay: Value = serde_json::from_str(&contents)
                .map_err(|e| format!("{}: {e}", overlay_path.display()))?;
            merge_overlay(&mut value, overlay, "");
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }

    serde_json::from_value(value)
        .map(Some)
        .map_err(|e| format!("{}: {e}", base_path.display()).into())
}

fn id_of(value: &Value) -> Option<&str> {
    value.get("id").and_then(Value::as_str)
}

fn is_id_array(items: &[Value]) -> bool {
    !items.is_empty() && items.iter().all(|item| id_of(item).is_some())
}

/// Deep-merges `overlay` into `base` following the module's merge rules. `key`
/// is the object key under which `base` sits (empty at the root).
pub fn merge_overlay(base: &mut Value, overlay: Value, key: &str) {
    match (base, overlay) {
        (Value::Object(base_map), Value::Object(overlay_map)) => {
            for (child_key, overlay_child) in overlay_map {
                match base_map.get_mut(&child_key) {
                    Some(base_child) => merge_overlay(base_child, overlay_child, &child_key),
                    None => {
                        base_map.insert(child_key, overlay_child);
                    }
                }
            }
        }
        (Value::Array(base_items), Value::Array(overlay_items)) => {
            if is_id_array(base_items) && is_id_array(&overlay_items) {
                for overlay_item in overlay_items {
                    let id = id_of(&overlay_item).unwrap_or_default().to_string();
                    match base_items
                        .iter_mut()
                        .find(|item| id_of(item) == Some(id.as_str()))
                    {
                        Some(base_item) => merge_overlay(base_item, overlay_item, ""),
                        None => base_items.push(overlay_item),
                    }
                }
            } else if UNION_KEYS.contains(&key) {
                for overlay_item in overlay_items {
                    if !base_items.contains(&overlay_item) {
                        base_items.push(overlay_item);
                    }
                }
            } else {
                *base_items = overlay_items;
            }
        }
        (base_slot, overlay_value) => *base_slot = overlay_value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn objects_merge_and_scalars_replace() {
        let mut base = json!({"label": "Look", "ui": {"title": "Look Around", "bar": true}});
        merge_overlay(
            &mut base,
            json!({"label": "查看", "ui": {"title": "環顧四周"}}),
            "",
        );
        assert_eq!(
            base,
            json!({"label": "查看", "ui": {"title": "環顧四周", "bar": true}})
        );
    }

    #[test]
    fn id_arrays_merge_by_id_and_append_unknown() {
        let mut base = json!([
            {"id": "a", "label": "A", "kind": "misc"},
            {"id": "b", "label": "B"}
        ]);
        merge_overlay(
            &mut base,
            json!([{"id": "b", "label": "乙"}, {"id": "c", "label": "丙"}]),
            "",
        );
        assert_eq!(
            base,
            json!([
                {"id": "a", "label": "A", "kind": "misc"},
                {"id": "b", "label": "乙"},
                {"id": "c", "label": "丙"}
            ])
        );
    }

    #[test]
    fn phrases_and_aliases_union_but_other_string_arrays_replace() {
        let mut base =
            json!({"phrases": ["look", "l"], "aliases": ["chalk"], "tags": ["Sigil Magic"]});
        merge_overlay(
            &mut base,
            json!({"phrases": ["查看", "l"], "aliases": ["粉筆"], "tags": ["符紋魔法"]}),
            "",
        );
        assert_eq!(
            base,
            json!({"phrases": ["look", "l", "查看"], "aliases": ["chalk", "粉筆"], "tags": ["符紋魔法"]})
        );
    }

    #[test]
    fn nested_id_arrays_inside_objects_merge() {
        let mut base = json!({"skills": [{"id": "strike", "player_action": {"label": "Attack", "phrases": ["attack"]}}]});
        merge_overlay(
            &mut base,
            json!({"skills": [{"id": "strike", "player_action": {"label": "攻擊", "phrases": ["攻擊"]}}]}),
            "",
        );
        assert_eq!(
            base,
            json!({"skills": [{"id": "strike", "player_action": {"label": "攻擊", "phrases": ["attack", "攻擊"]}}]})
        );
    }
}
