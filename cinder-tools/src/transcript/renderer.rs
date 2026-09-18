use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub fn render_transcript_file(path: &Path, include_raw: bool) -> Result<String, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut lines: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut turn_requests: HashMap<String, Value> = HashMap::new();
    let mut raw_blocks: Vec<String> = Vec::new();
    let mut run_id = path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();

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
            run_id = id.to_string();
        }

        let topic = event.get("topic").and_then(|v| v.as_str()).unwrap_or("");
        let role = event.get("sender_role").and_then(|v| v.as_str()).unwrap_or("");
        let payload = match event.get("payload") {
            Some(p) => p,
            None => continue,
        };

        if role == "actor_turn_decider" && topic == "model.request" {
            let request = payload.get("dialogue_request").cloned().unwrap_or(Value::Null);
            let actor_id = payload
                .get("actor_id")
                .or_else(|| request.get("actor_id"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !actor_id.is_empty() {
                turn_requests.insert(actor_id.to_string(), request);
            }
            continue;
        }

        if role == "actor_turn_decider" && topic == "model.response" {
            let actor_id = payload.get("actor_id").and_then(|v| v.as_str()).unwrap_or("");
            let actor_name = payload
                .get("actor_name")
                .and_then(|v| v.as_str())
                .unwrap_or(actor_id);
            let decision = payload.get("decision").and_then(|v| v.as_str()).unwrap_or("").trim();
            let req = turn_requests.get(actor_id);

            if decision == "MOVE" || decision.starts_with("MOVE ") {
                if let Some(room_title) = req.and_then(|r| r.get("move_target_room_title")).and_then(|v| v.as_str()) {
                    let text = format!("{actor_name} heads to the {room_title}.");
                    if seen.insert(text.clone()) {
                        lines.push(text);
                    }
                }
            } else if let Some(stripped) = decision.strip_prefix("ACT") {
                let action = stripped.trim_start_matches([' ', ':', '-', '—']).trim();
                if !action.is_empty() {
                    let mut norm = action.trim();
                    if norm.starts_with(actor_name) {
                        norm = norm[actor_name.len()..].trim_start();
                    }
                    let text = format!("{actor_name} {}.", norm.trim_end_matches(['.', '!', '?']));
                    if seen.insert(text.clone()) {
                        lines.push(text);
                    }
                }
            }
            continue;
        }

        if role == "actor_dialogue" && topic == "model.response" {
            let actor_name = payload.get("actor_name").and_then(|v| v.as_str()).unwrap_or("");
            let resp = payload.get("response_text").and_then(|v| v.as_str()).unwrap_or("").trim();
            if !actor_name.is_empty() && !resp.is_empty() {
                let text = format!("{actor_name}: {resp}");
                if seen.insert(text.clone()) {
                    lines.push(text);
                }
            }
            continue;
        }

        if topic == "workflow.complete"
            && let Some(text) = payload.get("text").and_then(|v| v.as_str()) {
                raw_blocks.push(text.to_string());
                for l in text.lines() {
                    let s = l.trim();
                    if !s.is_empty() && seen.insert(s.to_string()) {
                        lines.push(s.to_string());
                    }
                }
            }
    }

    let mut output = Vec::new();
    output.push(format!("=== {run_id} ==="));
    output.push(format!("Source: {}", path.display()));
    output.push(String::new());

    if lines.is_empty() {
        output.push("(no dialogue, movement, or action lines reconstructed)".to_string());
    } else {
        output.extend(lines);
    }

    if include_raw && !raw_blocks.is_empty() {
        output.push(String::new());
        output.push("--- raw workflow.complete ---".to_string());
        output.extend(raw_blocks);
    }

    Ok(output.join("\n"))
}
