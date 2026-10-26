mod event;
mod renderers;
mod state;

use event::EventFields;
use renderers::RENDERERS;
use serde_json::Value;
use state::RenderState;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub fn render_transcript_file(
    path: &Path,
    include_raw: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let run_id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();
    let mut state = RenderState::new(run_id);

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let event: Value = match serde_json::from_str(trimmed) {
            Ok(val) => val,
            Err(_) => continue,
        };

        if let Some(id) = event.get("run_id").and_then(|v| v.as_str()) {
            state.run_id = id.to_string();
        }

        let Some(payload) = event.get("payload") else {
            continue;
        };
        let fields = EventFields {
            role: event
                .get("sender_role")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
            topic: event.get("topic").and_then(|v| v.as_str()).unwrap_or(""),
            payload,
        };

        if let Some(renderer) = RENDERERS.iter().find(|r| r.matches(&fields)) {
            renderer.render(&mut state, &fields);
        }
    }

    Ok(assemble(&state, path, include_raw))
}

fn assemble(state: &RenderState, path: &Path, include_raw: bool) -> String {
    let mut output = vec![
        format!("=== {} ===", state.run_id),
        format!("Source: {}", path.display()),
        String::new(),
    ];

    if state.lines.is_empty() {
        output.push("(no dialogue, movement, or action lines reconstructed)".to_string());
    } else {
        output.extend(state.lines.iter().cloned());
    }

    if include_raw && !state.raw_blocks.is_empty() {
        output.push(String::new());
        output.push("--- raw workflow.complete ---".to_string());
        output.extend(state.raw_blocks.iter().cloned());
    }

    output.join("\n")
}
