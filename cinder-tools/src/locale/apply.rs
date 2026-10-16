//! `locale apply`: write reviewed translations into the locale files, and
//! `locale check`: report translation coverage.

use super::{TRANSLATIONS_DIR, collect_entries, load_maps, read_locale_dir};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

pub fn apply(pack_dir: &Path, locale: &str) -> Result<usize, Box<dyn std::error::Error>> {
    let dst = pack_dir.join("locales").join(locale);
    let maps = load_maps(&dst.join(TRANSLATIONS_DIR))?;
    let mut files = read_locale_dir(&dst)?;

    let mut applied = 0usize;
    let mut changed: BTreeSet<String> = BTreeSet::new();
    let mut problems: Vec<String> = Vec::new();

    for (file, map) in &maps {
        let Some(value) = files.get_mut(file) else {
            problems.push(format!("{file}: no such locale file"));
            continue;
        };
        for (pointer, zh) in map {
            if zh.is_empty() {
                continue;
            }
            match value.pointer_mut(pointer) {
                Some(slot @ Value::String(_)) => {
                    *slot = Value::String(zh.clone());
                    applied += 1;
                    changed.insert(file.clone());
                }
                Some(_) => problems.push(format!("{file} {pointer}: target is not a string")),
                None => problems.push(format!("{file} {pointer}: pointer unresolved")),
            }
        }
    }

    for name in &changed {
        if let Some(value) = files.get(name) {
            std::fs::write(dst.join(name), serde_json::to_string_pretty(value)? + "\n")?;
        }
    }

    println!(
        "Applied {applied} translations across {} file(s) in '{locale}'.",
        changed.len()
    );
    for problem in &problems {
        eprintln!("warning: {problem}");
    }
    Ok(applied)
}

/// Prints per-file coverage and returns the number of untranslated strings.
pub fn check(
    pack_dir: &Path,
    from: &str,
    locale: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    let src = pack_dir.join("locales").join(from);
    let dst = pack_dir.join("locales").join(locale);
    let entries = collect_entries(&src)?;
    let maps = load_maps(&dst.join(TRANSLATIONS_DIR))?;

    let mut total_all = 0usize;
    let mut filled_all = 0usize;
    for (file, items) in &entries {
        if items.is_empty() {
            continue;
        }
        let map = maps.get(file);
        let filled = items
            .iter()
            .filter(|(pointer, _)| {
                map.and_then(|m| m.get(pointer))
                    .is_some_and(|zh| !zh.is_empty())
            })
            .count();
        total_all += items.len();
        filled_all += filled;
        let status = if filled == items.len() {
            "ok"
        } else {
            "pending"
        };
        println!("{file:<20} {filled:>5}/{:<5} {status}", items.len());
    }
    println!(
        "\nLocale '{locale}': {filled_all}/{total_all} strings translated ({} pending).",
        total_all - filled_all
    );
    Ok(total_all - filled_all)
}
