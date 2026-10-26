use std::path::PathBuf;

use crate::scaffold;

pub fn run(name: String, content_dir: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    scaffold::scaffold_pack(&content_dir, &name)?;
    let dir = content_dir.join(&name);
    println!("Created pack skeleton at {}", dir.display());
    Ok(())
}
