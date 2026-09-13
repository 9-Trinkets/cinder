use std::path::PathBuf;

/// Returns the project root directory.
///
/// Checks the runtime `CINDER_PROJECT_DIR` environment variable first,
/// falling back to the compile-time `CINDER_PROJECT_DIR` embedded by `build.rs`.
pub fn project_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("CINDER_PROJECT_DIR") {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    PathBuf::from(env!("CINDER_PROJECT_DIR"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_dir_default_or_env() {
        let dir = project_dir();
        assert!(!dir.as_os_str().is_empty());
    }
}
