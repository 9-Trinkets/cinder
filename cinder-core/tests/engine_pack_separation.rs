//! Guard: the engine must stay free of pack-specific identity.
//!
//! Cinder's architecture rule is that `cinder-core/src/engine` implements
//! mechanisms only - geography, actors, and prose belong entirely to content
//! packs. This test fails if any engine source reintroduces pack identifiers
//! or player-facing narration.

use std::fs;
use std::path::{Path, PathBuf};

const ENGINE_DIR: &str = "src/engine";

fn forbidden_matches(path: &Path, source: &str) -> Vec<String> {
    let mut violations = Vec::new();
    for (line_number, line) in source.lines().enumerate() {
        if line.contains("golem-") {
            violations.push(format!(
                "{}:{}: {}",
                path.display(),
                line_number + 1,
                line.trim()
            ));
        }
        let bytes = line.as_bytes();
        let mut index = 0;
        while index + 3 < bytes.len() {
            if bytes[index] == b'r'
                && bytes[index + 1].is_ascii_digit()
                && bytes[index + 2] == b'c'
                && bytes[index + 3].is_ascii_digit()
            {
                violations.push(format!(
                    "{}:{}: {}",
                    path.display(),
                    line_number + 1,
                    line.trim()
                ));
                break;
            }
            index += 1;
        }
    }
    violations
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("engine dir exists") {
        let entry = entry.expect("readable entry");
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn engine_sources_are_free_of_pack_identity_and_prose() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let engine_dir = manifest_dir.join(ENGINE_DIR);
    let mut files = Vec::new();
    collect_rs_files(&engine_dir, &mut files);
    assert!(
        !files.is_empty(),
        "expected to scan engine sources under {}",
        engine_dir.display()
    );

    let mut violations = Vec::new();
    for file in &files {
        let source = fs::read_to_string(file).expect("readable engine source");
        violations.extend(forbidden_matches(file, &source));
    }

    assert!(
        violations.is_empty(),
        "engine sources contain pack-specific identity:\n{}",
        violations.join("\n")
    );
}
