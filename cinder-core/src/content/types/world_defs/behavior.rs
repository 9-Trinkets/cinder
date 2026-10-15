use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Actor movement behavior declared in `behavior.json`. Attacks and support
/// are defined as autonomous uses in `skills.json`; movement hold rules remain
/// here because they constrain navigation rather than execute a skill.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BehaviorDefinition {
    #[serde(default)]
    pub defaults: BehaviorActorDefinition,
    #[serde(default)]
    pub actors: BTreeMap<String, BehaviorActorDefinition>,
}

/// The hold rule for one actor (or for the pack default). Per-actor entries
/// override the correspondingly-named field of the default; a field left
/// `null` inherits the default.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BehaviorActorDefinition {
    /// `effect_table` rule returning `[{ "kind": "hold" }]` when the actor must
    /// stay put (blocking movement), or `[]` when it is free to move.
    #[serde(default)]
    pub hold: Option<Value>,
}

impl BehaviorActorDefinition {
    pub fn resolved_with_default(
        &self,
        default: &BehaviorActorDefinition,
    ) -> BehaviorActorDefinition {
        BehaviorActorDefinition {
            hold: self.hold.clone().or_else(|| default.hold.clone()),
        }
    }
}
