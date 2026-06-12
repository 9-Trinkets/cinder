mod cli;
mod effects;
mod input;
mod projector;
mod render;
mod theme;
mod transcript;

use clap::Parser;
use std::error::Error;

#[derive(Debug, Parser)]
#[command(name = "cinder")]
struct Cli {
    #[arg(long, default_value_t = false)]
    trace_events: bool,

    #[arg(long, default_value = "ella")]
    content: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let pack = cinder_core::content::loader::load_named_pack(&cli.content, None)?;
    let runtime = cinder_core::CinderRuntime::new(pack, cli.trace_events)?;
    cli::run(runtime)
}
