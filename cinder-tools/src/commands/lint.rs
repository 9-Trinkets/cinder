use std::path::PathBuf;

use crate::lint;

pub fn run(pack: Option<String>, content_dir: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let packs_to_check: Vec<String> = if let Some(p) = pack {
        vec![p]
    } else {
        let mut found = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&content_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir()
                    && let Some(name) = entry.file_name().to_str()
                {
                    found.push(name.to_string());
                }
            }
        }
        found.sort();
        if found.is_empty() {
            eprintln!("No packs found under {}", content_dir.display());
            std::process::exit(1);
        }
        found
    };

    let mut exit_code = 0;
    for pack_name in packs_to_check {
        let pack_dir = content_dir.join(&pack_name);
        let report = lint::lint_pack(&pack_dir, "en");
        for w in &report.warnings {
            println!("warning: {w}");
        }
        for e in &report.errors {
            eprintln!("error: {e}");
            exit_code = 1;
        }
    }
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
    Ok(())
}
