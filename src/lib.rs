pub mod tui;
pub mod content;
pub mod engine;

use std::error::Error;

pub fn run_cli(trace_events: bool, content_pack: &str) -> Result<(), Box<dyn Error>> {
    let pack = content::loader::load_named_pack(content_pack, None)?;
    let runtime = engine::runtime::CinderRuntime::new(pack, trace_events)?;
    tui::cli::run(runtime)
}
