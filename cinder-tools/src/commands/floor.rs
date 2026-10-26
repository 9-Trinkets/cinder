use std::path::PathBuf;

use crate::builder;

pub fn run_build(
    preset: String,
    pack_dir: PathBuf,
    locale: String,
    export: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    match preset.as_str() {
        "floor1" => {
            let builder = builder::presets::build_floor1();
            if export {
                let (floor_count, total) =
                    builder.export_to_pack(&pack_dir, &locale, |id| !id.starts_with('r'))?;
                println!(
                    "Exported Floor 1 ({} rooms) to {}. Total rooms: {}",
                    floor_count,
                    pack_dir.display(),
                    total
                );
            } else {
                let (rooms, map) = builder.build();
                println!(
                    "Built Floor 1: {} rooms, map '{}' ({} rooms)",
                    rooms.len(),
                    map.label,
                    map.rooms.len()
                );
            }
        }
        "floor4" => {
            let builder = builder::presets::build_floor4();
            if export {
                let (floor_count, total) = builder.export_to_pack(&pack_dir, &locale, |id| {
                    id.starts_with('r')
                        || id.starts_with('d')
                        || (id.starts_with('o') && id != "old_drain_pipe")
                })?;
                println!(
                    "Exported Floor 4 ({} rooms) to {}. Total rooms: {}",
                    floor_count,
                    pack_dir.display(),
                    total
                );
            } else {
                let (rooms, map) = builder.build();
                println!(
                    "Built Floor 4: {} rooms, map '{}' ({} rooms)",
                    rooms.len(),
                    map.label,
                    map.rooms.len()
                );
            }
        }
        other => {
            eprintln!(
                "Unknown preset: '{}'. Available presets: floor1, floor4",
                other
            );
            std::process::exit(1);
        }
    }
    Ok(())
}
