mod parsing;
mod prompts;
pub mod scripted;
mod synapse;
pub mod types;

pub use types::*;

pub use self::synapse::{SynapseChapterSummaryGenerator, SynapseDialogueGenerator};

pub use scripted::ScriptedDialogueGenerator;

use crate::content::types::SpeechIntentLabel;

pub(crate) use self::prompts::build_actor_turn_affordance_option;
use self::prompts::{
    actor_turn_decider_system_prompt, build_actor_turn_action_prompt,
    build_conversation_memory_summary_prompt, build_direct_speech_intent_prompt,
    build_hostility_plan_prompt, build_menu_intent_prompt, build_scene_brief_dialogue_prompt,
    dialogue_system_prompt,
};

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

    fn generate_transition_commentary(
        &self,
        request: &TransitionCommentaryRequest,
    ) -> Result<Vec<String>, String> {
        Ok(vec![request.fallback_text.clone()])
    }

    fn generate_comms_dispatch(&self, request: &CommsDispatchRequest) -> Result<String, String> {
        Ok(request.fallback_text.clone())
    }
}

/// Sanitizes generated comms dispatch text by:
/// 1. Stripping speaker prefixes (e.g. "Jamil: ")
/// 2. Removing markdown italic stage directions/sound effects (*mechanical rumble...*)
/// 3. Removing parenthetical sound effects or stage directions ((hisses), (grunts))
/// 4. Removing bracketed meta tags ([GIVE: ...])
/// 5. Trimming stray leading/trailing quotes, dashes, punctuation, and whitespace
pub fn sanitize_comms_dispatch(raw: &str, reporter_name: &str) -> String {
    let mut text = raw
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim()
        .to_string();

    // Strip speaker prefix if present (e.g. "Jamil: ...")
    let prefix = format!("{reporter_name}:");
    if let Some(stripped) = text.strip_prefix(&prefix) {
        text = stripped.trim().to_string();
    } else if text
        .to_ascii_lowercase()
        .starts_with(&prefix.to_ascii_lowercase())
    {
        text = text[prefix.len()..].trim().to_string();
    }

    // Strip bracketed meta tags e.g. [GIVE: sensory-enhancer]
    while let Some(start) = text.find('[') {
        if let Some(end) = text[start..].find(']') {
            text.replace_range(start..=start + end, "");
        } else {
            break;
        }
    }

    // Strip markdown italic blocks (*sound effects*, *stage directions*)
    while let Some(start) = text.find('*') {
        if let Some(end) = text[start + 1..].find('*') {
            text.replace_range(start..=start + 1 + end, "");
        } else {
            break;
        }
    }

    // Strip parenthetical stage directions e.g. (hisses with steam)
    while let Some(start) = text.find('(') {
        if let Some(end) = text[start..].find(')') {
            text.replace_range(start..=start + end, "");
        } else {
            break;
        }
    }

    // Strip leading and trailing punctuation often left over from stage directions (e.g. "—", "-", ":")
    let cleaned = text.trim();
    let trimmed = cleaned
        .trim_start_matches(|c: char| {
            c == '—'
                || c == '-'
                || c == ':'
                || c == ';'
                || c == ','
                || c == '.'
                || c == '"'
                || c == '\''
        })
        .trim_end_matches(|c: char| c == '"' || c == '\'')
        .trim();

    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_comms_dispatch_strips_stage_directions_and_rumble() {
        let raw = "*Low mechanical rumble, the sound of stone grinding against stone, and a sharp brass hiss*—Heavy wounds weaken us at Frost-Leopard. Three foes remain; stand firm, shield the fragile.";
        let cleaned = sanitize_comms_dispatch(raw, "Jamil");
        assert_eq!(
            cleaned,
            "Heavy wounds weaken us at Frost-Leopard. Three foes remain; stand firm, shield the fragile."
        );
    }

    #[test]
    fn test_sanitize_comms_dispatch_strips_name_prefix() {
        let raw = "Einar: Engagement continues at The Iron-Ram Approach; we hold firm with 2 hostiles remaining.";
        let cleaned = sanitize_comms_dispatch(raw, "Einar");
        assert_eq!(
            cleaned,
            "Engagement continues at The Iron-Ram Approach; we hold firm with 2 hostiles remaining."
        );
    }

    #[test]
    fn test_sanitize_comms_dispatch_strips_bracketed_tags() {
        let raw = "All hostiles down at courtyard! [GIVE: key]";
        let cleaned = sanitize_comms_dispatch(raw, "Astrid");
        assert_eq!(cleaned, "All hostiles down at courtyard!");
    }

    #[test]
    fn test_sanitize_comms_dispatch_returns_empty_when_pure_stage_direction() {
        let raw = "*loud metallic clang and groaning*";
        let cleaned = sanitize_comms_dispatch(raw, "Sakhra");
        assert!(cleaned.is_empty());
    }
}
