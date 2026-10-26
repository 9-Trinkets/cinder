use std::path::PathBuf;

use crate::locale;

pub fn scaffold(
    pack_dir: PathBuf,
    from: String,
    to: String,
    force: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    locale::scaffold::scaffold(&pack_dir, &from, &to, force)?;
    Ok(())
}

pub fn apply(pack_dir: PathBuf, locale_code: String) -> Result<(), Box<dyn std::error::Error>> {
    locale::apply::apply(&pack_dir, &locale_code)?;
    Ok(())
}

pub fn check(
    pack_dir: PathBuf,
    from: String,
    locale_code: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut pending = locale::apply::check(&pack_dir, &from, &locale_code)?;
    pending += locale::overlay_check::check_overlays(&pack_dir, &locale_code)?;
    if pending > 0 {
        std::process::exit(1);
    }
    Ok(())
}
