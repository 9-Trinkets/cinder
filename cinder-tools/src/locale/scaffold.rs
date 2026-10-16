//! `locale scaffold`: duplicate a source locale's structure into a new locale.

use super::{TRANSLATIONS_DIR, collect_entries};
use std::path::Path;

pub fn scaffold(
    pack_dir: &Path,
    from: &str,
    to: &str,
    force: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let src = pack_dir.join("locales").join(from);
    let dst = pack_dir.join("locales").join(to);
    if !src.is_dir() {
        return Err(format!("source locale not found: {}", src.display()).into());
    }
    if from == to {
        return Err("source and target locale must differ".into());
    }

    if force {
        if dst.exists() {
            std::fs::remove_dir_all(&dst)?;
        }
        std::fs::create_dir_all(&dst)?;
        copy_files(&src, &dst, true)?;
    } else {
        std::fs::create_dir_all(&dst)?;
        copy_files(&src, &dst, false)?;
    }
    std::fs::create_dir_all(dst.join(TRANSLATIONS_DIR))?;

    let entries = collect_entries(&src)?;
    let total: usize = entries.values().map(Vec::len).sum();
    println!(
        "Scaffolded locale '{}' from '{}' at {}",
        to,
        from,
        dst.display()
    );
    println!(
        "Copied {} content file(s); {} translatable strings.",
        entries.len(),
        total
    );
    println!(
        "Author translations under {} and run: cinder-tools locale apply --locale {}",
        dst.join(TRANSLATIONS_DIR).display(),
        to
    );
    Ok(())
}

fn copy_files(src: &Path, dst: &Path, overwrite: bool) -> Result<(), Box<dyn std::error::Error>> {
    for entry in std::fs::read_dir(src)?.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.ends_with(".json") || name.starts_with('_') {
            continue;
        }
        if !overwrite && dst.join(name).exists() {
            continue;
        }
        std::fs::copy(&path, dst.join(name))?;
    }
    Ok(())
}
