//! An interaction: an authored choice the player can make about an actor or a
//! feature. Saying something to a person, turning a lamp down, unlocking a gate
//! and operating a valve are all interactions.

use super::{AdvanceEffect, Condition};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Interaction {
    pub id: String,
    /// The option as listed in its panel, e.g. "Greet Wren" or "Dim the lamp".
    pub label: String,
    /// Other things the player can type to choose it, such as "free zayd". Each
    /// is matched as the whole input.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    /// Conditions that must hold before the interaction is offered.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub when: Vec<Condition>,
    /// What the player reads when they choose it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lines: Vec<String>,
    /// What choosing it changes. A `set_feature_state` with no `feature` acts on
    /// the feature that owns the interaction.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<AdvanceEffect>,
    /// Offered again after use. Defaults to once only.
    #[serde(default)]
    pub repeatable: bool,
}
