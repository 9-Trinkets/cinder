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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeriodicActorEffectTrigger {
    pub room_item: String,
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
    }
}
