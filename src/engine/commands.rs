use crate::content::types::{ActorDefinition, CommandDefinition, ContentPack};
use crate::engine::state::WorldState;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TurnAction {
    Look,
    Move,
    MoveTo {
        room_id: String,
    },
    Command {
        command_id: String,
        target_room_id: Option<String>,
        target_actor_id: Option<String>,
        feature_id: Option<String>,
        consumable_id: Option<String>,
        context_label: Option<String>,
        freeform_text: Option<String>,
    },
    Help,
    Quit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum PlayerCommand {
    Authored {
        command_id: String,
        input: Option<String>,
    },
    Help,
    Quit,
    Unknown,
}

#[derive(Debug)]
pub(crate) struct ResolvedActorReferenceInput {
    pub actor_id: String,
    pub actor_name: String,
    pub player_message: Option<String>,
    pub actor_in_room: bool,
}

pub(crate) fn parse_command(content: &ContentPack, raw_input: &str) -> PlayerCommand {
    let trimmed = raw_input.trim();
    let lower = trimmed.to_ascii_lowercase();
    match lower.as_str() {
        "help" | "h" | "?" => return PlayerCommand::Help,
        "quit" | "exit" => return PlayerCommand::Quit,
        _ => {}
    }

    if let Some((command, matched_phrase)) = best_player_command_match(content, trimmed, &lower) {
        return PlayerCommand::Authored {
            command_id: command.id.clone(),
            input: matched_phrase.remainder,
        };
    }

    PlayerCommand::Unknown
}

pub(crate) fn player_command_help_text(content: &ContentPack) -> String {
    player_command_help_lines(content).join("\n")
}

pub(crate) fn player_command_suggestions(content: &ContentPack) -> String {
    let mut suggestions = player_command_examples(content);
    suggestions.push("help".to_string());
    suggestions.push("quit".to_string());
    suggestions.join(", ")
}

pub(crate) fn resolve_actor_reference_input(
    content: &ContentPack,
    state: &WorldState,
    current_room_id: &str,
    remainder: &str,
) -> Option<ResolvedActorReferenceInput> {
    match_actor_reference(
        content
            .actors
            .iter()
            .filter(|actor| state.actor_room_id(&actor.id, &actor.room_id) == current_room_id),
        remainder,
    )
    .map(|(actor, player_message)| ResolvedActorReferenceInput {
        actor_id: actor.id.clone(),
        actor_name: actor.name.clone(),
        player_message,
        actor_in_room: true,
    })
    .or_else(|| {
        match_actor_reference(content.actors.iter(), remainder).map(|(actor, player_message)| {
            ResolvedActorReferenceInput {
                actor_id: actor.id.clone(),
                actor_name: actor.name.clone(),
                player_message,
                actor_in_room: state.actor_room_id(&actor.id, &actor.room_id) == current_room_id,
            }
        })
    })
}

pub(crate) fn unknown_target_token(remainder: &str) -> String {
    remainder
        .split_whitespace()
        .next()
        .unwrap_or(remainder)
        .trim()
        .to_string()
}

#[derive(Debug, Clone)]
struct PlayerPhraseMatch {
    remainder: Option<String>,
}

fn best_player_command_match<'a>(
    content: &'a ContentPack,
    trimmed: &str,
    lower: &str,
) -> Option<(&'a CommandDefinition, PlayerPhraseMatch)> {
    let mut best: Option<(&CommandDefinition, (usize, usize), PlayerPhraseMatch)> = None;
    for command in &content.commands.actions {
        let Some(metadata) = command.player_command.as_ref() else {
            continue;
        };
        if !command.player_enabled {
            continue;
        }
        let input_metadata = metadata.input.as_ref();
        let requires_input = input_metadata.is_some_and(|input| input.required);
        let accepts_input = input_metadata.is_some();
        for phrase in &command.player_phrases {
            let phrase_trimmed = phrase.trim();
            if phrase_trimmed.is_empty() {
                continue;
            }
            let phrase_lower = phrase_trimmed.to_ascii_lowercase();
            let matched = if lower == phrase_lower {
                if requires_input {
                    None
                } else {
                    Some(PlayerPhraseMatch { remainder: None })
                }
            } else {
                if !accepts_input {
                    None
                } else {
                    lower
                        .strip_prefix(&phrase_lower)
                        .and_then(|rest| rest.strip_prefix(' '))
                        .and_then(|rest| {
                            let remainder =
                                trimmed[(trimmed.len() - rest.len())..].trim().to_string();
                            if remainder.is_empty() && requires_input {
                                None
                            } else {
                                Some(PlayerPhraseMatch {
                                    remainder: Some(remainder),
                                })
                            }
                        })
                }
            };
            if let Some(matched) = matched {
                let score = (
                    phrase_trimmed.len(),
                    usize::from(matched.remainder.is_some() && accepts_input),
                );
                if best
                    .as_ref()
                    .is_none_or(|(_, best_score, _)| score > *best_score)
                {
                    best = Some((command, score, matched));
                }
            }
        }
    }
    best.map(|(command, _, matched)| (command, matched))
}

fn player_command_help_lines(content: &ContentPack) -> Vec<String> {
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for command in content.player_commands() {
        let Some(metadata) = &command.player_command else {
            continue;
        };
        if metadata.usage.is_empty() {
            continue;
        }
        let group = if command.group.is_empty() {
            "general"
        } else {
            command.group.as_str()
        };
        let line = format!("- {}", metadata.usage);
        groups.entry(group.to_string()).or_default().push(line);
    }
    let order = [
        "observation",
        "conversation",
        "book",
        "service",
        "session",
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
            "session" => "Session",
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
    for command in content.player_commands() {
        let Some(metadata) = &command.player_command else {
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

fn match_actor_reference<'a>(
    actors: impl IntoIterator<Item = &'a ActorDefinition>,
    remainder: &str,
) -> Option<(&'a ActorDefinition, Option<String>)> {
    let trimmed = remainder.trim();
    let lower = trimmed.to_ascii_lowercase();
    let mut best: Option<(&'a ActorDefinition, usize, Option<String>)> = None;
    for actor in actors {
        for reference in actor_references(actor) {
            let reference_lower = reference.to_ascii_lowercase();
            let exact = lower == reference_lower;
            let prefix = lower
                .strip_prefix(&reference_lower)
                .and_then(|rest| rest.strip_prefix(' '));
            if exact || prefix.is_some() {
                let player_message = if exact {
                    None
                } else {
                    let tail = trimmed[reference.len()..].trim();
                    if tail.is_empty() {
                        None
                    } else {
                        Some(tail.to_string())
                    }
                };
                let reference_len = reference.len();
                if best
                    .as_ref()
                    .is_none_or(|(_, best_len, _)| reference_len > *best_len)
                {
                    best = Some((actor, reference_len, player_message));
                }
            }
        }
    }
    best.map(|(actor, _, player_message)| (actor, player_message))
}

fn actor_references(actor: &ActorDefinition) -> Vec<&str> {
    let mut refs = vec![actor.name.as_str(), actor.id.as_str()];
    refs.extend(actor.aliases.iter().map(String::as_str));
    refs
}
