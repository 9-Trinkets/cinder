/// Extract generated transition-commentary lines from an LLM response.
///
/// The pack's prompt template dictates the output contract, so this parser
/// stays shape-agnostic: any JSON object of string values is accepted (each
/// value becomes one message), JSON arrays of strings work too, and non-JSON
/// responses fall back to a plaintext paragraph split. Nothing is hardcoded
/// about the pack's room keys or persona. The only convention kept is the
/// ordering of the widely used "summary"/"introduction" pair so that packs
/// which follow it (the engine's own default template does) get their summary
/// line first.
pub(crate) fn parse_transition_commentary_response(raw: &str) -> Vec<String> {
    let trimmed = raw.trim();
    let json_str = if let Some(stripped) = trimmed.strip_prefix("```json") {
        stripped.strip_suffix("```").unwrap_or(stripped).trim()
    } else if let Some(stripped) = trimmed.strip_prefix("```") {
        stripped.strip_suffix("```").unwrap_or(stripped).trim()
    } else {
        trimmed
    };

    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
        let mut messages = Vec::new();
        match parsed {
            serde_json::Value::Object(entries) => {
                let summary = entries.get("summary").and_then(|v| v.as_str());
                let introduction = entries.get("introduction").and_then(|v| v.as_str());
                if summary.is_some() || introduction.is_some() {
                    if let Some(text) = summary {
                        push_clean(&mut messages, text);
                    }
                    if let Some(text) = introduction {
                        push_clean(&mut messages, text);
                    }
                    // Ignore any other keys a model may have added.
                } else {
                    for (_, value) in entries {
                        if let serde_json::Value::String(text) = value {
                            push_clean(&mut messages, &text);
                        }
                    }
                }
            }
            serde_json::Value::Array(entries) => {
                for value in entries {
                    if let serde_json::Value::String(text) = value {
                        push_clean(&mut messages, &text);
                    }
                }
            }
            _ => {}
        }
        if !messages.is_empty() {
            return messages;
        }
    }

    let parts: Vec<String> = trimmed
        .split("\n\n")
        .map(|p| p.trim().trim_matches('"').trim().to_string())
        .filter(|p| !p.is_empty())
        .collect();
    if !parts.is_empty() {
        parts
    } else if !trimmed.is_empty() {
        vec![trimmed.to_string()]
    } else {
        Vec::new()
    }
}

fn push_clean(messages: &mut Vec<String>, text: &str) {
    let cleaned = text.trim().trim_matches('"').trim();
    if !cleaned.is_empty() {
        messages.push(cleaned.to_string());
    }
}