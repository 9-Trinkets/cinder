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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeriodicActorEffectDefinition {
    pub id: String,
    pub trigger: PeriodicActorEffectTrigger,
    pub targets: PeriodicActorEffectTargets,
    pub effect: PeriodicActorEffect,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PeriodicActorEffectTrigger {
    pub room_item: String,
    /// Fixed number of times the effect may fire before its room item is spent
    /// and removed. `None` (the default) means the effect fires indefinitely
    /// while the room item remains.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_activations: Option<u32>,
    /// Content message key narrated when the trigger's room item runs out of
    /// activations and fades away. Empty when the pack wants no line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deplete_message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PeriodicActorEffectTargets {
    HostileLiving,
    AlliedLiving,
    AnyLiving,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PeriodicActorEffect {
    Damage { amount: i32 },
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

    #[test]
    fn periodic_damage_effect_deserializes_from_settings_shape() {
        let definition: PeriodicActorEffectDefinition = serde_json::from_value(serde_json::json!({
            "id": "drain_sigil",
            "trigger": { "room_item": "drain-sigil" },
            "targets": "hostile_living",
            "effect": { "kind": "damage", "amount": 2 },
            "message": "combat.actor_drained"
        }))
        .unwrap();

        assert_eq!(definition.effect, PeriodicActorEffect::Damage { amount: 2 });
        assert_eq!(
            definition.targets,
            PeriodicActorEffectTargets::HostileLiving
        );
        assert_eq!(definition.trigger.max_activations, None);
        assert_eq!(definition.trigger.deplete_message, None);
    }

    #[test]
    fn periodic_effect_trigger_accepts_activation_charges() {
        let definition: PeriodicActorEffectDefinition = serde_json::from_value(serde_json::json!({
            "id": "drain_sigil",
            "trigger": {
                "room_item": "drain-sigil",
                "max_activations": 5,
                "deplete_message": "combat.drain_sigil_spent"
            },
            "targets": "hostile_living",
            "effect": { "kind": "damage", "amount": 2 },
            "message": "combat.actor_drained"
        }))
        .unwrap();

        assert_eq!(definition.trigger.max_activations, Some(5));
        assert_eq!(
            definition.trigger.deplete_message.as_deref(),
            Some("combat.drain_sigil_spent")
        );
    }
}
