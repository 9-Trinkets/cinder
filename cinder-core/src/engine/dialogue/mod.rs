mod parsing;
mod prompts;
mod synapse;
pub mod scripted;
pub mod types;

pub use types::*;

pub use self::synapse::{SynapseChapterSummaryGenerator, SynapseDialogueGenerator};

pub use scripted::ScriptedDialogueGenerator;

use crate::content::types::SpeechIntentLabel;

use self::prompts::{
    actor_turn_decider_system_prompt, build_actor_turn_action_prompt,
    build_conversation_memory_summary_prompt, build_direct_speech_intent_prompt,
    build_hostility_plan_prompt, build_menu_intent_prompt, build_scene_brief_dialogue_prompt,
    dialogue_system_prompt,
};
pub(crate) use self::prompts::build_actor_turn_affordance_option;

pub fn render_scene_dialogue_prompt(request: &DialogueRequest) -> String {
    build_scene_brief_dialogue_prompt(request)
}

pub fn scene_dialogue_system_prompt_text(request: &DialogueRequest) -> String {
    dialogue_system_prompt(request).to_string()
}

pub fn render_actor_turn_decider_prompt(request: &ActorTurnActionRequest) -> String {
    build_actor_turn_action_prompt(request)
}

pub fn actor_turn_decider_system_prompt_text(request: &ActorTurnActionRequest) -> String {
    actor_turn_decider_system_prompt(request).to_string()
}

pub trait DialogueGenerator: Send + Sync {
    fn build_prompt(&self, request: &DialogueRequest) -> String {
        build_scene_brief_dialogue_prompt(request)
    }

    fn build_actor_turn_action_prompt(&self, request: &ActorTurnActionRequest) -> String {
        build_actor_turn_action_prompt(request)
    }

    fn build_hostility_plan_prompt(&self, request: &HostilityPlanRequest) -> String {
        build_hostility_plan_prompt(request)
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

    fn plan_hostility_actions(
        &self,
        request: &HostilityPlanRequest,
    ) -> Result<HostilityPlanDecision, String>;

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

    fn assign_stage_participants(
        &self,
        request: &StageAssignmentRequest,
    ) -> Result<StageAssignment, String>;

    fn generate_handler_descent_commentary(
        &self,
        request: &HandlerDescentCommentaryRequest,
    ) -> Result<Vec<String>, String> {
        Ok(vec![request.fallback_text.clone()])
    }
}
