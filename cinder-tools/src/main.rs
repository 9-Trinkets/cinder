use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

mod builder;
mod hygiene;
mod lint;
mod scaffold;
mod transcript;

#[derive(Parser)]
#[command(name = "cinder-tools")]
#[command(about = "Developer and content authoring toolkit for Cinder", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build or export floor room layouts and maps
    Floor {
        #[command(subcommand)]
        cmd: FloorCommands,
    },
    /// Lint content pack reference integrity
    Lint {
        /// Optional pack name (e.g. 'layla', 'aera'). If omitted, all packs are checked.
        #[arg(short, long)]
        pack: Option<String>,
        /// Content root directory (default: content)
        #[arg(long, default_value = "content")]
        content_dir: PathBuf,
    },
    /// Scaffold a new content pack skeleton
    NewPack {
        /// Name of the new pack
        name: String,
        /// Content root directory (default: content)
        #[arg(long, default_value = "content")]
        content_dir: PathBuf,
    },
    /// Render readable transcripts from NDJSON run traces
    Transcript {
        /// Path to NDJSON file or run id
        target: Option<PathBuf>,
        /// Include raw workflow.complete block
        #[arg(long)]
        raw: bool,
    },
    /// Check codebase hygiene (file lengths and test placement)
    Hygiene {
        /// Soft line limit for non-JSON source files (default: 500)
        #[arg(long, default_value_t = 500)]
        limit: usize,
    },
}

#[derive(Subcommand)]
enum FloorCommands {
    /// Build a floor from a known preset (floor1, floor4)
    Build {
        /// Preset name: 'floor1' or 'floor4'
        #[arg(long)]
        preset: String,
        /// Target pack directory (e.g. content/layla)
        #[arg(long, default_value = "content/layla")]
        pack_dir: PathBuf,
        /// Locale to update (default: en)
        #[arg(long, default_value = "en")]
        locale: String,
        /// Export changes directly to rooms.json and maps.json
        #[arg(long)]
        export: bool,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Floor { cmd } => match cmd {
            FloorCommands::Build { preset, pack_dir, locale, export } => {
                match preset.as_str() {
                    "floor1" => {
                        let builder = builder::presets::build_floor1();
                        if export {
                            let (floor_count, total) = builder.export_to_pack(&pack_dir, &locale, |id| !id.starts_with('r'))?;
                            println!("Exported Floor 1 ({} rooms) to {}. Total rooms: {}", floor_count, pack_dir.display(), total);
                        } else {
                            let (rooms, map) = builder.build();
                            println!("Built Floor 1: {} rooms, map '{}' ({} rooms)", rooms.len(), map.label, map.rooms.len());
                        }
                    }
                    "floor4" => {
                        let builder = builder::presets::build_floor4();
                        if export {
                            let (floor_count, total) = builder.export_to_pack(&pack_dir, &locale, |id| {
                                id.starts_with('r') || id.starts_with('d') || (id.starts_with('o') && id != "old_drain_pipe")
                            })?;
                            println!("Exported Floor 4 ({} rooms) to {}. Total rooms: {}", floor_count, pack_dir.display(), total);
                        } else {
                            let (rooms, map) = builder.build();
                            println!("Built Floor 4: {} rooms, map '{}' ({} rooms)", rooms.len(), map.label, map.rooms.len());
                        }
                    }
                    other => {
                        eprintln!("Unknown preset: '{}'. Available presets: floor1, floor4", other);
                        std::process::exit(1);
                    }
                }
            }
        },
        Commands::Lint { pack, content_dir } => {
            let packs_to_check: Vec<String> = if let Some(p) = pack {
                vec![p]
            } else {
                let mut found = Vec::new();
                if let Ok(entries) = std::fs::read_dir(&content_dir) {
                    for entry in entries.flatten() {
                        if entry.path().is_dir()
                            && let Some(name) = entry.file_name().to_str()
                                && !name.starts_with('.') {
                                    found.push(name.to_string());
                                }
                    }
                }
                found.sort();
                found
            };

            let mut total_warnings = 0;
            let mut total_errors = 0;

            for pack_name in &packs_to_check {
                let pdir = content_dir.join(pack_name);
                let report = lint::lint_pack(&pdir, "en");
                for err in &report.errors {
                    eprintln!("error: {}", err);
                    total_errors += 1;
                }
                for warn in &report.warnings {
                    println!("warning: {}", warn);
                    total_warnings += 1;
                }
                if report.is_clean() {
                    println!("Pack '{}' is clean.", pack_name);
                }
            }

            println!("\nTotal errors: {}, warnings: {}", total_errors, total_warnings);
            if total_errors > 0 {
                std::process::exit(1);
            }
        }
        Commands::NewPack { name, content_dir } => {
            scaffold::scaffold_pack(&content_dir, &name)?;
            println!("Scaffolded new content pack at {}/{}", content_dir.display(), name);
        }
        Commands::Transcript { target, raw } => {
            let path = if let Some(t) = target {
                t
            } else {
                let runs_dir = Path::new(".cinder-state/runs");
                let mut files = Vec::new();
                if let Ok(entries) = std::fs::read_dir(runs_dir) {
                    for entry in entries.flatten() {
                        if entry.path().extension().is_some_and(|ext| ext == "ndjson") {
                            files.push(entry.path());
                        }
                    }
                }
                files.sort_by_key(|p| p.metadata().and_then(|m| m.modified()).ok());
                if let Some(latest) = files.last() {
                    latest.clone()
                } else {
                    eprintln!("No .ndjson files found in .cinder-state/runs");
                    std::process::exit(1);
                }
            };

            let output = transcript::render_transcript_file(&path, raw)?;
            println!("{}", output);
        }
        Commands::Hygiene { limit } => {
            let root = Path::new(".");
            let length_warnings = hygiene::check_file_lengths(root, limit);
            for w in &length_warnings {
                println!("warning: {} has {} lines, exceeding the {}-line soft limit", w.path.display(), w.lines, limit);
            }

            let test_warnings = hygiene::check_test_placement(root);
            for tw in &test_warnings {
                println!("warning: [{}] is an integration test suite under src/; move it to tests/", tw.display());
            }

            println!("\nFile length warnings: {}. Placement warnings: {}.", length_warnings.len(), test_warnings.len());
        }
    }

    Ok(())
}
