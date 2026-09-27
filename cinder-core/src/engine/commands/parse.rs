use super::{PlayerCommand, items};
use crate::content::types::{ActionDefinition, ActionVerbKind, ContentPack, PartyOrderKind};

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

    if let Some(cmd) = parse_action_command(content, trimmed, &lower) {
        return cmd;
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

fn parse_action_command(
    content: &ContentPack,
    trimmed: &str,
    lower: &str,
) -> Option<PlayerCommand> {
    if content.actions.is_empty() {
        return None;
    }

    if let Some((action, matched_phrase)) = best_player_action_match(content, trimmed, lower) {
        return Some(
            action
                .verb_kind()
                .resolve_command(&action.id, matched_phrase.remainder.as_deref()),
        );
    }

    // Fallback: match by action ID directly (used by web UI overflow actions)
    content
        .actions
        .iter()
        .find(|action| action.player_enabled && action.id.to_ascii_lowercase() == *lower)
        .map(|action| action.verb_kind().resolve_command(&action.id, None))
}

impl ActionVerbKind {
    pub(crate) fn resolve_command(
        &self,
        action_id: &str,
        remainder: Option<&str>,
    ) -> PlayerCommand {
        match self {
            ActionVerbKind::Take => VerbGrammar::parse_take(remainder),
            ActionVerbKind::Give => VerbGrammar::parse_give(remainder),
            ActionVerbKind::Drop => PlayerCommand::Drop {
                target: remainder.unwrap_or_default().to_string(),
            },
            ActionVerbKind::Equip => PlayerCommand::Equip {
                target: remainder.unwrap_or_default().to_string(),
            },
            ActionVerbKind::Unequip => PlayerCommand::Unequip {
                target: remainder.unwrap_or_default().to_string(),
            },
            ActionVerbKind::Use => PlayerCommand::Use {
                target: remainder.unwrap_or_default().to_string(),
            },
            ActionVerbKind::Authored => PlayerCommand::Authored {
                command_id: action_id.to_string(),
                input: remainder.map(str::to_string),
            },
        }
    }
}

/// Natural language preposition grammar parsing for item commands.
struct VerbGrammar;

impl VerbGrammar {
    fn parse_take(remainder: Option<&str>) -> PlayerCommand {
        let Some(remainder) = remainder.map(str::trim).filter(|s| !s.is_empty()) else {
            return PlayerCommand::Take {
                target: String::new(),
            };
        };

        if let Some(target) = remainder.strip_prefix("off ") {
            return PlayerCommand::Unequip {
                target: target.trim().to_string(),
            };
        }

        let lower = remainder.to_ascii_lowercase();
        if let Some(idx) = lower.find(" from ") {
            let item_target = remainder[..idx].trim().to_string();
            let actor_reference = remainder[idx + " from ".len()..].trim().to_string();
            if !item_target.is_empty() && !actor_reference.is_empty() {
                return PlayerCommand::TakeFromPartyMember {
                    item_target,
                    actor_reference,
                };
            }
        }

        PlayerCommand::Take {
            target: remainder.to_string(),
        }
    }

    fn parse_give(remainder: Option<&str>) -> PlayerCommand {
        let Some(remainder) = remainder.map(str::trim).filter(|s| !s.is_empty()) else {
            return PlayerCommand::GiveToPartyMember {
                item_target: String::new(),
                actor_reference: String::new(),
            };
        };

        let lower = remainder.to_ascii_lowercase();
        if let Some(idx) = lower.find(" to ") {
            let item_target = remainder[..idx].trim().to_string();
            let actor_reference = remainder[idx + " to ".len()..].trim().to_string();
            PlayerCommand::GiveToPartyMember {
                item_target,
                actor_reference,
            }
        } else {
            PlayerCommand::GiveToPartyMember {
                item_target: remainder.to_string(),
                actor_reference: String::new(),
            }
        }
    }
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
    if lower == "unfollow" || lower == "stop following" || lower == "follow" {
        return Some(PlayerCommand::Follow { target: None });
    }
    let rest = lower
        .strip_prefix("follow:")
        .or_else(|| lower.strip_prefix("follow "))?;
    let rest_trimmed = rest.trim();
    if rest_trimmed.is_empty() || rest_trimmed == "none" || rest_trimmed == "nobody" {
        return Some(PlayerCommand::Follow { target: None });
    }
    let target = trimmed[(trimmed.len() - rest.len())..].trim();
    Some(PlayerCommand::Follow {
        target: Some(target.to_string()),
    })
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
    use crate::content::types::{ActionPlayerCommand, ActionPlayerInput, CommandEffect};
    use crate::engine::test_fixtures::minimal_test_pack;

    fn parse(raw: &str) -> PlayerCommand {
        parse_command(&minimal_test_pack(), raw)
    }

    fn test_action(id: &str, phrases: &[&str], effect: Option<CommandEffect>) -> ActionDefinition {
        ActionDefinition {
            id: id.to_string(),
            player_enabled: true,
            phrases: phrases.iter().map(|s| s.to_string()).collect(),
            player_command: Some(ActionPlayerCommand {
                input: Some(ActionPlayerInput {
                    required: false,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            effects: effect.into_iter().collect(),
            ..Default::default()
        }
    }

    fn test_pack_with_actions() -> ContentPack {
        let mut pack = minimal_test_pack();
        pack.actions = vec![
            test_action(
                "take",
                &["take", "pick up"],
                Some(CommandEffect::PickUpItem),
            ),
            test_action("give", &["give"], None),
            test_action("drop", &["drop"], Some(CommandEffect::DropItem)),
            test_action("equip", &["equip"], Some(CommandEffect::EquipItem)),
            test_action("unequip", &["unequip"], Some(CommandEffect::UnequipItem)),
            test_action("use", &["use"], Some(CommandEffect::UseItem)),
            test_action("dance", &["dance"], None),
        ];
        pack
    }

    #[test]
    fn parses_grammar_variations_for_item_actions() {
        let pack = test_pack_with_actions();

        // Take
        assert_eq!(
            parse_command(&pack, "take sword"),
            PlayerCommand::Take {
                target: "sword".to_string(),
            }
        );
        assert_eq!(
            parse_command(&pack, "pick up key"),
            PlayerCommand::Take {
                target: "key".to_string(),
            }
        );
        assert_eq!(
            parse_command(&pack, "take off iron helmet"),
            PlayerCommand::Unequip {
                target: "iron helmet".to_string(),
            }
        );
        assert_eq!(
            parse_command(&pack, "take potion from malik"),
            PlayerCommand::TakeFromPartyMember {
                item_target: "potion".to_string(),
                actor_reference: "malik".to_string(),
            }
        );

        // Give
        assert_eq!(
            parse_command(&pack, "give apple to zayd"),
            PlayerCommand::GiveToPartyMember {
                item_target: "apple".to_string(),
                actor_reference: "zayd".to_string(),
            }
        );
        assert_eq!(
            parse_command(&pack, "give apple"),
            PlayerCommand::GiveToPartyMember {
                item_target: "apple".to_string(),
                actor_reference: String::new(),
            }
        );

        // Drop, Equip, Unequip, Use
        assert_eq!(
            parse_command(&pack, "drop stone"),
            PlayerCommand::Drop {
                target: "stone".to_string(),
            }
        );
        assert_eq!(
            parse_command(&pack, "equip shield"),
            PlayerCommand::Equip {
                target: "shield".to_string(),
            }
        );
        assert_eq!(
            parse_command(&pack, "unequip boots"),
            PlayerCommand::Unequip {
                target: "boots".to_string(),
            }
        );
        assert_eq!(
            parse_command(&pack, "use torch"),
            PlayerCommand::Use {
                target: "torch".to_string(),
            }
        );

        // Authored
        assert_eq!(
            parse_command(&pack, "dance wildly"),
            PlayerCommand::Authored {
                command_id: "dance".to_string(),
                input: Some("wildly".to_string()),
            }
        );

        // Direct action ID click fallback (UI overflow)
        assert_eq!(
            parse_command(&pack, "take"),
            PlayerCommand::Take {
                target: String::new(),
            }
        );
        assert_eq!(
            parse_command(&pack, "give"),
            PlayerCommand::GiveToPartyMember {
                item_target: String::new(),
                actor_reference: String::new(),
            }
        );
        assert_eq!(
            parse_command(&pack, "dance"),
            PlayerCommand::Authored {
                command_id: "dance".to_string(),
                input: None,
            }
        );
    }

    #[test]
    fn parses_follow_commands() {
        let pack = minimal_test_pack();
        assert_eq!(
            parse_command(&pack, "follow"),
            PlayerCommand::Follow { target: None }
        );
        assert_eq!(
            parse_command(&pack, "unfollow"),
            PlayerCommand::Follow { target: None }
        );
        assert_eq!(
            parse_command(&pack, "stop following"),
            PlayerCommand::Follow { target: None }
        );
        assert_eq!(
            parse_command(&pack, "follow: none"),
            PlayerCommand::Follow { target: None }
        );
        assert_eq!(
            parse_command(&pack, "follow: Harun"),
            PlayerCommand::Follow {
                target: Some("Harun".to_string())
            }
        );
        assert_eq!(
            parse_command(&pack, "follow Malik"),
            PlayerCommand::Follow {
                target: Some("Malik".to_string())
            }
        );
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
