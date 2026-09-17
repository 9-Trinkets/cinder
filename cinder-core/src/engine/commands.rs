mod items;

use crate::content::types::{
    ActionDefinition, ActorDefinition, CommandEffect, ContentPack, PartyOrderKind,
};
use crate::engine::state::{WorldState, current_cast_member_actor_id, display_actor_name};
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
    /// A generic `take <item>` / `pick up <item>` / `get <item>` command. The
    /// bare target is resolved against loose items in the current room by the
    /// planner, independent of any per-item content action.
    Take {
        target: String,
    },
    /// A generic `drop <item>` command. The bare target is resolved against
    /// the player's inventory by the planner, independent of any per-item
    /// content action.
    Drop {
        target: String,
    },
    /// A generic `equip <item>` command resolved against inventory.
    Equip {
        target: String,
    },
    /// A generic `unequip <item>` command resolved against equipped items.
    Unequip {
        target: String,
    },
    PartyOrder {
        actor_reference: String,
        order: PartyOrderKind,
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

    if !content.actions.is_empty() {
        if let Some((action, matched_phrase)) = best_player_action_match(content, trimmed, &lower) {
            if (action.id == "take" || action.has_effect(CommandEffect::PickUpItem))
                && action.item_id.is_empty()
            {
                if let Some(remainder) = matched_phrase.remainder.as_deref() {
                    if let Some(target) = remainder.strip_prefix("off ") {
                        return PlayerCommand::Unequip {
                            target: target.trim().to_string(),
                        };
                    }
                }
                return PlayerCommand::Take {
                    target: matched_phrase.remainder.unwrap_or_default(),
                };
            }
            if (action.id == "drop" || action.has_effect(CommandEffect::DropItem))
                && action.item_id.is_empty()
            {
                return PlayerCommand::Drop {
                    target: matched_phrase.remainder.unwrap_or_default(),
                };
            }
            if (action.id == "equip" || action.has_effect(CommandEffect::EquipItem))
                && action.item_id.is_empty()
            {
                return PlayerCommand::Equip {
                    target: matched_phrase.remainder.unwrap_or_default(),
                };
            }
            if (action.id == "unequip" || action.has_effect(CommandEffect::UnequipItem))
                && action.item_id.is_empty()
            {
                return PlayerCommand::Unequip {
                    target: matched_phrase.remainder.unwrap_or_default(),
                };
            }
            return PlayerCommand::Authored {
                command_id: action.id.clone(),
                input: matched_phrase.remainder,
            };
        }
        // Fallback: match by action ID directly (used by web UI overflow actions)
        for action in &content.actions {
            if action.player_enabled && action.id.to_ascii_lowercase() == lower {
                if (action.id == "take" || action.has_effect(CommandEffect::PickUpItem))
                    && action.item_id.is_empty()
                {
                    return PlayerCommand::Take {
                        target: String::new(),
                    };
                }
                if (action.id == "drop" || action.has_effect(CommandEffect::DropItem))
                    && action.item_id.is_empty()
                {
                    return PlayerCommand::Drop {
                        target: String::new(),
                    };
                }
                if (action.id == "equip" || action.has_effect(CommandEffect::EquipItem))
                    && action.item_id.is_empty()
                {
                    return PlayerCommand::Equip {
                        target: String::new(),
                    };
                }
                if (action.id == "unequip" || action.has_effect(CommandEffect::UnequipItem))
                    && action.item_id.is_empty()
                {
                    return PlayerCommand::Unequip {
                        target: String::new(),
                    };
                }
                return PlayerCommand::Authored {
                    command_id: action.id.clone(),
                    input: None,
                };
            }
        }
    }

    // Generic item commands are checked after authored actions so packs can
    // retain custom phrases while using the shared engine flow by default.
    if let Some(command) = items::parse_item_command(trimmed) {
        let is_allowed = match &command {
            PlayerCommand::Take { .. } => content.player_can_take_items(),
            PlayerCommand::Drop { .. } => content.player_can_drop_items(),
            _ => true,
        };
        if is_allowed {
            return command;
        }
    }
    if let Some((actor_reference, order)) = party_order_phrase(trimmed) {
        return PlayerCommand::PartyOrder {
            actor_reference,
            order,
        };
    }

    PlayerCommand::Unknown
}

/// A pack-defined party directive assigned to a member (`order <member>
/// <directive>`). Any single-word directive id is accepted; whether it does
/// anything reflects the pack's `OrderIs` combat rules.
fn party_order_phrase(trimmed: &str) -> Option<(String, PartyOrderKind)> {
    if !trimmed.to_ascii_lowercase().starts_with("order ") {
        return None;
    }
    let remainder = &trimmed["order ".len()..];
    let (actor_reference, order) = remainder.rsplit_once(' ')?;
    let order = order.trim().to_ascii_lowercase();
    if order.is_empty() {
        return None;
    }
    let actor_reference = actor_reference.trim();
    (!actor_reference.is_empty()).then(|| (actor_reference.to_string(), order))
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
        state,
        content.actors.iter().filter(|actor| {
            !content.is_player_actor(&actor.id)
                && !state.actor_is_defeated(&actor.id, &content.settings.combat.health_stat_id)
                && state.actor_is_in_room(content, &actor.id, current_room_id)
        }),
        remainder,
        &content.settings.act_member_alias,
    )
    .map(|(actor, player_message)| ResolvedActorReferenceInput {
        actor_id: actor.id.clone(),
        actor_name: display_actor_name(state, actor),
        player_message,
        actor_in_room: true,
    })
    .or_else(|| {
        match_actor_reference(
            state,
            content.actors.iter().filter(|actor| {
                !content.is_player_actor(&actor.id)
                    && !state.actor_is_defeated(&actor.id, &content.settings.combat.health_stat_id)
            }),
            remainder,
            &content.settings.act_member_alias,
        )
        .map(|(actor, player_message)| ResolvedActorReferenceInput {
            actor_id: actor.id.clone(),
            actor_name: display_actor_name(state, actor),
            player_message,
            actor_in_room: state.actor_is_in_room(content, &actor.id, current_room_id),
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

fn best_player_action_match<'a>(
    content: &'a ContentPack,
    trimmed: &str,
    lower: &str,
) -> Option<(&'a ActionDefinition, PlayerPhraseMatch)> {
    let mut best: Option<(&ActionDefinition, (usize, usize), PlayerPhraseMatch)> = None;
    for action in &content.actions {
        let Some(metadata) = action.player_command.as_ref() else {
            continue;
        };
        if !action.player_enabled {
            continue;
        }
        let input_metadata = metadata.input.as_ref();
        let requires_input = input_metadata.is_some_and(|input| input.required);
        let accepts_input = input_metadata.is_some();
        for phrase in &action.phrases {
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
                    best = Some((action, score, matched));
                }
            }
        }
    }
    best.map(|(action, _, matched)| (action, matched))
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

fn match_actor_reference<'a>(
    state: &WorldState,
    actors: impl IntoIterator<Item = &'a ActorDefinition>,
    remainder: &str,
    act_member_alias: &str,
) -> Option<(&'a ActorDefinition, Option<String>)> {
    let trimmed = remainder.trim();
    let lower = trimmed.to_ascii_lowercase();
    let mut best: Option<(&'a ActorDefinition, usize, Option<String>)> = None;
    for actor in actors {
        for reference in actor_references(state, actor, act_member_alias) {
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

fn actor_references(
    state: &WorldState,
    actor: &ActorDefinition,
    act_member_alias: &str,
) -> Vec<String> {
    let mut refs = vec![
        actor.name.clone(),
        actor.id.clone(),
        display_actor_name(state, actor),
    ];
    if current_cast_member_actor_id(state).is_some_and(|actor_id| actor_id == actor.id)
        && !act_member_alias.is_empty()
    {
        refs.push(act_member_alias.to_string());
    }
    refs.extend(actor.aliases.iter().cloned());
    refs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::test_fixtures::minimal_test_pack;

    fn parse(raw: &str) -> PlayerCommand {
        parse_command(&minimal_test_pack(), raw)
    }

    #[test]
    fn party_order_phrases_resolve_to_single_member_orders() {
        assert!(matches!(
            parse("order blair guard"),
            PlayerCommand::PartyOrder {
                actor_reference,
                order,
            } if actor_reference == "blair" && order == "guard"
        ));
        assert!(matches!(
            parse("order dark golem 1 assist"),
            PlayerCommand::PartyOrder {
                actor_reference,
                order,
            } if actor_reference == "dark golem 1" && order == "assist"
        ));
        // Directives are pack-defined; any single word is accepted verbatim.
        assert!(matches!(
            parse("order blair rally"),
            PlayerCommand::PartyOrder {
                actor_reference,
                order,
            } if actor_reference == "blair" && order == "rally"
        ));
    }

}
