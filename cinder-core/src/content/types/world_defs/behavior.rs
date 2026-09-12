use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// The behavior of a hostile actor during a tick, declared per pack in
/// `behavior.json`. Movement *destination* and *cadence* stay in
/// `movement.json`; this file governs the *eligibility* decisions — whether an
/// actor strikes, holds, or is free to move. Each rule is a neuron symbolic
/// hook (`effect_table`) evaluated against a rich actor/world input JSON, so
/// all policy lives in content rather than Rust.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BehaviorDefinition {
    #[serde(default)]
    pub defaults: BehaviorActorDefinition,
    #[serde(default)]
    pub actors: BTreeMap<String, BehaviorActorDefinition>,
}

/// The two behavior rules for one actor (or for the pack default). Per-actor
/// entries override the correspondingly-named field of the default; fields
/// left `null` inherit the default.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BehaviorActorDefinition {
    /// `effect_table` rule returning `[{ "kind": "strike" }]` when the actor
    /// should strike this tick.
    #[serde(default)]
    pub strike: Option<Value>,
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
            strike: self.strike.clone().or_else(|| default.strike.clone()),
            hold: self.hold.clone().or_else(|| default.hold.clone()),
        }
    }
}