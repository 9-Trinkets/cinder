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
    /// Items scattered into this actor's room as loose items when it is
    /// defeated by the player's attack, item id → drop spec.
    #[serde(default)]
    pub drops: BTreeMap<String, DropSpec>,
    /// Equipment slots filled when this actor spawns (slot id → item id).
    #[serde(default)]
    pub initial_equipment: BTreeMap<String, String>,
    /// Loose items in this actor's inventory when it spawns (item id → count or drop spec).
    #[serde(default)]
    pub initial_inventory: BTreeMap<String, DropSpec>,
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
    #[serde(default)]
    pub healing: Option<ActorHealingSpec>,
    #[serde(default)]
    pub skills: Vec<String>,
    pub prompt_context: ActorPromptContext,
    #[serde(default)]
    pub act_cast: Option<ActorActCast>,
    #[serde(default)]
    pub transformations: Vec<ActorTransformation>,
    #[serde(default)]
    pub game_data: BTreeMap<String, String>,
}

/// Optional healing capability configured on an actor definition.
/// Hostile actors prioritize healing wounded allies (or self) before striking.
/// Allied party members use this to determine support reaction healing values.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorHealingSpec {
    #[serde(default)]
    pub amount: i32,
    #[serde(default)]
    pub message: String,
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

/// A content-declared transformation stage for an actor (job change,
/// evolution, awakening, promotion, ...). The engine applies the first stage
/// whose trigger is met at a transformation checkpoint (e.g. when the actor
/// joins or follows the party), but never decides *what* transformations
/// mean — renames, stance changes, signals, and narration are pack-authored.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActorTransformation {
    /// Stable stage id for state tracking (e.g. `"awakening"`). Applied
    /// stages are recorded as `transformed:{actor}:{id}` story variables and
    /// exposed through `WorldState::is_actor_transformed`.
    pub id: String,
    /// Condition under which the transformation applies.
    #[serde(default)]
    pub trigger: TransformationTrigger,
    /// Optional rename the actor takes on when transformed.
    #[serde(default)]
    pub rename: Option<TransformationRename>,
    /// Stance the actor adopts once transformed.
    #[serde(default)]
    pub stance: Option<String>,
    /// Whether the actor also follows/joins the player once transformed.
    #[serde(default = "default_transformation_follows")]
    pub follows_player: bool,
    /// Beat advance signals emitted when the transformation is applied.
    #[serde(default)]
    pub signals: Vec<String>,
    /// Override message keys used for the transformation's narration. When
    /// empty the engine uses the pack's standard `transformation.wake.*`
    /// narration keys, so all renames share one prose set by default.
    #[serde(default)]
    pub narration_keys: Vec<String>,
    /// Suppresses the transformation's narration entirely. Use for quiet state
    /// shifts — a villager's mood turning, an actor settling into a new role —
    /// where the pack narrates its own beat or nothing should be said at all.
    /// Without this, a shift with no `narration_keys` falls back to the
    /// `transformation.wake.*` prose, which would be wrong for it.
    #[serde(default)]
    pub silent: bool,
    /// Story variables set in WorldState when the transformation is applied.
    #[serde(default)]
    pub story_vars: std::collections::BTreeMap<String, String>,
}

fn default_transformation_follows() -> bool {
    true
}

/// Condition a transformation stage waits on.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum TransformationTrigger {
    /// Fires immediately whenever the transformation is evaluated.
    #[default]
    Always,
    /// Requires an (effective) stat to meet a threshold.
    Stat {
        /// Stat key, e.g. `"wisdom"`.
        stat: String,
        #[serde(default = "default_trigger_gte")]
        gte: i32,
    },
    /// Requires a story variable to equal `value`.
    StoryVar {
        key: String,
        #[serde(default)]
        value: String,
    },
}

fn default_trigger_gte() -> i32 {
    10
}

/// The identity a transformed actor takes on.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TransformationRename {
    /// New display name (kept empty to keep the actor's current name).
    #[serde(default)]
    pub name: String,
    /// Blurb establishing who they really are (used in prompt grounding).
    #[serde(default)]
    pub who: String,
    /// Forgotten-memory fragment surfaced during the transformation.
    #[serde(default)]
    pub fragment: String,
    /// Inspect text shown after the transformation.
    #[serde(default)]
    pub inspect_text: Option<String>,
    /// Prompt context used for the transformed actor.
    #[serde(default)]
    pub prompt_context: Option<ActorPromptContext>,
}
