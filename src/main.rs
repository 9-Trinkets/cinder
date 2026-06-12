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
    cinder::run_cli(cli.trace_events, &cli.content)
}
