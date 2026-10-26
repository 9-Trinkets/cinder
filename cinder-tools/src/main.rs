use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod builder;
mod commands;
mod hygiene;
mod lint;
mod locale;
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
    /// Check codebase hygiene (file lengths, cyclomatic complexity, and test placement)
    Hygiene {
        /// Soft line limit for non-JSON source files (default: 500)
        #[arg(long, default_value_t = 500)]
        limit: usize,
        /// Cyclomatic complexity threshold per function (default: 15)
        #[arg(long, default_value_t = 15)]
        complexity: usize,
    },
    /// Scaffold, apply, or check content-pack locale translations
    Locale {
        #[command(subcommand)]
        cmd: LocaleCommands,
    },
}

#[derive(Subcommand)]
enum LocaleCommands {
    /// Duplicate a source locale's structure into a new locale and emit a worklist
    Scaffold {
        /// Pack directory (e.g. content/layla)
        #[arg(long, default_value = "content/layla")]
        pack_dir: PathBuf,
        /// Source locale code (default: en)
        #[arg(long, default_value = "en")]
        from: String,
        /// Target locale code (e.g. zh-TW)
        #[arg(long)]
        to: String,
        /// Overwrite the target locale, discarding existing files and translations
        #[arg(long)]
        force: bool,
    },
    /// Apply reviewed translations from the worklist into the locale files
    Apply {
        /// Pack directory (e.g. content/layla)
        #[arg(long, default_value = "content/layla")]
        pack_dir: PathBuf,
        /// Target locale code
        #[arg(long)]
        locale: String,
    },
    /// Report translation coverage for a locale
    Check {
        /// Pack directory (e.g. content/layla)
        #[arg(long, default_value = "content/layla")]
        pack_dir: PathBuf,
        /// Source locale code (default: en)
        #[arg(long, default_value = "en")]
        from: String,
        /// Target locale code
        #[arg(long)]
        locale: String,
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
            FloorCommands::Build {
                preset,
                pack_dir,
                locale,
                export,
            } => {
                commands::floor::run_build(preset, pack_dir, locale, export)?;
            }
        },
        Commands::Lint { pack, content_dir } => {
            commands::lint::run(pack, content_dir)?;
        }
        Commands::NewPack { name, content_dir } => {
            commands::newpack::run(name, content_dir)?;
        }
        Commands::Transcript { target, raw } => {
            commands::transcript::run(target, raw)?;
        }
        Commands::Hygiene { limit, complexity } => {
            commands::hygiene::run(limit, complexity);
        }
        Commands::Locale { cmd } => match cmd {
            LocaleCommands::Scaffold {
                pack_dir,
                from,
                to,
                force,
            } => {
                commands::locale::scaffold(pack_dir, from, to, force)?;
            }
            LocaleCommands::Apply { pack_dir, locale } => {
                commands::locale::apply(pack_dir, locale)?;
            }
            LocaleCommands::Check {
                pack_dir,
                from,
                locale,
            } => {
                commands::locale::check(pack_dir, from, locale)?;
            }
        },
    }

    Ok(())
}
