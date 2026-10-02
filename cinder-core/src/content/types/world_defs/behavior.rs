use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Non-skill actor behavior declared in `behavior.json`. Strict skill packs
/// define autonomous attacks and support in `skills.json`; movement hold rules
/// remain here because they constrain navigation rather than execute a skill.
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
    /// Skill required by the compatibility strike rule. Strict skill packs
    /// reject legacy strike rules and author autonomous uses in `skills.json`.
    #[serde(default)]
    pub strike_skill_id: String,
    /// `effect_table` rule returning `[{ "kind": "strike" }]` when the actor
    /// should strike this tick. Retained for packs that do not declare
    /// centralized autonomous skill behavior.
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
            strike_skill_id: if self.strike_skill_id.is_empty() {
                default.strike_skill_id.clone()
            } else {
                self.strike_skill_id.clone()
            },
            strike: self.strike.clone().or_else(|| default.strike.clone()),
            hold: self.hold.clone().or_else(|| default.hold.clone()),
        }
    }
}
