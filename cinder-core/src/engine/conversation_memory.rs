use crate::content::types::ContentPack;
use crate::engine::dialogue::{ConversationMemorySummaryRequest, DialogueGenerator};
use crate::engine::state::WorldState;

pub fn refresh_conversation_summaries(
    content: &ContentPack,
    dialogue: &dyn DialogueGenerator,
    state: &mut WorldState,
) -> Result<(), String> {
    for key in state.conversation_keys_needing_summary() {
        let (participant_a_id, participant_b_id) = WorldState::conversation_participants(&key)
            .ok_or_else(|| format!("invalid conversation key '{key}'"))?;
        let recent_lines = state
            .conversation_history(participant_a_id, participant_b_id)
            .to_vec();
        if recent_lines.is_empty() {
            continue;
        }
        let request = ConversationMemorySummaryRequest {
            locale: content.locale.clone(),
            system_text: content.system_text.clone(),
            participant_a_id: participant_a_id.to_string(),
            participant_a_name: participant_name(content, participant_a_id),
            participant_b_id: participant_b_id.to_string(),
            participant_b_name: participant_name(content, participant_b_id),
            existing_summary: state
                .conversation_summary(participant_a_id, participant_b_id)
                .map(str::to_string),
            recent_lines,
        };
        let summary = truncate_chars(&dialogue.summarize_conversation_memory(&request)?, 240);
        if summary.is_empty() {
            return Err(format!(
                "conversation memory summarizer returned an empty summary for '{}'",
                key
            ));
        }
        state.set_conversation_summary(participant_a_id, participant_b_id, summary);
    }
    Ok(())
}

fn participant_name(content: &ContentPack, participant_id: &str) -> String {
    if let Some(actor) = content.actor(participant_id) {
        return actor.name.clone();
    }
    if participant_id == format!("viewer:{}", content.opening.id) {
        return content.opening.title.clone();
    }
    participant_id.to_string()
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    text.chars()
        .take(max_chars)
        .collect::<String>()
        .trim()
        .to_string()
}
