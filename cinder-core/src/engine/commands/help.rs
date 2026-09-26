use crate::content::types::ContentPack;
use std::collections::BTreeMap;

pub(crate) fn player_command_help_text(content: &ContentPack) -> String {
    player_command_help_lines(content).join("\n")
}

pub(crate) fn player_command_suggestions(content: &ContentPack) -> String {
    let mut suggestions = player_command_examples(content);
    suggestions.push("help".to_string());
    suggestions.push("quit".to_string());
    suggestions.join(", ")
}

fn player_command_help_lines(content: &ContentPack) -> Vec<String> {
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();

    if !content.settings.party.initial_orders.is_empty() {
        let directives = content.settings.party.directives();
        let usage = if directives.is_empty() {
            "<directive>".to_string()
        } else {
            directives.join("|")
        };
        groups
            .entry("general".to_string())
            .or_default()
            .push(format!("- order <party member> {usage}"));
    }

    for action in content
        .actions
        .iter()
        .filter(|a| a.player_enabled && !a.phrases.is_empty())
    {
        let Some(metadata) = &action.player_command else {
            continue;
        };
        if metadata.usage.is_empty() {
            continue;
        }
        let group = if action.group.is_empty() {
            "general"
        } else {
            action.group.as_str()
        };
        let line = format!("- {}", metadata.usage);
        groups.entry(group.to_string()).or_default().push(line);
    }
    let order = [
        "observation",
        "conversation",
        "book",
        "service",
        "act",
        "general",
    ];
    let mut lines = Vec::new();
    for group in order {
        let Some(entries) = groups.remove(group) else {
            continue;
        };
        if !lines.is_empty() {
            lines.push(String::new());
        }
        let label = match group {
            "observation" => "Observation",
            "conversation" => "Conversation",
            "book" => "Book",
            "service" => "Service",
            "act" => "Act",
            _ => "General",
        };
        lines.push(format!("— {} —", label));
        for line in entries {
            if !lines.contains(&line) {
                lines.push(line);
            }
        }
    }
    lines
}

fn player_command_examples(content: &ContentPack) -> Vec<String> {
    let mut examples = Vec::new();
    for action in content
        .actions
        .iter()
        .filter(|a| a.player_enabled && !a.phrases.is_empty())
    {
        let Some(metadata) = &action.player_command else {
            continue;
        };
        if metadata.example.is_empty() {
            continue;
        }
        if !examples.contains(&metadata.example) {
            examples.push(metadata.example.clone());
        }
    }
    examples
}