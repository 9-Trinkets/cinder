pub mod tui;

use std::error::Error;

pub fn run_cli(trace_events: bool, content_pack: &str) -> Result<(), Box<dyn Error>> {
    let pack = cinder_core::content::loader::load_named_pack(content_pack, None)?;
    let runtime = cinder_core::CinderRuntime::new(pack, trace_events)?;
    tui::cli::run(runtime)
}
