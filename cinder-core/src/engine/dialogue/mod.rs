mod parsing;
mod prompts;
#[cfg(test)]
mod scripted;
pub mod types;

pub use types::*;

#[cfg(test)]
pub use scripted::ScriptedDialogueGenerator;

use self::parsing::{
    ActorTurnActionParseContext, parse_actor_turn_action, parse_direct_speech_intent_label,
    parse_menu_intent_label,
};
pub(crate) use self::prompts::build_actor_turn_affordance_option;
use self::prompts::{
    actor_turn_decider_system_prompt, build_actor_turn_action_prompt,
    build_chapter_relationship_summary_prompt, build_chapter_script_summary_prompt,
    build_conversation_memory_summary_prompt, build_direct_speech_intent_prompt,
    build_menu_intent_prompt, build_scene_brief_dialogue_prompt,
    chapter_relationship_summarizer_system_prompt, chapter_script_summarizer_system_prompt,
    conversation_memory_summarizer_system_prompt, dialogue_system_prompt,
    direct_speech_intent_system_prompt, menu_intent_system_prompt, sanitize_statement,
};

use crate::content::types::SpeechIntentLabel;
use crate::engine::neuron::{
    NeuronRoleService, RoleExecutionError, RoleExecutionResponse, RoleMetadata, WorkflowDefinition,
};
use serde_json::json;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

const ACTOR_DIALOGUE_ROLE: &str = "actor_dialogue";
const MENU_INTENT_CLARIFIER_ROLE: &str = "menu_intent_clarifier";
const ACTOR_TURN_DECIDER_ROLE: &str = "actor_turn_decider";
const CONVERSATION_MEMORY_SUMMARIZER_ROLE: &str = "conversation_memory_summarizer";
const CHAPTER_SCRIPT_SUMMARIZER_ROLE: &str = "chapter_script_summarizer";
const CHAPTER_RELATIONSHIP_SUMMARIZER_ROLE: &str = "chapter_relationship_summarizer";
const DIRECT_SPEECH_ATTRACTION_INTENT_ROLE: &str = "direct_speech_intent";
const PERSPECTIVE_REVIEW_ROLE: &str = "session_feedback";
const CONVERSATION_MEMORY_SUMMARY_TIMEOUT: Duration = Duration::from_secs(10);
const VALIDATED_ROLE_MAX_ATTEMPTS: usize = 4;

pub trait DialogueGenerator: Send + Sync {
    fn build_prompt(&self, request: &DialogueRequest) -> String {
        build_scene_brief_dialogue_prompt(request)
    }

    fn build_actor_turn_action_prompt(&self, request: &ActorTurnActionRequest) -> String {
        build_actor_turn_action_prompt(request)
    }

    fn build_menu_intent_prompt(&self, request: &MenuIntentRequest) -> String {
        build_menu_intent_prompt(request)
    }

    fn build_conversation_memory_summary_prompt(
        &self,
        request: &ConversationMemorySummaryRequest,
    ) -> String {
        build_conversation_memory_summary_prompt(request)
    }

    fn build_direct_speech_intent_prompt(
        &self,
        request: &DirectSpeechIntentRequest,
        intents: &[SpeechIntentLabel],
    ) -> String {
        build_direct_speech_intent_prompt(request, intents)
    }

    fn trace_metadata(&self, _role_name: &str) -> serde_json::Value {
        serde_json::Value::Null
    }

    fn generate(&self, request: &DialogueRequest) -> Result<String, String>;

    fn clarify_menu_intent(
        &self,
        request: &MenuIntentRequest,
    ) -> Result<MenuIntentDecision, String>;

    fn choose_actor_turn_action(
        &self,
        request: &ActorTurnActionRequest,
    ) -> Result<ActorTurnActionDecision, String>;

    fn summarize_conversation_memory(
        &self,
        request: &ConversationMemorySummaryRequest,
    ) -> Result<String, String>;

    fn extract_direct_speech_intent(
        &self,
        request: &DirectSpeechIntentRequest,
        intents: &[SpeechIntentLabel],
    ) -> Result<DirectSpeechIntentDecision, String>;

    fn generate_dynamic_menu_options(
        &self,
        request: &DynamicMenuRequest,
    ) -> Result<Vec<DynamicMenuOptionOutput>, String>;

    fn generate_perspective_review(
        &self,
        request: &PerspectiveReviewRequest,
    ) -> Result<PerspectiveReview, String>;
}

pub struct SynapseDialogueGenerator {
    workflow: WorkflowDefinition,
    service: NeuronRoleService,
}

fn build_role_service() -> Result<NeuronRoleService, String> {
    let config_path = PathBuf::from(env!("CINDER_PROJECT_DIR")).join("neuron.toml");
    let dotenv_path = PathBuf::from(env!("CINDER_PROJECT_DIR")).join(".env");
    NeuronRoleService::new_with_config_path_and_dotenv_path(&config_path, &dotenv_path)
        .map_err(|error| format!("failed to initialize role execution service: {error}"))
}

impl SynapseDialogueGenerator {
    pub fn new(workflow: WorkflowDefinition) -> Result<Self, String> {
        let service = build_role_service()?;
        Ok(Self { workflow, service })
    }

    fn run_text_role(
        &self,
        role_name: &str,
        prompt: String,
        fallback_system_prompt: String,
    ) -> Result<String, String> {
        self.run_text_role_detailed(role_name, prompt, fallback_system_prompt)
            .map(|response| response.text)
            .map_err(|error| error.to_string())
    }

    fn run_text_role_detailed(
        &self,
        role_name: &str,
        prompt: String,
        fallback_system_prompt: String,
    ) -> Result<RoleExecutionResponse, Box<RoleExecutionError>> {
        self.service.execute_role_detailed(
            &self.workflow,
            role_name,
            prompt,
            Some(fallback_system_prompt),
        )
    }

    fn run_validated_text_role<T, F>(
        &self,
        role_name: &'static str,
        prompt: String,
        fallback_system_prompt: String,
        parse: F,
    ) -> Result<T, String>
    where
        F: Fn(&str) -> Result<T, String>,
    {
        let mut validation_errors = Vec::new();
        for attempt in 1..=VALIDATED_ROLE_MAX_ATTEMPTS {
            let response = self
                .run_text_role_detailed(role_name, prompt.clone(), fallback_system_prompt.clone())
                .map_err(format_role_execution_error)?;
            match parse(&response.text) {
                Ok(parsed) => return Ok(parsed),
                Err(error) => validation_errors.push(format!(
                    "attempt {attempt}: {:?} ({error})",
                    response.text.trim()
                )),
            }
        }
        Err(format!(
            "role '{role_name}' returned invalid output after {VALIDATED_ROLE_MAX_ATTEMPTS} attempts:\n{}",
            validation_errors.join("\n")
        ))
    }

    fn preview_role(&self, role_name: &str) -> Result<RoleMetadata, String> {
        self.service.preview_role(&self.workflow, role_name)
    }

    fn run_text_role_with_timeout(
        &self,
        role_name: &'static str,
        prompt: String,
        fallback_system_prompt: String,
        timeout: Duration,
    ) -> Result<String, String> {
        let workflow = self.workflow.clone();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = build_role_service()
                .and_then(|service| {
                    service
                        .execute_role(&workflow, role_name, prompt, Some(fallback_system_prompt))
                        .map_err(|error| error.to_string())
                })
                .map(|text| text.trim().to_string());
            let _ = tx.send(result);
        });
        match rx.recv_timeout(timeout) {
            Ok(result) => result,
            Err(mpsc::RecvTimeoutError::Timeout) => Err(format!(
                "role '{role_name}' timed out after {}s",
                timeout.as_secs()
            )),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                Err(format!("role '{role_name}' worker disconnected"))
            }
        }
    }
}

pub struct SynapseChapterSummaryGenerator {
    workflow: WorkflowDefinition,
    service: NeuronRoleService,
}

impl SynapseChapterSummaryGenerator {
    pub fn new(workflow: WorkflowDefinition) -> Result<Self, String> {
        let service = build_role_service()?;
        Ok(Self { workflow, service })
    }

    pub fn summarize_script(
        &self,
        request: &ChapterScriptSummaryRequest,
    ) -> Result<String, String> {
        self.service
            .execute_role(
                &self.workflow,
                CHAPTER_SCRIPT_SUMMARIZER_ROLE,
                build_chapter_script_summary_prompt(request),
                Some(chapter_script_summarizer_system_prompt(request).to_string()),
            )
            .map(|text| text.trim().to_string())
            .map_err(|error| error.to_string())
    }

    pub fn summarize_relationships(
        &self,
        request: &ChapterRelationshipSummaryRequest,
    ) -> Result<String, String> {
        self.service
            .execute_role(
                &self.workflow,
                CHAPTER_RELATIONSHIP_SUMMARIZER_ROLE,
                build_chapter_relationship_summary_prompt(request),
                Some(chapter_relationship_summarizer_system_prompt(request).to_string()),
            )
            .map(|text| text.trim().to_string())
            .map_err(|error| error.to_string())
    }
}

impl DialogueGenerator for SynapseDialogueGenerator {
    fn build_prompt(&self, request: &DialogueRequest) -> String {
        build_scene_brief_dialogue_prompt(request)
    }

    fn trace_metadata(&self, role_name: &str) -> serde_json::Value {
        match self.preview_role(role_name) {
            Ok(metadata) => json!({
                "backend": metadata.backend,
                "planner_mode": metadata.planner_mode,
                "model": metadata.model,
                "agent_profile": metadata.agent_profile,
                "base_url": metadata.base_url,
            }),
            Err(error) => json!({ "error": error }),
        }
    }

    fn generate(&self, request: &DialogueRequest) -> Result<String, String> {
        self.run_text_role(
            ACTOR_DIALOGUE_ROLE,
            self.build_prompt(request),
            dialogue_system_prompt(request).to_string(),
        )
    }

    fn clarify_menu_intent(
        &self,
        request: &MenuIntentRequest,
    ) -> Result<MenuIntentDecision, String> {
        self.run_validated_text_role(
            MENU_INTENT_CLARIFIER_ROLE,
            self.build_menu_intent_prompt(request),
            menu_intent_system_prompt(request).to_string(),
            parse_menu_intent_label,
        )
    }

    fn choose_actor_turn_action(
        &self,
        request: &ActorTurnActionRequest,
    ) -> Result<ActorTurnActionDecision, String> {
        self.run_validated_text_role(
            ACTOR_TURN_DECIDER_ROLE,
            self.build_actor_turn_action_prompt(request),
            actor_turn_decider_system_prompt(request).to_string(),
            |text| {
                parse_actor_turn_action(
                    text,
                    &ActorTurnActionParseContext {
                        affordances: &request.affordances,
                    },
                )
            },
        )
    }

    fn summarize_conversation_memory(
        &self,
        request: &ConversationMemorySummaryRequest,
    ) -> Result<String, String> {
        let key = format!("{}::{}", request.participant_a_id, request.participant_b_id);
        self.run_text_role_with_timeout(
            CONVERSATION_MEMORY_SUMMARIZER_ROLE,
            self.build_conversation_memory_summary_prompt(request),
            conversation_memory_summarizer_system_prompt(request).to_string(),
            CONVERSATION_MEMORY_SUMMARY_TIMEOUT,
        )
        .map(|text| sanitize_statement(&text))
        .map_err(|error| format!("conversation summary failed for '{key}': {error}"))
    }

    fn extract_direct_speech_intent(
        &self,
        request: &DirectSpeechIntentRequest,
        intents: &[SpeechIntentLabel],
    ) -> Result<DirectSpeechIntentDecision, String> {
        let response = self.run_text_role(
            DIRECT_SPEECH_ATTRACTION_INTENT_ROLE,
            self.build_direct_speech_intent_prompt(request, intents),
            direct_speech_intent_system_prompt(request).to_string(),
        )?;
        parse_direct_speech_intent_label(&response)
    }

    fn generate_perspective_review(
        &self,
        request: &PerspectiveReviewRequest,
    ) -> Result<PerspectiveReview, String> {
        let prompt = format!(
            r#"Patient: {actor_name}
Therapist: {other_person_name}

Session Outcome
{stats_context}

Session Summary
{session_summary}

Relationship Notes
{relationship_lines}

Write a short Yelp-style review from {actor_name}'s perspective about their therapy session with {other_person_name}. Be specific and in character.

Return ONLY valid JSON (no markdown, no backticks) in this exact format:
{{"rating": <1-5>, "review_text": "<the review text>"}}

The rating (1-5 stars) should reflect the patient's genuine experience based on the session outcome.
The review text should be 2-5 sentences in the patient's voice — honest, specific, and a bit Yelp-flavored."#,
            actor_name = request.actor_name,
            other_person_name = request.other_person_name,
            stats_context = request.stats_context,
            session_summary = request.session_summary,
            relationship_lines = request.relationship_lines.join("\n"),
        );
        let response = self.run_text_role(
            PERSPECTIVE_REVIEW_ROLE,
            prompt,
            "You write a short Yelp-style review from a patient's perspective. Respond only with valid JSON."
                .to_string(),
        )?;
        serde_json::from_str::<PerspectiveReview>(&response)
            .map_err(|e| format!("failed to parse perspective review: {e}"))
    }

    fn generate_dynamic_menu_options(
        &self,
        request: &DynamicMenuRequest,
    ) -> Result<Vec<DynamicMenuOptionOutput>, String> {
        let bio = &request.character_bio;
        let beats = request.current_beat_notes.join("\n");
        let guidance = request.intent_guidance.trim();
        let recent = request
            .recent_memory
            .iter()
            .map(|m| format!("{}: {}", m.speaker_name, m.text))
            .collect::<Vec<_>>()
            .join("\n");
        let role_specific_instructions = if request.role_name == "book_recommender" {
            "Generate exactly 3 fictional book recommendations. Each option must be a plausible novel title paired with a one-line thematic blurb that fits this specific patient and this specific conversation. None of the options should be framed as the correct answer."
        } else {
            "Generate exactly 3 options that fit this specific character and this specific conversation."
        };
        let prompt = format!(
            r#"Menu id: {menu_id}

Menu prompt:
{menu_prompt}

Intent guidance:
{guidance}

Context about {actor_name}:
{bio}

Session context:
{beats}

Recent conversation:
{recent}

{role_specific_instructions}

Return ONLY valid JSON (no markdown, no backticks) in this exact format:
[
  {{"id": "kebab-case-option-id", "title": "Short Display Title", "menu_text": "Brief description of this option"}},
  ...
]

The "id" must be a unique kebab-case slug derived from the title.
The "title" is a short label shown as the option name.
The "menu_text" is a one-line description shown below the title.
Make the options feel distinct from each other and grounded in the recent conversation rather than generic defaults."#,
            menu_id = request.menu_id,
            menu_prompt = request.menu_prompt,
            guidance = guidance,
            actor_name = request.actor_name,
            role_specific_instructions = role_specific_instructions,
        );
        let response = self.run_text_role(
            &request.role_name,
            prompt,
            "You generate menu options for a dialogue-driven game. Respond only with valid JSON."
                .to_string(),
        )?;
        serde_json::from_str::<Vec<DynamicMenuOptionOutput>>(&response)
            .map_err(|e| format!("failed to parse dynamic menu options: {e}"))
    }
}

fn format_role_execution_error(error: Box<RoleExecutionError>) -> String {
    if error.plan_rejections.is_empty() {
        return error.to_string();
    }
    let rejection_lines = error
        .plan_rejections
        .iter()
        .enumerate()
        .map(|(index, rejection)| {
            if let Some(step_text) = rejection.step_text.as_deref() {
                format!(
                    "{}. {} => {:?} ({})",
                    index + 1,
                    rejection.step_kind,
                    step_text,
                    rejection.error_message
                )
            } else if let Some(tool_name) = rejection.tool_name.as_deref() {
                format!(
                    "{}. {} {} {:?} ({})",
                    index + 1,
                    rejection.step_kind,
                    tool_name,
                    rejection.tool_args,
                    rejection.error_message
                )
            } else {
                format!(
                    "{}. {} ({})",
                    index + 1,
                    rejection.step_kind,
                    rejection.error_message
                )
            }
        })
        .collect::<Vec<_>>()
        .join(" | ");
    format!(
        "{}. Rejected planner steps: {}",
        error.message, rejection_lines
    )
}
