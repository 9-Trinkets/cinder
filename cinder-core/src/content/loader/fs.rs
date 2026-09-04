use crate::content::loader::DEFAULT_LOCALE;
use serde::de::DeserializeOwned;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

pub fn read_optional_json<T: DeserializeOwned>(
    path: &Path,
    file_name: &str,
) -> Result<Option<T>, Box<dyn Error>> {
    read_optional_path(&path.join(file_name))
}

pub fn read_optional_json_raw(path: &Path, file_name: &str) -> Result<Option<String>, Box<dyn Error>> {
    let file_path = path.join(file_name);
    if file_path.exists() {
        Ok(Some(fs::read_to_string(&file_path)?))
    } else {
        Ok(None)
    }
}

pub fn read_json<T: DeserializeOwned>(path: &Path, file_name: &str) -> Result<T, Box<dyn Error>> {
    read_required_path(&path.join(file_name))
}

pub fn localized_file_path(path: &Path, locale: &str, file_name: &str) -> PathBuf {
    let localized = path.join("locales").join(locale).join(file_name);
    if localized.exists() {
        return localized;
    }
    let default_localized = path.join("locales").join(DEFAULT_LOCALE).join(file_name);
    if default_localized.exists() {
        return default_localized;
    }
    path.join(file_name)
}

pub fn read_optional_path<T: DeserializeOwned>(path: &Path) -> Result<Option<T>, Box<dyn Error>> {
    match fs::read_to_string(path) {
        Ok(contents) => Ok(Some(serde_json::from_str(&contents)?)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub fn read_required_path<T: DeserializeOwned>(path: &Path) -> Result<T, Box<dyn Error>> {
    let contents = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&contents)?)
}

pub struct LocalizedPaths<'a> {
    pub root: &'a Path,
    pub locale: &'a str,
}

impl<'a> LocalizedPaths<'a> {
    pub fn new(root: &'a Path, locale: &'a str) -> Self {
        Self { root, locale }
    }

    pub fn read_optional<T: DeserializeOwned>(
        &self,
        file_name: &str,
    ) -> Result<Option<T>, Box<dyn Error>> {
        read_optional_path(&localized_file_path(self.root, self.locale, file_name))
    }

    pub fn read_required<T: DeserializeOwned>(&self, file_name: &str) -> Result<T, Box<dyn Error>> {
        read_required_path(&localized_file_path(self.root, self.locale, file_name))
    }

}
