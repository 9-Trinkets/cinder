use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ErrorTextDefinition {
    pub room_missing: String,
    pub cannot_go: String,
    pub actor_not_here: String,
    pub actor_unknown: String,
    pub feature_unknown: String,
    pub unknown_input: String,
    pub dialogue_unavailable: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PresentationTextDefinition {
    pub room_observation: String,
    pub objective: String,
    pub features: String,
    /// Line listing loose items lying in the room (e.g. dropped flags). Rendered
    /// into the `{items}` slot of `room_observation` when any are present.
    #[serde(default)]
    pub loose_items: String,
    pub people: String,
    pub exits: String,
    pub feature_consumables: String,
    pub actor_speech: String,
    #[serde(default = "default_actor_targeted_speech")]
    pub actor_targeted_speech: String,
    pub actor_departed: String,
    pub actor_arrived: String,
    pub act_ended: String,
    /// Suffix appended to a present actor's name when their stance is allied,
    /// so same-named allies and hostiles in one room read differently.
    #[serde(default)]
    pub ally_suffix: String,
    /// Suffix appended to a present actor's name when their stance is hostile.
    #[serde(default)]
    pub hostile_suffix: String,
}

use super::super::default_actor_targeted_speech;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PresentationDefinition {
    #[serde(default)]
    pub error_text: ErrorTextDefinition,
    #[serde(default)]
    pub presentation_text: PresentationTextDefinition,
}