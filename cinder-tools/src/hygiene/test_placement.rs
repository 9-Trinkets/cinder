use std::fs;
use std::path::{Path, PathBuf};

const SRC_ROOTS: &[&str] = &["cinder-core/src", "cinder-srv/src", "cinder-tools/src"];
const SKIP_PATH_PARTS: &[&str] = &["target", "node_modules", ".git", "bin", "main.rs", "lib.rs"];

pub fn check_test_placement(root: &Path) -> Vec<PathBuf> {
    let mut misplaced = Vec::new();

    for src_rel in SRC_ROOTS {
        let base = root.join(src_rel);
        if !base.exists() {
            continue;
        }
        scan_src_dir(root, &base, &mut misplaced);
    }

    misplaced.sort();
    misplaced
}

fn scan_src_dir(root: &Path, current: &Path, out: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(current) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if path.is_dir() {
            if name == "tests" {
                let rel = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
                out.push(rel);
            } else if !SKIP_PATH_PARTS.contains(&name) {
                scan_src_dir(root, &path, out);
            }
        } else if path.is_file()
            && name == "tests.rs" {
                let rel = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
                out.push(rel);
            }
    }
}
