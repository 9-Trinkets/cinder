use super::PlayerCommand;

pub(super) fn parse_item_command(trimmed: &str) -> Option<PlayerCommand> {
    let lower = trimmed.to_ascii_lowercase();

    // 1. Give to party member: `give <item> to <member>` or `hand <item> to <member>`
    for prefix in &["give ", "hand "] {
        if let Some(rest) = lower.strip_prefix(prefix) {
            let actual_rest = trimmed[prefix.len()..].trim();
            if actual_rest.is_empty() {
                continue;
            }
            if let Some(idx) = rest.find(" to ") {
                let item_target = actual_rest[..idx].trim().to_string();
                let actor_reference = actual_rest[idx + 4..].trim().to_string();
                return Some(PlayerCommand::GiveToPartyMember {
                    item_target,
                    actor_reference,
                });
            } else {
                return Some(PlayerCommand::GiveToPartyMember {
                    item_target: actual_rest.to_string(),
                    actor_reference: String::new(),
                });
            }
        }
    }

    // 2. Equipment must precede take so `take off X` is not interpreted as an
    // item named `off X`.
    if let Some(target) = phrase_target(
        trimmed,
        &[
            "equip ",
            "wear ",
            "wield ",
            "ready ",
            "draw ",
            "slide ",
            "put on ",
            "take up ",
            "grip ",
            "arm with ",
        ],
    ) {
        return Some(PlayerCommand::Equip { target });
    }

    if let Some(target) = phrase_target(
        trimmed,
        &[
            "unequip ",
            "take off ",
            "stow ",
            "remove ",
            "sheathe ",
            "put away ",
            "set down ",
        ],
    ) {
        return Some(PlayerCommand::Unequip { target });
    }

    // 3. Take from member: `take <item> from <member>`
    for prefix in &["take ", "pick up ", "get ", "pickup "] {
        if let Some(rest) = lower.strip_prefix(prefix) {
            let actual_rest = trimmed[prefix.len()..].trim();
            if let Some(idx) = rest.find(" from ") {
                let item_target = actual_rest[..idx].trim().to_string();
                let actor_reference = actual_rest[idx + 6..].trim().to_string();
                if !item_target.is_empty() && !actor_reference.is_empty() {
                    return Some(PlayerCommand::TakeFromPartyMember {
                        item_target,
                        actor_reference,
                    });
                }
            }
        }
    }

    // 4. Take from room
    if let Some(target) = phrase_target(trimmed, &["take ", "pick up ", "get ", "pickup "]) {
        return Some(PlayerCommand::Take { target });
    }

    // 5. Drop
    if let Some(target) = phrase_target(trimmed, &["drop "]) {
        return Some(PlayerCommand::Drop { target });
    }

    None
}

fn phrase_target(trimmed: &str, prefixes: &[&str]) -> Option<String> {
    let lower = trimmed.to_ascii_lowercase();
    prefixes.iter().find_map(|prefix| {
        lower.strip_prefix(prefix).and_then(|_| {
            let target = trimmed[prefix.len()..].trim();
            (!target.is_empty()).then(|| target.to_string())
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn take_and_drop_phrases_resolve() {
        assert!(matches!(
            parse_item_command("take ring"),
            Some(PlayerCommand::Take { target }) if target == "ring"
        ));
        assert!(matches!(
            parse_item_command("pick up scroll"),
            Some(PlayerCommand::Take { target }) if target == "scroll"
        ));
        assert!(matches!(
            parse_item_command("get coffee"),
            Some(PlayerCommand::Take { target }) if target == "coffee"
        ));
        assert!(matches!(
            parse_item_command("pickup badge"),
            Some(PlayerCommand::Take { target }) if target == "badge"
        ));
        assert!(matches!(
            parse_item_command("drop the cracked scroll"),
            Some(PlayerCommand::Drop { target }) if target == "the cracked scroll"
        ));
    }

    #[test]
    fn equipment_phrases_resolve() {
        assert!(matches!(
            parse_item_command("equip iron chisel"),
            Some(PlayerCommand::Equip { target }) if target == "iron chisel"
        ));
        assert!(matches!(
            parse_item_command("wear leaf helm"),
            Some(PlayerCommand::Equip { target }) if target == "leaf helm"
        ));
        assert!(matches!(
            parse_item_command("wield leaf spear"),
            Some(PlayerCommand::Equip { target }) if target == "leaf spear"
        ));
        assert!(matches!(
            parse_item_command("unequip iron chisel"),
            Some(PlayerCommand::Unequip { target }) if target == "iron chisel"
        ));
        assert!(matches!(
            parse_item_command("take off leaf helm"),
            Some(PlayerCommand::Unequip { target }) if target == "leaf helm"
        ));
        assert!(matches!(
            parse_item_command("stow leaf spear"),
            Some(PlayerCommand::Unequip { target }) if target == "leaf spear"
        ));
        assert!(matches!(
            parse_item_command("take up the spear"),
            Some(PlayerCommand::Equip { target }) if target == "the spear"
        ));
        assert!(matches!(
            parse_item_command("set down the spear"),
            Some(PlayerCommand::Unequip { target }) if target == "the spear"
        ));
    }

    #[test]
    fn bare_item_verbs_do_not_resolve() {
        for input in ["take", "drop", "equip", "unequip", "give", "hand"] {
            assert!(parse_item_command(input).is_none());
        }
    }

    #[test]
    fn party_item_transfer_phrases_resolve() {
        assert!(matches!(
            parse_item_command("give iron sword to blair"),
            Some(PlayerCommand::GiveToPartyMember { item_target, actor_reference })
                if item_target == "iron sword" && actor_reference == "blair"
        ));
        assert!(matches!(
            parse_item_command("hand healing potion to dark golem"),
            Some(PlayerCommand::GiveToPartyMember { item_target, actor_reference })
                if item_target == "healing potion" && actor_reference == "dark golem"
        ));
        assert!(matches!(
            parse_item_command("take iron sword from blair"),
            Some(PlayerCommand::TakeFromPartyMember { item_target, actor_reference })
                if item_target == "iron sword" && actor_reference == "blair"
        ));
        assert!(matches!(
            parse_item_command("get shield from companion-1"),
            Some(PlayerCommand::TakeFromPartyMember { item_target, actor_reference })
                if item_target == "shield" && actor_reference == "companion-1"
        ));
    }
}
