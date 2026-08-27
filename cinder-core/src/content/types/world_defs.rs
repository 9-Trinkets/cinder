use super::{RuleBundleProgressRef, default_actor_targeted_speech, default_stat_default_value};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// The single source of truth for all movement configuration in a content pack:
/// which stages lock all movement, which rooms are unreachable, the reactive
/// suppression rule (e.g. low stamina), and each actor's target rules.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MovementConfigDefinition {
    #[serde(default)]
    pub stage_locks: Vec<String>,
    #[serde(default)]
    pub unreachable_rooms: Vec<String>,
    #[serde(default)]
    pub suppress_when: Option<Value>,
    #[serde(default)]
    pub actors: BTreeMap<String, ActorMovementRulesDefinition>,
}

/// The single source of truth for speech-selection policy in a content pack.
/// It controls whether actors speak, whether they prefer direct speech, and
/// when room-addressed speech is available.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechConfigDefinition {
    #[serde(default)]
    pub should_speak_when: Option<Value>,
    #[serde(default)]
    pub should_direct_speech_when: Option<Value>,
    #[serde(default = "default_speech_room_min_audience")]
    pub room_min_audience: usize,
}

impl Default for SpeechConfigDefinition {
    fn default() -> Self {
        Self {
            should_speak_when: None,
            should_direct_speech_when: None,
            room_min_audience: default_speech_room_min_audience(),
        }
    }
}

fn default_speech_room_min_audience() -> usize {
    2
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuleBundlesDefinition {
    #[serde(default)]
    pub bundles: Vec<RuleBundleDefinition>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuleBundleDefinition {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub stage_ids: Vec<String>,
    #[serde(default)]
    pub progress: RuleBundleProgressDefinition,
    #[serde(default)]
    pub completion: RuleBundleCompletionDefinition,
    #[serde(default)]
    pub guidance: RuleBundleGuidanceDefinition,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuleBundleProgressDefinition {
    #[serde(default)]
    pub keys: Vec<RuleBundleProgressKeyDefinition>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuleBundleProgressKeyDefinition {
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub label: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuleBundleCompletionDefinition {
    #[serde(default)]
    pub mark_actor_complete_on: Vec<RuleBundleCompletionTrigger>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuleBundleGuidanceDefinition {
    #[serde(default)]
    pub prompt_note_if_actor_incomplete: String,
    #[serde(default)]
    pub prompt_note_if_others_incomplete: String,
    #[serde(default)]
    pub prioritize: Vec<RuleBundleAffordancePriorityDefinition>,
    #[serde(default)]
    pub conditional: Vec<RuleBundleConditionalGuidanceDefinition>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuleBundleConditionalGuidanceDefinition {
    #[serde(default)]
    pub required_bundle_progress: Vec<RuleBundleProgressRef>,
    #[serde(default)]
    pub blocked_by_bundle_progress: Vec<RuleBundleProgressRef>,
    #[serde(default)]
    pub prompt_note: String,
    #[serde(default)]
    pub prioritize: Vec<RuleBundleAffordancePriorityDefinition>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuleBundleAffordancePriorityDefinition {
    #[serde(default)]
    pub command_id: String,
    #[serde(default)]
    pub target: RuleBundleAffordanceTarget,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuleBundleAffordanceTarget {
    #[default]
    Any,
    Actor,
    Room,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuleBundleCompletionTrigger {
    SpeechToActor,
    SpeechToRoom,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActorMovementRulesDefinition {
    #[serde(default)]
    pub target_rules: Vec<ActorMovementTargetRuleDefinition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MovementTargetBehavior {
    Move,
    Stay,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActorMovementTargetRuleDefinition {
    #[serde(default)]
    pub target_room_id: String,
    #[serde(default)]
    pub when_player_room_id: String,
    #[serde(default)]
    pub required_story_var: String,
    #[serde(default)]
    pub any_active_stage_ids: Vec<String>,
    #[serde(default)]
    pub target_from_story_var: String,
    #[serde(default)]
    pub target_behavior: Option<MovementTargetBehavior>,
}

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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PresentationDefinition {
    #[serde(default)]
    pub error_text: ErrorTextDefinition,
    #[serde(default)]
    pub presentation_text: PresentationTextDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomExitDefinition {
    pub room_id: String,
    pub label: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    /// Optional label shown in the room-switch menu in place of the
    /// destination room title. When absent the room title is used.
    #[serde(default)]
    pub menu_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomDefinition {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub inspect_text: String,
    #[serde(default)]
    pub allow_rest: bool,
    pub features: Vec<RoomFeatureDefinition>,
    pub exits: Vec<RoomExitDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomFeatureDefinition {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub allow_rest: bool,
    #[serde(default)]
    pub consumables: Vec<ConsumableDefinition>,
    pub inspect_text: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConsumableKind {
    Eat,
    Drink,
    Consume,
    VideoClip,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumableDefinition {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub kind: ConsumableKind,
    #[serde(default)]
    pub initial_stock: u32,
    #[serde(default)]
    pub hunger_recovery: u32,
    #[serde(default)]
    pub stamina_recovery: u32,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorDefinition {
    pub id: String,
    pub name: String,
    pub room_id: String,
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
    /// defeated by the player's attack, item id → count.
    #[serde(default)]
    pub drops: BTreeMap<String, u32>,
    /// XP awarded to the whole party when this actor is defeated.
    #[serde(default)]
    pub xp_drop: u32,
    /// Game minutes between this actor's autonomous hostile strikes. Only used
    /// while the actor is hostile; defaults to 4 when omitted.
    #[serde(default)]
    pub attack_interval_minutes: Option<u32>,
    pub prompt_context: ActorPromptContext,
    #[serde(default)]
    pub act_cast: Option<ActorActCast>,
    #[serde(default)]
    pub game_data: BTreeMap<String, String>,
}

impl ActorDefinition {
    pub fn attack_interval_minutes(&self, default_minutes: u32) -> u32 {
        self.attack_interval_minutes.unwrap_or(default_minutes)
    }
}

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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatDefinition {
    #[serde(default = "default_stat_default_value")]
    pub default: i32,
    #[serde(default)]
    pub min: Option<i32>,
    #[serde(default)]
    pub max: Option<i32>,
    #[serde(default)]
    pub time_step_minutes: Option<u32>,
}

impl StatDefinition {
    pub fn clamp(&self, value: i32) -> i32 {
        let lower = self.min.unwrap_or(i32::MIN);
        let upper = self.max.unwrap_or(i32::MAX);
        value.clamp(lower, upper)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatsDefinition {
    #[serde(default)]
    pub actor: BTreeMap<String, StatDefinition>,
    #[serde(default)]
    pub pair: BTreeMap<String, StatDefinition>,
}
