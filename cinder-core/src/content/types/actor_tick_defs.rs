use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorTickScope {
    /// Tick only actors on the connected room graph containing the player.
    #[default]
    CurrentBoard,
    /// Tick actors regardless of which disconnected room graph they occupy.
    AllRooms,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actor_tick_scope_defaults_to_current_board_and_accepts_all_rooms() {
        assert_eq!(ActorTickScope::default(), ActorTickScope::CurrentBoard);
        assert_eq!(
            serde_json::from_str::<ActorTickScope>("\"all_rooms\"").unwrap(),
            ActorTickScope::AllRooms
        );
    }
}
