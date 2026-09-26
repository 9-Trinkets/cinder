#[derive(serde::Deserialize)]
struct DescentCommentaryJson {
    #[serde(default)]
    summary: String,
    #[serde(default)]
    introduction: String,
}

pub(crate) fn parse_descent_commentary_response(raw: &str) -> Vec<String> {
    let trimmed = raw.trim();
    let json_str = if let Some(stripped) = trimmed.strip_prefix("```json") {
        stripped.strip_suffix("```").unwrap_or(stripped).trim()
    } else if let Some(stripped) = trimmed.strip_prefix("```") {
        stripped.strip_suffix("```").unwrap_or(stripped).trim()
    } else {
        trimmed
    };

    if let Ok(parsed) = serde_json::from_str::<DescentCommentaryJson>(json_str) {
        let mut messages = Vec::new();
        let summary = parsed.summary.trim().trim_matches('"').trim();
        if !summary.is_empty() {
            messages.push(summary.to_string());
        }
        let intro = parsed.introduction.trim().trim_matches('"').trim();
        if !intro.is_empty() {
            messages.push(intro.to_string());
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