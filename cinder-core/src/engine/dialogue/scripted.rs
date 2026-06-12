use super::DialogueGenerator;
use super::types::{
    ActorTurnActionDecision, ActorTurnActionRequest, ConversationMemorySummaryRequest,
    DialogueRequest, DirectSpeechIntentDecision, DirectSpeechIntentRequest,
    DynamicMenuOptionOutput, DynamicMenuRequest, MenuIntentDecision, MenuIntentRequest,
};
use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub struct ScriptedDialogueGenerator {
    replies: BTreeMap<String, String>,
    menu_intents: BTreeMap<String, MenuIntentDecision>,
    actor_turn_actions: BTreeMap<String, ActorTurnActionDecision>,
    memory_summaries: BTreeMap<String, String>,
    attraction_intents: BTreeMap<String, DirectSpeechIntentDecision>,
    requests: std::sync::Arc<std::sync::Mutex<Vec<DialogueRequest>>>,
}

impl ScriptedDialogueGenerator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_reply(mut self, actor_id: &str, reply: &str) -> Self {
        self.replies.insert(actor_id.to_string(), reply.to_string());
        self
    }

    pub fn with_menu_intent(mut self, menu_id: &str, should_open: bool) -> Self {
        self.menu_intents.insert(
            menu_id.to_string(),
            MenuIntentDecision {
                should_open,
                label: if should_open { "OPEN" } else { "PASS" }.to_string(),
            },
        );
        self
    }

    pub fn with_actor_turn_action(
        mut self,
        actor_id: &str,
        decision: ActorTurnActionDecision,
    ) -> Self {
        self.actor_turn_actions
            .insert(actor_id.to_string(), decision);
        self
    }

    pub fn with_memory_summary(
        mut self,
        participant_a_id: &str,
        participant_b_id: &str,
        summary: &str,
    ) -> Self {
        let key = if participant_a_id <= participant_b_id {
            format!("{participant_a_id}::{participant_b_id}")
        } else {
            format!("{participant_b_id}::{participant_a_id}")
        };
        self.memory_summaries.insert(key, summary.to_string());
        self
    }

    pub fn with_attraction_intent(
        mut self,
        actor_id: &str,
        other_person_id: &str,
        decision: DirectSpeechIntentDecision,
    ) -> Self {
        self.attraction_intents
            .insert(format!("{actor_id}::{other_person_id}"), decision);
        self
    }

    pub fn request_log(&self) -> std::sync::Arc<std::sync::Mutex<Vec<DialogueRequest>>> {
        self.requests.clone()
    }
}

impl DialogueGenerator for ScriptedDialogueGenerator {
    fn generate(&self, request: &DialogueRequest) -> Result<String, String> {
        self.requests
            .lock()
            .expect("lock scripted requests")
            .push(request.clone());
        self.replies
            .get(&request.actor_id)
            .cloned()
            .ok_or_else(|| format!("missing scripted reply for '{}'", request.actor_id))
    }

    fn clarify_menu_intent(
        &self,
        request: &MenuIntentRequest,
    ) -> Result<MenuIntentDecision, String> {
        Ok(self
            .menu_intents
            .get(&request.menu_id)
            .cloned()
            .unwrap_or(MenuIntentDecision {
                should_open: false,
                label: "PASS".to_string(),
            }))
    }

    fn choose_actor_turn_action(
        &self,
        request: &ActorTurnActionRequest,
    ) -> Result<ActorTurnActionDecision, String> {
        Ok(self
            .actor_turn_actions
            .get(&request.actor_id)
            .cloned()
            .unwrap_or(ActorTurnActionDecision::Move))
    }

    fn summarize_conversation_memory(
        &self,
        request: &ConversationMemorySummaryRequest,
    ) -> Result<String, String> {
        let key = if request.participant_a_id <= request.participant_b_id {
            format!("{}::{}", request.participant_a_id, request.participant_b_id)
        } else {
            format!("{}::{}", request.participant_b_id, request.participant_a_id)
        };
        Ok(self
            .memory_summaries
            .get(&key)
            .cloned()
            .unwrap_or_else(|| "They are still figuring each other out.".to_string()))
    }

    fn generate_dynamic_menu_options(
        &self,
        _request: &DynamicMenuRequest,
    ) -> Result<Vec<DynamicMenuOptionOutput>, String> {
        Ok(vec![
            DynamicMenuOptionOutput {
                id: "option-1".to_string(),
                title: "Option One".to_string(),
                menu_text: "First generated option".to_string(),
            },
            DynamicMenuOptionOutput {
                id: "option-2".to_string(),
                title: "Option Two".to_string(),
                menu_text: "Second generated option".to_string(),
            },
        ])
    }

    fn extract_direct_speech_intent(
        &self,
        request: &DirectSpeechIntentRequest,
    ) -> Result<DirectSpeechIntentDecision, String> {
        Ok(self
            .attraction_intents
            .get(&format!(
                "{}::{}",
                request.actor_id, request.other_person_id
            ))
            .copied()
            .unwrap_or(DirectSpeechIntentDecision::None))
    }
}
