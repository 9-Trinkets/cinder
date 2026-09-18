use cinder_core::content::loader::load_pack_from_dir_with_locale;
use cinder_core::content::types::DropSpec;
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Debug, Default)]
pub struct LintReport {
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

impl LintReport {
    pub fn is_clean(&self) -> bool {
        self.warnings.is_empty() && self.errors.is_empty()
    }
}

pub fn lint_pack(pack_dir: &Path, locale: &str) -> LintReport {
    let mut report = LintReport::default();
    let pack_name = pack_dir.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");

    let pack = match load_pack_from_dir_with_locale(pack_dir, Some(locale)) {
        Ok(p) => p,
        Err(err) => {
            report.errors.push(format!("[{pack_name}] pack failed to load: {err}"));
            return report;
        }
    };

    let room_ids: BTreeSet<&str> = pack.rooms.iter().map(|r| r.id.as_str()).collect();
    let item_ids: BTreeSet<&str> = pack.items.iter().map(|i| i.id.as_str()).collect();
    let hook_names: BTreeSet<&str> = pack.hooks.keys().map(|k| k.as_str()).collect();

    // 1. Room exits
    for room in &pack.rooms {
        for exit in &room.exits {
            if !room_ids.contains(exit.room_id.as_str()) {
                report.warnings.push(format!(
                    "[{pack_name}] room '{}' has exit to unknown room '{}'",
                    room.id, exit.room_id
                ));
            }

            // Check reciprocal cardinal exits
            let opp_label = match exit.label.as_str() {
                "North" => Some("South"),
                "South" => Some("North"),
                "East" => Some("West"),
                "West" => Some("East"),
                _ => None,
            };
            if let Some(back_label) = opp_label
                && let Some(target_room) = pack.room(&exit.room_id) {
                    let has_back = target_room.exits.iter().any(|e| e.room_id == room.id && e.label == back_label);
                    if !has_back && exit.requires_story_var.is_empty() {
                        // Non-gated cardinal exit should ideally be bidirectional
                        report.warnings.push(format!(
                            "[{pack_name}] one-way cardinal exit: '{}' -> '{}' ({}) lacks reciprocal '{}' in target",
                            room.id, exit.room_id, exit.label, back_label
                        ));
                    }
                }
        }

        // Features
        for feature in &room.features {
            if feature.label.trim().is_empty() {
                report.warnings.push(format!("[{pack_name}] room '{}' feature '{}' has empty label", room.id, feature.id));
            }
            if feature.inspect_text.trim().is_empty() {
                report.warnings.push(format!("[{pack_name}] room '{}' feature '{}' has empty inspect text", room.id, feature.id));
            }
        }
    }

    // 2. Maps
    for map in &pack.maps {
        for map_room in &map.rooms {
            if !room_ids.contains(map_room.room_id.as_str()) {
                report.warnings.push(format!(
                    "[{pack_name}] map '{}' references unknown room '{}'",
                    map.id, map_room.room_id
                ));
            }
        }
    }

    // 3. Actors
    for actor in &pack.actors {
        if !actor.room_id.is_empty() && !room_ids.contains(actor.room_id.as_str()) {
            report.warnings.push(format!(
                "[{pack_name}] actor '{}' starts in unknown room '{}'",
                actor.id, actor.room_id
            ));
        }

        for (item_id, drop_spec) in &actor.drops {
            match drop_spec {
                DropSpec::Always(_) | DropSpec::Chance(_) | DropSpec::Conditional(_) => {
                    if !item_ids.contains(item_id.as_str()) {
                        report.warnings.push(format!(
                            "[{pack_name}] actor '{}' drops unknown item '{}'",
                            actor.id, item_id
                        ));
                    }
                }
                DropSpec::Weighted(pool) => {
                    for entry in &pool.entries {
                        if !item_ids.contains(entry.item_id.as_str()) {
                            report.warnings.push(format!(
                                "[{pack_name}] actor '{}' weighted drop references unknown item '{}'",
                                actor.id, entry.item_id
                            ));
                        }
                    }
                }
            }
        }
    }

    // 4. Items & Hooks
    for item in &pack.items {
        if !item.use_hook.is_empty() && !hook_names.contains(item.use_hook.as_str()) {
            report.warnings.push(format!(
                "[{pack_name}] item '{}' use_hook '{}' not found in hooks.json",
                item.id, item.use_hook
            ));
        }
    }

    // 5. Action creation gates
    for action in &pack.actions {
        if let Some(creation) = &action.item_creation {
            for craftable_id in creation.craftable_item_gates.keys() {
                if !item_ids.contains(craftable_id.as_str()) {
                    report.warnings.push(format!(
                        "[{pack_name}] action '{}' craftable gate references unknown item '{}'",
                        action.id, craftable_id
                    ));
                }
            }
        }
    }

    report
}
