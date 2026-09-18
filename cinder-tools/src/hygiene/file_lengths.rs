use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

const SOURCE_EXTS: &[&str] = &[".rs", ".ts", ".tsx", ".js", ".jsx", ".go", ".py"];
const SKIP_DIRS: &[&str] = &["target", "node_modules", ".git", "dist", "build", ".vercel", "data"];

pub struct FileLengthWarning {
    pub path: PathBuf,
    pub lines: usize,
}

pub fn check_file_lengths(root: &Path, limit: usize) -> Vec<FileLengthWarning> {
    let mut warnings = Vec::new();
    scan_dir(root, root, limit, &mut warnings);
    warnings.sort_by(|a, b| b.lines.cmp(&a.lines));
    warnings
}

fn scan_dir(root: &Path, current: &Path, limit: usize, out: &mut Vec<FileLengthWarning>) {
    let entries = match fs::read_dir(current) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !SKIP_DIRS.contains(&name) {
                scan_dir(root, &path, limit, out);
            }
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let dotted = format!(".{ext}");
            if SOURCE_EXTS.contains(&dotted.as_str())
                && let Ok(file) = fs::File::open(&path) {
                    let lines = BufReader::new(file).lines().count();
                    if lines > limit {
                        let rel = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
                        out.push(FileLengthWarning { path: rel, lines });
                    }
                }
        }
    }
}
