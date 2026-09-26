use super::{PlayerCommand, items};
use crate::content::types::{ActionDefinition, CommandEffect, ContentPack, PartyOrderKind};

pub(crate) fn parse_command(content: &ContentPack, raw_input: &str) -> PlayerCommand {
    let trimmed = raw_input.trim();
    let lower = trimmed.to_ascii_lowercase();
    match lower.as_str() {
        "help" | "h" | "?" => return PlayerCommand::Help,
        "quit" | "exit" => return PlayerCommand::Quit,
        _ => {}
    }

    if lower.starts_with("switch-room:") {
        let room_id = trimmed["switch-room:".len()..].trim();
        return PlayerCommand::Authored {
            command_id: "move".to_string(),
            input: Some(room_id.to_string()),
        };
    }

    if let Some(cmd) = parse_follow_command(trimmed) {
        return cmd;
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
                    let lower_remainder = remainder.to_ascii_lowercase();
                    if let Some(idx) = lower_remainder.find(" from ") {
                        let item_target = remainder[..idx].trim().to_string();
                        let actor_reference = remainder[idx + 6..].trim().to_string();
                        if !item_target.is_empty() && !actor_reference.is_empty() {
                            return PlayerCommand::TakeFromPartyMember {
                                item_target,
                                actor_reference,
                            };
                        }
                    }
                }
                return PlayerCommand::Take {
                    target: matched_phrase.remainder.unwrap_or_default(),
                };
            }
            if action.id == "give" {
                if let Some(remainder) = matched_phrase.remainder.as_deref() {
                    let lower_remainder = remainder.to_ascii_lowercase();
                    if let Some(idx) = lower_remainder.find(" to ") {
                        let item_target = remainder[..idx].trim().to_string();
                        let actor_reference = remainder[idx + 4..].trim().to_string();
                        return PlayerCommand::GiveToPartyMember {
                            item_target,
                            actor_reference,
                        };
                    } else {
                        return PlayerCommand::GiveToPartyMember {
                            item_target: remainder.trim().to_string(),
                            actor_reference: String::new(),
                        };
                    }
                }
                return PlayerCommand::GiveToPartyMember {
                    item_target: String::new(),
                    actor_reference: String::new(),
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
            if (action.id == "use" || action.has_effect(CommandEffect::UseItem))
                && action.item_id.is_empty()
            {
                return PlayerCommand::Use {
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
                if action.id == "give" {
                    return PlayerCommand::GiveToPartyMember {
                        item_target: String::new(),
                        actor_reference: String::new(),
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
                if (action.id == "use" || action.has_effect(CommandEffect::UseItem))
                    && action.item_id.is_empty()
                {
                    return PlayerCommand::Use {
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

fn parse_follow_command(trimmed: &str) -> Option<PlayerCommand> {
    let lower = trimmed.to_ascii_lowercase();
    if lower == "unfollow" || lower == "stop following" {
        return Some(PlayerCommand::Follow { target: None });
    }
    if let Some(rest) = lower.strip_prefix("follow:") {
        let rest_trimmed = rest.trim();
        if rest_trimmed.is_empty() || rest_trimmed == "none" || rest_trimmed == "nobody" {
            return Some(PlayerCommand::Follow { target: None });
        }
        let target = trimmed["follow:".len()..].trim();
        return Some(PlayerCommand::Follow {
            target: Some(target.to_string()),
        });
    }
    if let Some(rest) = lower.strip_prefix("follow ") {
        let rest_trimmed = rest.trim();
        if rest_trimmed.is_empty() || rest_trimmed == "none" || rest_trimmed == "nobody" {
            return Some(PlayerCommand::Follow { target: None });
        }
        let target = trimmed["follow ".len()..].trim();
        return Some(PlayerCommand::Follow {
            target: Some(target.to_string()),
        });
    }
    if lower == "follow" {
        return Some(PlayerCommand::Follow { target: None });
    }
    None
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