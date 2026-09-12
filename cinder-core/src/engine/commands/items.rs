use super::PlayerCommand;

pub(super) fn parse_item_command(trimmed: &str) -> Option<PlayerCommand> {
    // Equipment must precede take so `take off X` is not interpreted as an
    // item named `off X`.
    phrase_target(
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
    )
        .map(|target| PlayerCommand::Equip { target })
        .or_else(|| {
            phrase_target(
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
            )
            .map(|target| PlayerCommand::Unequip { target })
        })
        .or_else(|| {
            phrase_target(trimmed, &["take ", "pick up ", "get ", "pickup "])
                .map(|target| PlayerCommand::Take { target })
        })
        .or_else(|| {
            phrase_target(trimmed, &["drop "]).map(|target| PlayerCommand::Drop { target })
        })
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
        for input in ["take", "drop", "equip", "unequip"] {
            assert!(parse_item_command(input).is_none());
        }
    }
}
