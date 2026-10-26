use std::fs;
use std::path::PathBuf;

use crate::transcript;

pub fn run(target: Option<PathBuf>, raw: bool) -> Result<(), Box<dyn std::error::Error>> {
    let path = match target {
        Some(p) => p,
        None => {
            let mut files: Vec<PathBuf> = fs::read_dir(".cinder-state/runs")?
                .filter_map(|entry| {
                    let entry = entry.ok()?;
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("ndjson") {
                        Some(path)
                    } else {
                        None
                    }
                })
                .collect();
            files.sort_by_key(|p| p.metadata().and_then(|m| m.modified()).ok());
            if let Some(latest) = files.last() {
                latest.clone()
            } else {
                eprintln!("No .ndjson files found in .cinder-state/runs");
                std::process::exit(1);
            }
        }
    };

    let output = transcript::render_transcript_file(&path, raw)?;
    println!("{}", output);
    Ok(())
}
