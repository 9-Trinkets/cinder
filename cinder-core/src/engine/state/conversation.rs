use super::*;

pub const MAX_CONVERSATION_RECENT_LINES: usize = 4;
pub const CONVERSATION_SUMMARY_TRIGGER_LINES: usize = 4;

impl WorldState {
    pub fn conversation_history(
        &self,
        first_participant_id: &str,
        second_participant_id: &str,
    ) -> &[ConversationMemoryLine] {
        self.conversation_memory
            .get(&Self::conversation_key(
                first_participant_id,
                second_participant_id,
            ))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn push_conversation_line(
        &mut self,
        first_participant_id: &str,
        second_participant_id: &str,
        mut line: ConversationMemoryLine,
    ) {
        line.event_sequence = self.conversation_event_sequence;
        self.conversation_event_sequence = self.conversation_event_sequence.saturating_add(1);
        let key = Self::conversation_key(first_participant_id, second_participant_id);
        let lines = self.conversation_memory.entry(key.clone()).or_default();
        lines.push(line);
        if lines.len() > MAX_CONVERSATION_RECENT_LINES {
            lines.drain(..lines.len() - MAX_CONVERSATION_RECENT_LINES);
        }
        self.conversation_summaries
            .entry(key)
            .or_default()
            .pending_line_count += 1;
    }

    pub fn conversation_summary(
        &self,
        first_participant_id: &str,
        second_participant_id: &str,
    ) -> Option<&str> {
        self.conversation_summaries
            .get(&Self::conversation_key(
                first_participant_id,
                second_participant_id,
            ))
            .and_then(|state| state.summary.as_deref())
    }

    pub fn set_conversation_summary(
        &mut self,
        first_participant_id: &str,
        second_participant_id: &str,
        summary: String,
    ) {
        let state = self
            .conversation_summaries
            .entry(Self::conversation_key(
                first_participant_id,
                second_participant_id,
            ))
            .or_default();
        state.summary = Some(summary);
        state.pending_line_count = 0;
    }

    pub fn conversation_keys_needing_summary(&self) -> Vec<String> {
        self.conversation_summaries
            .iter()
            .filter(|(_, state)| state.pending_line_count >= CONVERSATION_SUMMARY_TRIGGER_LINES)
            .map(|(key, _)| key.clone())
            .collect()
    }

    pub fn conversation_participants(key: &str) -> Option<(&str, &str)> {
        let (first, second) = key.split_once("::")?;
        Some((first, second))
    }

    pub fn set_pending_reply(
        &mut self,
        speaker_id: &str,
        listener_id: &str,
        room_id: &str,
        turn_number: u32,
    ) {
        self.pending_replies.insert(
            Self::conversation_key(speaker_id, listener_id),
            PendingReplyState {
                speaker_id: speaker_id.to_string(),
                listener_id: listener_id.to_string(),
                room_id: room_id.to_string(),
                turn_number,
            },
        );
    }

    pub fn pending_reply(
        &self,
        first_participant_id: &str,
        second_participant_id: &str,
    ) -> Option<&PendingReplyState> {
        self.pending_replies.get(&Self::conversation_key(
            first_participant_id,
            second_participant_id,
        ))
    }

    pub fn clear_pending_reply(&mut self, first_participant_id: &str, second_participant_id: &str) {
        self.pending_replies.remove(&Self::conversation_key(
            first_participant_id,
            second_participant_id,
        ));
    }

    pub fn clear_stale_pending_replies(&mut self) {
        let turn_number = self.turn_number;
        self.pending_replies
            .retain(|_, pending| pending.turn_number + 1 >= turn_number);
    }

    pub(crate) fn conversation_key(first_participant_id: &str, second_participant_id: &str) -> String {
        if first_participant_id <= second_participant_id {
            format!("{first_participant_id}::{second_participant_id}")
        } else {
            format!("{second_participant_id}::{first_participant_id}")
        }
    }
}
