pub use super::text_defs::{
    ActionBarDefinition, ActionBarItem, ShellMenuDefinition, ShellMenuItem, SystemTextDefinition,
    UiTextDefinition,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::collections::HashMap;

mod theme;
pub use theme::ThemeDefinition;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpeningDefinition {
    pub id: String,
    pub title: String,
    pub start_room_id: String,
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
    #[serde(default = "default_speech_stamina_cost_floor")]
    pub speech_stamina_cost_floor: i32,
    #[serde(default = "default_true")]
    pub show_day_summary: bool,
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

fn default_speech_stamina_cost_floor() -> i32 {
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

impl Default for ContentSettingsDefinition {
    fn default() -> Self {
        Self {
            typewriter_char_ms: default_typewriter_char_ms(),
            npc_tick_interval_ms: default_npc_tick_interval_ms(),
            tick_minutes_per_turn: default_tick_minutes_per_turn(),
            default_language: default_default_language(),
            channel_surfing_only: false,
            autonomous_actor_dialogue: false,
            speech_stamina_cost_floor: default_speech_stamina_cost_floor(),
            workflow_id: String::default(),
            show_day_summary: true,
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
    pub end_session: bool,
    #[serde(default)]
    pub advance_signals: Vec<AdvanceSignal>,
    #[serde(default)]
    pub next_stage_id: Option<String>,
    #[serde(default)]
    pub next_stage_ids: Vec<String>,
    #[serde(default)]
    pub on_advance_effects: Vec<AdvanceEffect>,
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
    pub actor_relocations: Vec<ActorRelocationDefinition>,
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
    PresentationDefinition, PresentationTextDefinition, RoomDefinition, RoomExitDefinition,
    RoomFeatureDefinition, StatDefinition, StatsDefinition,
};

mod command_defs;
pub use command_defs::{
    AffordanceDefinition, AffordancesDefinition, CommandDefinition, CommandEffect,
    CommandInputMode, CommandOutcomeMode, CommandTargetMode, CommandsDefinition,
    ContentEventDefinition, PlayerCommandInputMetadata, PlayerCommandMetadata,
    PlayerCommandTargetMode,
};

mod content_pack;
pub use content_pack::{ContentPack, RoomConsumableRef};

mod item_defs;
pub use item_defs::ItemDefinition;
