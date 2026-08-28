pub use super::text_defs::{
    ActClosureDefinition, ActClosureSectionDefinition, ActClosureSource, ShellMenuDefinition,
    ShellMenuItem, SystemTextDefinition, UiTextDefinition,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::collections::{BTreeMap, BTreeSet};

mod theme;
pub use theme::ThemeDefinition;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpeningDefinition {
    pub id: String,
    pub title: String,
    pub start_room_id: String,
    /// Optional list of candidate spawn rooms. When non-empty, the player
    /// starts in one chosen uniformly at random (fog of war: no fixed
    /// spawn). `start_room_id` remains the fallback when this is empty.
    #[serde(default)]
    pub start_room_ids: Vec<String>,
    #[serde(default = "default_opening_start_time_minutes")]
    pub start_time_minutes: u32,
    pub intro_text: String,
    pub help_text: String,
    #[serde(default)]
    pub prompt_context: OpeningPromptContext,
}

fn default_opening_start_time_minutes() -> u32 {
    20 * 60
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentSettingsDefinition {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub tagline: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_typewriter_char_ms")]
    pub typewriter_char_ms: u64,
    #[serde(default = "default_npc_tick_interval_ms")]
    pub npc_tick_interval_ms: u64,
    #[serde(default = "default_tick_minutes_per_turn")]
    pub tick_minutes_per_turn: u32,
    #[serde(default = "default_default_language")]
    pub default_language: String,
    #[serde(default)]
    pub channel_surfing_only: bool,
    #[serde(default)]
    pub autonomous_actor_dialogue: bool,
    #[serde(default)]
    pub workflow_id: String,
    #[serde(default)]
    pub closure_perspective_actor_id: String,
    #[serde(default)]
    pub act_member_alias: String,
    #[serde(default)]
    pub fallback_stage_id: String,
    #[serde(default)]
    pub fallback_required_story_vars: Vec<String>,
    #[serde(default = "default_true")]
    pub show_act_closure: bool,
    #[serde(default)]
    pub show_relationship_sidebar: bool,
    /// Room-id prefix that, once the player has travelled to a room on that
    /// board, reveals party levels in the sidebar. Empty means levels are
    /// always visible. Treats level info as a reward and foreshadows a
    /// hierarchy (e.g. a chess board) when the player descends.
    #[serde(default)]
    pub level_reveal_room_prefix: String,
    /// How autonomous hostile strikes are decided on background ticks.
    /// `rules` selects deterministically; `llm` asks a validated LLM planner.
    #[serde(default)]
    pub autonomous_hostility_mode: AutonomousHostilityMode,
    /// Binds the generic strike mechanism to this pack's stat vocabulary.
    #[serde(default)]
    pub combat: CombatSettingsDefinition,
    /// Items the player starts with, item id → count. Seed a finite resource
    /// (e.g. layla's stone markers) here so it can be dropped into rooms and
    /// picked back up.
    #[serde(default)]
    pub starting_items: BTreeMap<String, u32>,
    /// Fixed equipment slot list (e.g. "weapon", "armor", "trinket"). Each
    /// slot holds at most one equipped item; equippable items name one of
    /// these slots and their bonuses feed effective stat reads.
    #[serde(default)]
    pub equipment_slots: BTreeSet<String>,
    #[serde(default)]
    pub theme: ThemeDefinition,
}

fn default_typewriter_char_ms() -> u64 {
    40
}

fn default_npc_tick_interval_ms() -> u64 {
    2_000
}

fn default_tick_minutes_per_turn() -> u32 {
    1
}

pub(super) fn default_stat_default_value() -> i32 {
    0
}

pub(super) fn default_actor_targeted_speech() -> String {
    "{actor_name} (to {target_name}): {text}".to_string()
}

fn default_default_language() -> String {
    "en".to_string()
}

pub(super) fn default_true() -> bool {
    true
}

/// Policy selector for autonomous hostile strikes on background ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutonomousHostilityMode {
    /// Deterministic: every eligible hostile actor in the player's room
    /// strikes when its cooldown elapses.
    #[default]
    Rules,
    /// An LLM planner receives a grounded world snapshot and returns validated
    /// strike decisions. The reducer still enforces eligibility, so the model
    /// can never bypass pacing or range rules.
    Llm,
}

/// Content-declared vocabulary for the generic hostile-strike mechanism.
/// The engine never assumes specific stat ids or a player actor id; packs
/// bind the mechanism to their own stats by filling this in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CombatSettingsDefinition {
    /// Actor id that strikes are directed at and whose death ends the game.
    #[serde(default = "default_player_actor_id")]
    pub player_actor_id: String,
    /// Stat id consulted for alive/defeated checks.
    #[serde(default = "default_health_stat_id")]
    pub health_stat_id: String,
    /// Stat id read from the attacker when computing damage.
    #[serde(default = "default_attack_stat_id")]
    pub attack_stat_id: String,
    /// Stat id read from the defender when computing damage.
    #[serde(default = "default_defense_stat_id")]
    pub defense_stat_id: String,
    /// Lower bound for a single strike's damage after mitigation.
    #[serde(default = "default_minimum_damage")]
    pub minimum_damage: i32,
    /// Cooldown used when an actor does not declare its own interval.
    #[serde(default = "default_attack_interval_minutes")]
    pub default_attack_interval_minutes: u32,
    /// Narration shown when the player actor's health stat reaches zero.
    #[serde(default = "default_player_defeat_text")]
    pub player_defeat_text: String,
}

/// One step of growth for an actor. `exp_required` is the XP needed to
/// advance from the *current* level (the entry's index + 1) to the next.
/// `stat_changes` are deltas applied on reaching that next level; `unlocks`
/// reserves space for future skills/spells earned at that level.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LevelDefinition {
    #[serde(default)]
    pub exp_required: u32,
    #[serde(default)]
    pub stat_changes: BTreeMap<String, i32>,
    /// Identifiers of skills/spells granted by reaching this level. Not yet
    /// consumed by the engine; declared so the schema is stable for when
    /// abilities land.
    #[serde(default)]
    pub unlocks: Vec<String>,
}

/// A single advance table for one actor (or the shared default). Index `n`
/// governs the transition from level `n+1` to `n+2`.
pub type LevelTable = Vec<LevelDefinition>;

/// Leveling rules declared by the content pack. The `default` table applies
/// to every actor unless that actor has a per-actor override in `actors`,
/// enabling differentiated classes/jobs to advance on their own curves.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LevelingDefinition {
    #[serde(default)]
    pub default: LevelTable,
    /// Per-actor override tables, keyed by actor id. An actor without an
    /// entry falls back to `default`.
    #[serde(default)]
    pub actors: BTreeMap<String, LevelTable>,
}

impl Default for CombatSettingsDefinition {
    fn default() -> Self {
        Self {
            player_actor_id: default_player_actor_id(),
            health_stat_id: default_health_stat_id(),
            attack_stat_id: default_attack_stat_id(),
            defense_stat_id: default_defense_stat_id(),
            minimum_damage: default_minimum_damage(),
            default_attack_interval_minutes: default_attack_interval_minutes(),
            player_defeat_text: default_player_defeat_text(),
        }
    }
}

fn default_player_actor_id() -> String {
    "player".to_string()
}

fn default_health_stat_id() -> String {
    "hp".to_string()
}

fn default_attack_stat_id() -> String {
    "strength".to_string()
}

fn default_defense_stat_id() -> String {
    "defense".to_string()
}

fn default_minimum_damage() -> i32 {
    1
}

pub(super) fn default_attack_interval_minutes() -> u32 {
    4
}

fn default_player_defeat_text() -> String {
    "The world tilts. Your legs give out. The last thing you feel is the cold stone beneath your palms."
        .to_string()
}

impl Default for ContentSettingsDefinition {
    fn default() -> Self {
        Self {
            title: String::default(),
            tagline: String::default(),
            description: String::default(),
            typewriter_char_ms: default_typewriter_char_ms(),
            npc_tick_interval_ms: default_npc_tick_interval_ms(),
            tick_minutes_per_turn: default_tick_minutes_per_turn(),
            default_language: default_default_language(),
            channel_surfing_only: false,
            autonomous_actor_dialogue: false,
            closure_perspective_actor_id: String::default(),
            act_member_alias: String::default(),
            fallback_stage_id: String::default(),
            fallback_required_story_vars: Vec::new(),
            workflow_id: String::default(),
            show_act_closure: true,
            show_relationship_sidebar: false,
            level_reveal_room_prefix: String::default(),
            autonomous_hostility_mode: AutonomousHostilityMode::Rules,
            combat: CombatSettingsDefinition::default(),
            starting_items: BTreeMap::new(),
            equipment_slots: BTreeSet::new(),
            theme: ThemeDefinition::default(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpeningPromptContext {
    #[serde(default)]
    pub setting_notes: Vec<String>,
    #[serde(default)]
    pub subtext_notes: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActCastMember {
    pub id: String,
    pub name: String,
    pub actor_id: String,
    #[serde(default)]
    pub inspect_blurb: String,
    #[serde(default)]
    pub intro_blurb: String,
    #[serde(default)]
    pub return_blurb: String,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
    #[serde(default)]
    pub actor_stats: BTreeMap<String, i32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum AdvanceSignal {
    Simple(String),
    Conditional {
        signal: String,
        #[serde(default)]
        conditions: Vec<AdvanceCondition>,
    },
}

impl AdvanceSignal {
    pub fn signal(&self) -> &str {
        match self {
            Self::Simple(s) => s,
            Self::Conditional { signal, .. } => signal,
        }
    }
    pub fn conditions(&self) -> &[AdvanceCondition] {
        match self {
            Self::Simple(_) => &[],
            Self::Conditional { conditions, .. } => conditions,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AdvanceCondition {
    pub path: String,
    pub operator: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AdvanceEffect {
    AdjustActorStat {
        actor_id: String,
        stat: String,
        delta: i32,
    },
    AdjustPairStat {
        participant_a_id: String,
        participant_b_id: String,
        stat: String,
        delta: i32,
    },
    SetStoryVar {
        key: String,
        value: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageAssignmentDefinition {
    #[serde(default)]
    pub selection_label: String,
    #[serde(default)]
    pub prompt_instructions: String,
    #[serde(default)]
    pub initiator_actor_id: String,
    #[serde(default)]
    pub selected_room_id: String,
    #[serde(default)]
    pub remaining_room_id: String,
    #[serde(default)]
    pub selected_room_story_var: String,
    #[serde(default)]
    pub remaining_room_story_var: String,
    #[serde(default)]
    pub selected_host_story_var: String,
    #[serde(default)]
    pub remaining_host_story_var: String,
    #[serde(default = "default_stage_assignment_max_selected")]
    pub max_selected_actors: usize,
    #[serde(default = "default_stage_assignment_min_selected")]
    pub min_selected_actors: usize,
    #[serde(default = "default_stage_assignment_score_threshold")]
    pub score_threshold: i32,
    #[serde(default)]
    pub initiator_line_template: String,
    #[serde(default)]
    pub selected_line_template: String,
    #[serde(default)]
    pub remaining_line_template: String,
    #[serde(default)]
    pub group_story_var_key: String,
    #[serde(default)]
    pub remaining_group_story_var_key: String,
}

impl Default for StageAssignmentDefinition {
    fn default() -> Self {
        Self {
            selection_label: String::new(),
            prompt_instructions: String::new(),
            initiator_actor_id: String::new(),
            selected_room_id: String::new(),
            remaining_room_id: String::new(),
            selected_room_story_var: String::new(),
            remaining_room_story_var: String::new(),
            selected_host_story_var: String::new(),
            remaining_host_story_var: String::new(),
            max_selected_actors: default_stage_assignment_max_selected(),
            min_selected_actors: default_stage_assignment_min_selected(),
            score_threshold: default_stage_assignment_score_threshold(),
            initiator_line_template: String::new(),
            selected_line_template: String::new(),
            remaining_line_template: String::new(),
            group_story_var_key: String::new(),
            remaining_group_story_var_key: String::new(),
        }
    }
}

fn default_stage_assignment_max_selected() -> usize {
    2
}

fn default_stage_assignment_min_selected() -> usize {
    1
}

fn default_stage_assignment_score_threshold() -> i32 {
    50
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SpeechIntentEffect {
    ActorStat { stat: String, delta: i32 },
    PairStat { stat: String, delta: i32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechIntentLabel {
    pub label: String,
    pub description: String,
    #[serde(default)]
    pub effects: Vec<SpeechIntentEffect>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpeechIntentsConfig {
    #[serde(default)]
    pub intents: Vec<SpeechIntentLabel>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BeatsDefinition {
    #[serde(default)]
    pub initial_stage_ids: Vec<String>,
    #[serde(default)]
    pub stages: Vec<BeatDefinition>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BeatDefinition {
    pub id: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub beat_note: String,
    #[serde(default)]
    pub update_message: String,
    #[serde(default)]
    pub next_chapter_preview: String,
    #[serde(default)]
    pub actor_relocations: Vec<ActorRelocationDefinition>,
    #[serde(default)]
    pub narrative_lines: Vec<String>,
    #[serde(default)]
    pub elapsed_minutes: u32,
    #[serde(default)]
    pub projector_sequence_var_key: String,
    #[serde(default)]
    pub end_act: bool,
    #[serde(default)]
    pub end_game: bool,
    #[serde(default)]
    pub advance_signals: Vec<AdvanceSignal>,
    #[serde(default)]
    pub next_stage_ids: Vec<String>,
    #[serde(default)]
    pub on_advance_effects: Vec<AdvanceEffect>,
    #[serde(default)]
    pub stage_assignment: Option<StageAssignmentDefinition>,
    #[serde(default)]
    pub open_menu: String,
    #[serde(default)]
    pub target_actor_story_var: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MenuTriggerMode {
    Agreement,
    IntentClarified,
    #[default]
    AnySpeak,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpeningMenuDefinition {
    pub id: String,
    #[serde(default)]
    pub actor_id: String,
    #[serde(default)]
    pub stage_id: String,
    #[serde(default)]
    pub trigger_mode: MenuTriggerMode,
    #[serde(default)]
    pub dynamic: bool,
    #[serde(default)]
    pub generation_role: String,
    #[serde(default)]
    pub proposal_line: String,
    #[serde(default)]
    pub intent_guidance: String,
    #[serde(default)]
    pub selection_prompt: String,
    #[serde(default)]
    pub invalid_choice_text: String,
    #[serde(default)]
    pub selection_confirmation: String,
    #[serde(default)]
    pub selection_var_key: String,
    #[serde(default)]
    pub selection_id_var_key: String,
    #[serde(default)]
    pub max_selections: usize,
    #[serde(default)]
    pub min_selections: usize,
    #[serde(default)]
    pub multi_selection_var_keys: Vec<String>,
    #[serde(default)]
    pub multi_selection_room_var_keys: Vec<String>,
    #[serde(default)]
    pub multi_selection_host_var_keys: Vec<String>,
    #[serde(default)]
    pub opening_narrative_lines: Vec<String>,
    #[serde(default)]
    pub narrative_lines: Vec<String>,
    #[serde(default)]
    pub options: Vec<OpeningMenuOptionDefinition>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActorRelocationDefinition {
    pub actor_id: String,
    pub to_room_id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpeningMenuOptionDefinition {
    pub id: String,
    pub title: String,
    pub menu_text: String,
    #[serde(default)]
    pub narrative_lines: Vec<String>,
    #[serde(default)]
    pub room_id: String,
    #[serde(default)]
    pub host_actor_id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpeningMovieDefinition {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub match_value: String,
    #[serde(default)]
    pub frames: Vec<OpeningMovieFrameDefinition>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpeningMovieFrameDefinition {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub text_path: String,
    #[serde(default)]
    pub duration_ms: u64,
}

mod world_defs;
pub use world_defs::{
    ActorDefinition, ActorMovementRulesDefinition, ActorMovementTargetRuleDefinition,
    ActorPromptContext, ConsumableDefinition, ConsumableKind, ErrorTextDefinition,
    MovementConfigDefinition, MovementTargetBehavior, PresentationDefinition,
    PresentationTextDefinition, RoomDefinition, RoomDescriptionOverride, RoomExitDefinition,
    RoomFeatureDefinition,
    RuleBundleAffordancePriorityDefinition, RuleBundleAffordanceTarget,
    RuleBundleCompletionDefinition, RuleBundleCompletionTrigger,
    RuleBundleConditionalGuidanceDefinition, RuleBundleDefinition, RuleBundleGuidanceDefinition,
    RuleBundleProgressDefinition, RuleBundleProgressKeyDefinition, RuleBundlesDefinition,
    SpeechConfigDefinition, StatDefinition, StatsDefinition,
};

mod command_defs;
pub use command_defs::{
    CommandEffect, CommandInputMode, CommandOutcomeMode, CommandTargetMode, ContentEventDefinition,
    ItemConsumerTarget, ItemStorageTarget, PlayerCommandInputMetadata, PlayerCommandMetadata,
    PlayerCommandTargetMode, RuleBundleProgressRef,
};

mod action_defs;
pub use action_defs::{
    ActionAvailability, ActionContentEvent, ActionDefinition, ActionItemConsumerTarget,
    ActionItemCreation, ActionItemStorageTarget, ActionNpc, ActionPlayerCommand, ActionPlayerInput,
    ActionUi, ActionsDefinition, PanelConfig, PanelDataSource, PanelSelectAction,
};

mod content_pack;
pub use content_pack::{ContentPack, RoomConsumableRef};

mod item_defs;
pub use item_defs::{ItemDefinition, ItemKind};
