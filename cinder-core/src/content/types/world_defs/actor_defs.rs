use super::DropSpec;
use crate::engine::state::ActorRelationship;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorPromptContext {
    #[serde(default)]
    pub character_notes: Vec<String>,
    #[serde(default)]
    pub subtext_notes: Vec<String>,
    #[serde(default)]
    pub response_notes: Vec<String>,
    #[serde(default)]
    pub behavior_examples: Vec<String>,
}

/// A single actor defined in `actors.json`. `room_id` is optional: empty means
/// the actor is offstage and has no spatial location.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorDefinition {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    /// The actor's home room. Omitted or empty means the actor is offstage:
    /// it has no spatial location and is excluded from movement, room
    /// observation, combat, targeting, party, and proximity while keeping its
    /// identity, relationships, and conversation memory.
    #[serde(default)]
    pub room_id: String,
    /// Starting level for this actor, seeded into world state at session
    /// creation and read by level-gated rules (e.g. the charm formula).
    /// Defaults to 1.
    #[serde(default = "default_actor_level")]
    pub level: u32,
    #[serde(default)]
    pub initial_stats: BTreeMap<String, i32>,
    #[serde(default)]
    pub initial_pair_stats: BTreeMap<String, BTreeMap<String, i32>>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub inspect_text: String,
    #[serde(default)]
    pub required_consumable_tags: Vec<String>,
    /// Whether the player can attack this actor. Defaults to false; combat
    /// packs opt their creatures in explicitly.
    #[serde(default)]
    pub attackable: bool,
    /// When this actor follows the player, it intercepts damage aimed at the
    /// player, taking it at a rate reduced by its own defense.
    #[serde(default)]
    pub guard: bool,
    /// Items scattered into this actor's room as loose items when it is
    /// defeated by the player's attack, item id → drop spec.
    #[serde(default)]
    pub drops: BTreeMap<String, DropSpec>,
    /// Equipment slots filled when this actor spawns (slot id → item id).
    #[serde(default)]
    pub initial_equipment: BTreeMap<String, String>,
    /// Loose items in this actor's inventory when it spawns (item id → count).
    #[serde(default)]
    pub initial_inventory: BTreeMap<String, u32>,
    /// XP awarded to the whole party when this actor is defeated.
    #[serde(default)]
    pub xp_drop: u32,
    /// Game minutes between this actor's autonomous hostile strikes. Only used
    /// while the actor is hostile; defaults to 4 when omitted.
    #[serde(default)]
    pub attack_interval_minutes: Option<u32>,
    /// Whether the actor starts hostile to the player (used for mobs that
    /// attack on sight, like the level-2 elf army).
    #[serde(default)]
    pub initial_hostile: bool,
    /// The actor's authored relationship toward the player, seeded into world
    /// state at session creation. Absent = neutral/non-following. When set,
    /// takes precedence over `initial_hostile` (which is shorthand for
    /// `{ stance: hostile, follows_player: false }` and used by combat packs).
    #[serde(default)]
    pub initial_relationship: Option<ActorRelationship>,
    /// The element this actor's basic attacks deal (e.g. "physical", "fire").
    /// Resistances on the *target* are consulted against this key. Empty
    /// defaults to "physical".
    #[serde(default)]
    pub attack_kind: String,
    /// Damage reduction per damage kind (arbitrary element strings like
    /// "physical", "fire"). Each attack of that kind is reduced by the value;
    /// when it reaches zero the fully-resisted hit narrates `combat.no_effect`
    /// instead of `combat.attack_hit`. Partial resistance and weakness (via
    /// negative values) work through the same map.
    #[serde(default)]
    pub resistances: BTreeMap<String, i32>,
    pub prompt_context: ActorPromptContext,
    #[serde(default)]
    pub act_cast: Option<ActorActCast>,
    #[serde(default)]
    pub game_data: BTreeMap<String, String>,
}

impl ActorDefinition {
    /// Whether the actor has no home room and therefore no spatial location.
    /// Offstage actors never participate in movement, room observation,
    /// combat, targeting, party, or proximity, but keep their identity,
    /// relationships, and conversation memory.
    pub fn is_offstage(&self) -> bool {
        self.room_id.trim().is_empty()
    }

    pub fn attack_interval_minutes(&self, default_minutes: u32) -> u32 {
        self.attack_interval_minutes.unwrap_or(default_minutes)
    }

    /// The element this actor's attacks deal, defaulting to "physical" when
    /// the pack omits `attack_kind`.
    pub fn attack_kind(&self) -> &str {
        if self.attack_kind.is_empty() {
            "physical"
        } else {
            &self.attack_kind
        }
    }
}

fn default_actor_level() -> u32 {
    1
}

/// Author-authored blurb/metadata for an actor in another actor's cast,
/// used by the dialogue layer to surface cast members of the participating
/// partner (see `collect_act_cast`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActorActCast {
    #[serde(default)]
    pub inspect_blurb: String,
    #[serde(default)]
    pub intro_blurb: String,
    #[serde(default)]
    pub return_blurb: String,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}