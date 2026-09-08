use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
    /// Cold system-voice lines layered under `intro_text` at session start,
    /// rendered as `NarrativeLineKind::System` (e.g. sigil teaching lines).
    #[serde(default)]
    pub system_lines: Vec<String>,
    /// Id of a scripted conversation sequence queued to play at session start
    /// (one line per advancing turn, before and alongside normal play). The
    /// opening exchange between Layla's narration and the handler uses this.
    #[serde(default)]
    pub opening_sequence_id: Option<String>,
    pub help_text: String,
    #[serde(default)]
    pub prompt_context: OpeningPromptContext,
}

fn default_opening_start_time_minutes() -> u32 {
    20 * 60
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
