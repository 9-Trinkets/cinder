pub(crate) mod help;
pub(crate) mod items;
pub(crate) mod parse;
pub(crate) mod refs;

use crate::content::types::PartyOrderKind;
use serde::{Deserialize, Serialize};

pub(crate) use help::{player_command_help_text, player_command_suggestions};
pub(crate) use parse::parse_command;
pub(crate) use refs::{resolve_actor_reference_input, unknown_target_token};

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    /// A generic `use <item>` / `eat <item>` / `drink <item>` / `consume <item>` command.
    Use {
        target: String,
    },
    PartyOrder {
        actor_reference: String,
        order: PartyOrderKind,
    },
    GiveToPartyMember {
        item_target: String,
        actor_reference: String,
    },
    TakeFromPartyMember {
        item_target: String,
        actor_reference: String,
    },
    Follow {
        target: Option<String>,
    },
    Help,
    Quit,
    Unknown,
}