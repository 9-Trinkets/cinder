//! The single actor-order vocabulary.
//!
//! An *order* is a short token an actor may hold. The same tokens are authored
//! per actor in `movement.json`, assigned to party members at runtime, and
//! matched by `OrderIs` combat rules. Every consumer parses those tokens here so
//! the vocabulary stays consistent: the movement engine maps an [`OrderKind`] to
//! a `MovementStrategy`, combat targeting reads [`OrderKind::Hold`] as "guard",
//! and party bookkeeping uses the same parse to decide follow behavior.
//!
//! Adding a new movement behavior is a single token here plus one arm in
//! `actor_tick::movement::strategy_from_order`.

/// A parsed order token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OrderKind {
    /// Hold position. Authored and assigned as `guard`.
    Hold,
    /// Drift to a random reachable adjacent room (`patrol`/`wander`/`roam`).
    Patrol,
    /// Advance toward the player (`hunt`/`advance`/`toward_player`).
    Hunt,
    /// Follow the player / party leader (`follow`).
    Follow,
    /// Deploy toward a room, then hold there (`deploy <room>`).
    Deploy(String),
    /// Path toward a specific room (`to`/`goto`).
    Goto(String),
    /// Take an exit matching a label or alias (`exit`).
    Exit(String),
    /// Send a conjured ally away (`dismiss`). Never held: the actor is removed.
    Dismiss,
}

/// Parses an order token (case-insensitive) into an [`OrderKind`]. Accepts an
/// optional inline target separated by whitespace or `:`, e.g. `deploy:gate` or
/// `deploy gate`. Returns `None` for an empty or unrecognized token.
pub(crate) fn parse_order(token: &str) -> Option<OrderKind> {
    let token = token.trim();
    if token.is_empty() {
        return None;
    }
    let mut parts = token.splitn(2, |c: char| c.is_whitespace() || c == ':');
    let base = parts.next().unwrap_or_default().to_ascii_lowercase();
    let target = parts.next().unwrap_or_default().trim().to_string();
    match base.as_str() {
        "guard" => Some(OrderKind::Hold),
        "patrol" | "wander" | "roam" | "random_adjacent" => Some(OrderKind::Patrol),
        "hunt" | "advance" | "toward_player" | "chase" | "pursue" => Some(OrderKind::Hunt),
        "follow" => Some(OrderKind::Follow),
        "deploy" => Some(OrderKind::Deploy(target)),
        "to" | "goto" | "move_to" => Some(OrderKind::Goto(target)),
        "exit" | "exit_label" | "follow_exit" => Some(OrderKind::Exit(target)),
        "dismiss" => Some(OrderKind::Dismiss),
        _ => None,
    }
}

/// Whether `token` is a guard order.
pub(crate) fn is_hold(token: &str) -> bool {
    matches!(parse_order(token), Some(OrderKind::Hold))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_canonical_tokens_and_aliases() {
        assert_eq!(parse_order("guard"), Some(OrderKind::Hold));
        assert_eq!(parse_order("GUARD"), Some(OrderKind::Hold));
        assert_eq!(parse_order("patrol"), Some(OrderKind::Patrol));
        assert_eq!(parse_order("random_adjacent"), Some(OrderKind::Patrol));
        assert_eq!(parse_order("advance"), Some(OrderKind::Hunt));
        assert_eq!(parse_order("toward_player"), Some(OrderKind::Hunt));
        assert_eq!(parse_order("follow"), Some(OrderKind::Follow));
    }

    #[test]
    fn parses_inline_targets_with_space_or_colon() {
        assert_eq!(
            parse_order("deploy:gate"),
            Some(OrderKind::Deploy("gate".to_string()))
        );
        assert_eq!(
            parse_order("deploy gate"),
            Some(OrderKind::Deploy("gate".to_string()))
        );
        assert_eq!(
            parse_order("goto gate"),
            Some(OrderKind::Goto("gate".to_string()))
        );
        assert_eq!(
            parse_order("exit Clockwise"),
            Some(OrderKind::Exit("Clockwise".to_string()))
        );
        assert_eq!(
            parse_order("deploy"),
            Some(OrderKind::Deploy(String::new()))
        );
    }

    #[test]
    fn empty_and_unknown_tokens_are_none() {
        assert_eq!(parse_order(""), None);
        assert_eq!(parse_order("   "), None);
        assert_eq!(parse_order("assist"), None);
        assert_eq!(parse_order("hold"), None);
        assert_eq!(parse_order("stay"), None);
        assert_eq!(parse_order("stationary"), None);
    }

    #[test]
    fn hold_helper_matches_only_hold_orders() {
        assert!(is_hold("guard"));
        assert!(!is_hold("hold"));
        assert!(!is_hold("patrol"));
        assert!(!is_hold("deploy:gate"));
    }
}
