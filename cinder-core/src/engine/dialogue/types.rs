use crate::content::types::{CommandInputMode, ConsumableKind, SystemTextDefinition};
use crate::engine::commands::TurnAction;
use crate::engine::state::ConversationMemoryLine;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueRequest {
    pub actor_id: String,
    pub actor_name: String,
    pub current_room_id: String,
    pub other_person_id: String,
    pub other_person_name: String,
    pub locale: String,
    pub system_text: SystemTextDefinition,
    pub character_notes: Vec<String>,
    pub setting_notes: Vec<String>,
    pub current_beat_notes: Vec<String>,
    pub subtext_notes: Vec<String>,
    pub behavior_examples: Vec<String>,
    pub response_notes: Vec<String>,
    pub other_person_message: Option<String>,
    pub recent_memory_summary: Option<String>,
    pub recent_memory: Vec<ConversationMemoryLine>,
    #[serde(default = "default_true")]
    pub include_conversation_summary_section: bool,
    #[serde(default = "default_true")]
    pub include_latest_line_section: bool,
}

pub fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuIntentRequest {
    pub menu_id: String,
    pub actor_id: String,
    pub actor_name: String,
    pub other_person_id: String,
    pub other_person_name: String,
    pub locale: String,
    pub system_text: SystemTextDefinition,
    pub setting_notes: Vec<String>,
    pub current_beat_notes: Vec<String>,
    pub recent_memory: Vec<ConversationMemoryLine>,
    pub other_person_message: Option<String>,
    pub intent_guidance: String,
    pub option_titles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuIntentDecision {
    pub should_open: bool,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMemorySummaryRequest {
    pub locale: String,
    pub system_text: SystemTextDefinition,
    pub participant_a_id: String,
    pub participant_a_name: String,
    pub participant_b_id: String,
    pub participant_b_name: String,
    pub existing_summary: Option<String>,
    pub recent_lines: Vec<ConversationMemoryLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterScriptSummaryRequest {
    pub locale: String,
    pub system_text: SystemTextDefinition,
    pub transcript_lines: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterRelationshipSummaryRequest {
    pub locale: String,
    pub system_text: SystemTextDefinition,
    pub pair_stat_lines: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectSpeechIntentRequest {
    pub locale: String,
    pub system_text: SystemTextDefinition,
    pub actor_id: String,
    pub actor_name: String,
    pub other_person_id: String,
    pub other_person_name: String,
    pub current_beat_notes: Vec<String>,
    pub subtext_notes: Vec<String>,
    pub recent_memory: Vec<ConversationMemoryLine>,
    pub other_person_message: Option<String>,
    pub target_person_message: Option<String>,
    pub spoken_line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct DirectSpeechIntentDecision(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorTurnActionRequest {
    pub actor_id: String,
    pub actor_name: String,
    pub locale: String,
    pub system_text: SystemTextDefinition,
    pub character_notes: Vec<String>,
    pub setting_notes: Vec<String>,
    pub current_beat_notes: Vec<String>,
    pub subtext_notes: Vec<String>,
    pub behavior_examples: Vec<String>,
    pub actor_stats: BTreeMap<String, i32>,
    pub has_rest_affordance: bool,
    pub has_hunger_recovery_consumable: bool,
    pub consume_target_item_id: Option<String>,
    pub has_pending_movement_target: bool,
    pub move_target_room_id: Option<String>,
    pub move_target_room_title: Option<String>,
    pub move_target_actor_name: Option<String>,
    pub move_target_social_note: Option<String>,
    pub affordances: Vec<ActorTurnAffordanceOption>,
    pub speak_candidates: Vec<ActorTurnSpeakCandidate>,
    pub recent_memory: Vec<ConversationMemoryLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorTurnAffordanceOption {
    pub affordance_id: String,
    pub command_id: String,
    pub group: String,
    pub available_text: String,
    pub decision_label: String,
    pub decision_prefix: Option<String>,
    pub invocation: ActorTurnCommandInvocation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ActorTurnCommandInvocation {
    Command {
        command_id: String,
        target_room_id: Option<String>,
        target_actor_id: Option<String>,
        feature_id: Option<String>,
        consumable_id: Option<String>,
        context_label: Option<String>,
        input_mode: CommandInputMode,
    },
}

#[derive(Debug, Clone)]
pub enum ActorTurnAffordanceTarget<'a> {
    Move {
        room_id: &'a str,
        room_title: &'a str,
        actor_name: Option<&'a str>,
    },
    Speak {
        actor_id: &'a str,
        actor_name: &'a str,
        reply_now: bool,
    },
    SpeakRoom {
        audience_label: &'a str,
    },
    Hug {
        actor_id: &'a str,
        actor_name: &'a str,
    },
    Rest {
        context_label: &'a str,
    },
    Consume {
        item_id: &'a str,
        item_label: &'a str,
        feature_label: &'a str,
        kind: ConsumableKind,
    },
    InspectFeature {
        feature_id: &'a str,
        feature_label: &'a str,
    },
    InspectActor {
        actor_id: &'a str,
        actor_name: &'a str,
    },
    Act,
}

impl ActorTurnCommandInvocation {
    pub fn into_decision(
        self,
        freeform_text: Option<&str>,
    ) -> Result<ActorTurnActionDecision, String> {
        Ok(match self {
            Self::Command {
                command_id,
                target_room_id,
                target_actor_id,
                feature_id,
                consumable_id,
                context_label,
                input_mode,
            } => {
                let freeform_text = match input_mode {
                    CommandInputMode::FreeformText => Some(
                        freeform_text
                            .map(str::trim)
                            .filter(|text| !text.is_empty())
                            .ok_or_else(|| {
                                format!(
                                    "actor turn decider returned '{}' without the required action text",
                                    command_id
                                )
                            })?
                            .to_string(),
                    ),
                    CommandInputMode::None => None,
                };
                ActorTurnActionDecision::Command {
                    command_id,
                    target_room_id,
                    target_actor_id,
                    feature_id,
                    consumable_id,
                    context_label,
                    freeform_text,
                }
            }
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorTurnSpeakCandidate {
    pub actor_id: String,
    pub actor_name: String,
    pub reply_now: bool,
    pub pair_stats: BTreeMap<String, i32>,
    pub affordances: BTreeMap<String, bool>,
    pub interaction_note: Option<String>,
    pub recent_summary: Option<String>,
    pub recent_memory: Vec<ConversationMemoryLine>,
    pub latest_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorTurnConsumeCandidate {
    pub item_id: String,
    pub item_label: String,
    pub feature_label: String,
    pub kind: ConsumableKind,
    pub hunger_recovery: u32,
}

pub type ActorTurnActionDecision = TurnAction;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicMenuRequest {
    pub locale: String,
    pub system_text: SystemTextDefinition,
    pub role_name: String,
    pub actor_name: String,
    pub character_bio: String,
    pub current_beat_notes: Vec<String>,
    pub recent_memory: Vec<ConversationMemoryLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicMenuOptionOutput {
    pub id: String,
    pub title: String,
    pub menu_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionFeedbackRequest {
    pub locale: String,
    pub system_text: SystemTextDefinition,
    pub actor_name: String,
    pub other_person_name: String,
    pub stats_context: String,
    pub session_summary: String,
    pub relationship_lines: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionFeedback {
    pub rating: u32,
    pub review_text: String,
}
