use crate::content::types::{
    ContentPack, OpeningMenuOptionDefinition, RoomDefinition, StatDefinition,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldState {
    pub current_room_id: String,
    pub turn_number: u32,
    pub current_time_minutes: u32,
    pub game_over: bool,
    pub conversation_event_sequence: u64,
    pub conversation_memory: BTreeMap<String, Vec<ConversationMemoryLine>>,
    pub conversation_summaries: BTreeMap<String, ConversationSummaryState>,
    pub actor_stat_defs: BTreeMap<String, StatDefinition>,
    pub pair_stat_defs: BTreeMap<String, StatDefinition>,
    pub pair_stats: BTreeMap<String, BTreeMap<String, i32>>,
    pub actor_stats: BTreeMap<String, BTreeMap<String, i32>>,
    pub pending_replies: BTreeMap<String, PendingReplyState>,
    pub active_objective_stage_ids: Vec<String>,
    pub actor_room_overrides: BTreeMap<String, String>,
    #[serde(default)]
    pub stages_completed: usize,
    pub feature_consumable_stock: BTreeMap<String, u32>,
    pub followed_actor_id: Option<String>,
    pub active_menu_id: Option<String>,
    #[serde(default)]
    pub generated_menu_options: HashMap<String, Vec<OpeningMenuOptionDefinition>>,
    pub pending_projector_sequence_id: Option<String>,
    pub pending_projector_narrative_lines: Vec<String>,
    pub story_vars: BTreeMap<String, String>,
    pub actor_known_room_ids: BTreeMap<String, BTreeSet<String>>,
    pub actor_observed_room_ids: BTreeMap<String, BTreeSet<String>>,
    pub actor_known_feature_ids: BTreeMap<String, BTreeSet<String>>,
    pub actor_known_actor_ids: BTreeMap<String, BTreeSet<String>>,
    pub actor_recent_observation_notes: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub initial_actor_stats: BTreeMap<String, BTreeMap<String, i32>>,
    #[serde(default)]
    pub initial_pair_stats: BTreeMap<String, BTreeMap<String, i32>>,
    #[serde(default)]
    pub transcript: Vec<String>,
    #[serde(default)]
    pub player_inventory: HashMap<String, u32>,
    #[serde(default)]
    pub appointment_series: Option<AppointmentSeriesState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMemoryLine {
    pub turn_number: u32,
    #[serde(default)]
    pub event_sequence: u64,
    pub speaker_id: String,
    pub speaker_name: String,
    #[serde(default)]
    pub kind: ConversationMemoryKind,
    #[serde(default)]
    pub target_label: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConversationSummaryState {
    pub summary: Option<String>,
    pub pending_line_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PendingReplyState {
    pub speaker_id: String,
    pub listener_id: String,
    pub room_id: String,
    pub turn_number: u32,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConversationMemoryKind {
    #[default]
    Speech,
    Action,
}

impl WorldState {
    pub fn new(content: &ContentPack) -> Self {
        Self {
            current_room_id: content.opening.start_room_id.clone(),
            turn_number: 0,
            current_time_minutes: content.opening.start_time_minutes,
            game_over: false,
            conversation_event_sequence: 0,
            conversation_memory: BTreeMap::new(),
            conversation_summaries: BTreeMap::new(),
            actor_stat_defs: content.stats.actor.clone(),
            pair_stat_defs: content.stats.pair.clone(),
            pair_stats: seeded_pair_stats(content, &content.stats.pair),
            actor_stats: seeded_actor_stats(content, &content.stats.actor),
            pending_replies: BTreeMap::new(),
            active_objective_stage_ids: content.beats.initial_stage_ids.clone(),
            actor_room_overrides: BTreeMap::new(),
            stages_completed: 0,
            feature_consumable_stock: seeded_feature_consumable_stock(content),
            followed_actor_id: None,
            active_menu_id: None,
            generated_menu_options: HashMap::new(),
            pending_projector_sequence_id: None,
            pending_projector_narrative_lines: Vec::new(),
            story_vars: BTreeMap::new(),
            actor_known_room_ids: content
                .actors
                .iter()
                .map(|actor| (actor.id.clone(), BTreeSet::from([actor.room_id.clone()])))
                .collect(),
            actor_observed_room_ids: BTreeMap::new(),
            actor_known_feature_ids: BTreeMap::new(),
            actor_known_actor_ids: BTreeMap::new(),
            actor_recent_observation_notes: BTreeMap::new(),
            initial_actor_stats: seeded_actor_stats(content, &content.stats.actor),
            initial_pair_stats: seeded_pair_stats(content, &content.stats.pair),
            transcript: Vec::new(),
            player_inventory: HashMap::new(),
            appointment_series: None,
        }
    }

    pub fn current_room<'a>(&self, content: &'a ContentPack) -> Option<&'a RoomDefinition> {
        content.room(&self.current_room_id)
    }

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

    pub fn actor_room_id<'a>(&'a self, actor_id: &str, default_room_id: &'a str) -> &'a str {
        self.actor_room_overrides
            .get(actor_id)
            .map(String::as_str)
            .unwrap_or(default_room_id)
    }

    pub fn pair_stat(
        &self,
        first_participant_id: &str,
        second_participant_id: &str,
        stat_key: &str,
    ) -> i32 {
        self.pair_stats
            .get(&Self::conversation_key(
                first_participant_id,
                second_participant_id,
            ))
            .and_then(|stats| stats.get(stat_key))
            .copied()
            .unwrap_or_else(|| {
                self.pair_stat_defs
                    .get(stat_key)
                    .map(|stat| stat.default)
                    .unwrap_or(0)
            })
    }

    pub fn actor_stat(&self, actor_id: &str, stat_key: &str) -> i32 {
        self.actor_stats
            .get(actor_id)
            .and_then(|stats| stats.get(stat_key))
            .copied()
            .unwrap_or_else(|| {
                self.actor_stat_defs
                    .get(stat_key)
                    .map(|stat| stat.default)
                    .unwrap_or(0)
            })
    }

    pub fn actor_stat_u32(&self, actor_id: &str, stat_key: &str) -> u32 {
        self.actor_stat(actor_id, stat_key).max(0) as u32
    }

    pub fn actor_stats_snapshot(&self, actor_id: &str) -> BTreeMap<String, i32> {
        self.actor_stat_defs
            .keys()
            .map(|stat_key| (stat_key.clone(), self.actor_stat(actor_id, stat_key)))
            .collect()
    }

    pub fn pair_stats_snapshot(
        &self,
        first_participant_id: &str,
        second_participant_id: &str,
    ) -> BTreeMap<String, i32> {
        self.pair_stat_defs
            .keys()
            .map(|stat_key| {
                (
                    stat_key.clone(),
                    self.pair_stat(first_participant_id, second_participant_id, stat_key),
                )
            })
            .collect()
    }

    pub fn actor_stat_deltas(&self, actor_id: &str) -> Option<BTreeMap<String, i32>> {
        let initial = self.initial_actor_stats.get(actor_id)?;
        Some(
            initial
                .keys()
                .map(|stat_key| {
                    let current = self.actor_stat(actor_id, stat_key);
                    let init = initial.get(stat_key).copied().unwrap_or(0);
                    (stat_key.clone(), current - init)
                })
                .collect(),
        )
    }

    pub fn pair_stat_deltas(
        &self,
        first_id: &str,
        second_id: &str,
    ) -> Option<BTreeMap<String, i32>> {
        let key = Self::conversation_key(first_id, second_id);
        let initial = self.initial_pair_stats.get(&key)?;
        Some(
            initial
                .keys()
                .map(|stat_key| {
                    let current = self.pair_stat(first_id, second_id, stat_key);
                    let init = initial.get(stat_key).copied().unwrap_or(0);
                    (stat_key.clone(), current - init)
                })
                .collect(),
        )
    }

    pub fn pair_stat_u32(
        &self,
        first_participant_id: &str,
        second_participant_id: &str,
        stat_key: &str,
    ) -> u32 {
        self.pair_stat(first_participant_id, second_participant_id, stat_key)
            .max(0) as u32
    }

    pub fn adjust_pair_stat(
        &mut self,
        first_participant_id: &str,
        second_participant_id: &str,
        stat_key: &str,
        delta: i32,
    ) -> Result<(), String> {
        let definition = self
            .pair_stat_defs
            .get(stat_key)
            .ok_or_else(|| format!("unknown pair stat '{stat_key}'"))?
            .clone();
        let stats = self
            .pair_stats
            .entry(Self::conversation_key(
                first_participant_id,
                second_participant_id,
            ))
            .or_default();
        let value = stats
            .entry(stat_key.to_string())
            .or_insert(definition.default);
        *value = definition.clamp(*value + delta);
        Ok(())
    }

    pub fn adjust_actor_stat(
        &mut self,
        actor_id: &str,
        stat_key: &str,
        delta: i32,
    ) -> Result<(), String> {
        let definition = self
            .actor_stat_defs
            .get(stat_key)
            .ok_or_else(|| format!("unknown actor stat '{stat_key}'"))?
            .clone();
        let stats = self.actor_stats.entry(actor_id.to_string()).or_default();
        let value = stats
            .entry(stat_key.to_string())
            .or_insert(definition.default);
        *value = definition.clamp(*value + delta);
        Ok(())
    }

    pub fn remaining_consumable_stock(
        &self,
        room_id: &str,
        feature_id: &str,
        consumable_id: &str,
    ) -> u32 {
        self.feature_consumable_stock
            .get(&consumable_key(room_id, feature_id, consumable_id))
            .copied()
            .unwrap_or(0)
    }

    pub fn consume_feature_consumable(
        &mut self,
        room_id: &str,
        feature_id: &str,
        consumable_id: &str,
    ) -> bool {
        let key = consumable_key(room_id, feature_id, consumable_id);
        let Some(stock) = self.feature_consumable_stock.get_mut(&key) else {
            return false;
        };
        if *stock == 0 {
            return false;
        }
        *stock -= 1;
        true
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

    fn conversation_key(first_participant_id: &str, second_participant_id: &str) -> String {
        if first_participant_id <= second_participant_id {
            format!("{first_participant_id}::{second_participant_id}")
        } else {
            format!("{second_participant_id}::{first_participant_id}")
        }
    }

    pub fn current_time_label(&self) -> String {
        format_clock_time(self.current_time_minutes)
    }

    pub fn current_day_number(&self) -> u32 {
        (self.current_time_minutes / MINUTES_PER_DAY) + 1
    }

    pub fn current_time_note(&self) -> String {
        format!("It is {}.", self.current_time_label())
    }

    pub fn actor_has_visited_room(&self, actor_id: &str, room_id: &str) -> bool {
        self.actor_known_room_ids
            .get(actor_id)
            .is_some_and(|rooms| rooms.contains(room_id))
    }

    pub fn mark_actor_room_visited(&mut self, actor_id: &str, room_id: &str) {
        self.actor_known_room_ids
            .entry(actor_id.to_string())
            .or_default()
            .insert(room_id.to_string());
    }

    pub fn actor_has_seen_feature(&self, actor_id: &str, room_id: &str, feature_id: &str) -> bool {
        self.actor_known_feature_ids
            .get(actor_id)
            .is_some_and(|features| features.contains(&feature_key(room_id, feature_id)))
    }

    pub fn mark_actor_feature_seen(&mut self, actor_id: &str, room_id: &str, feature_id: &str) {
        self.actor_known_feature_ids
            .entry(actor_id.to_string())
            .or_default()
            .insert(feature_key(room_id, feature_id));
    }

    pub fn actor_has_studied_actor(&self, actor_id: &str, target_actor_id: &str) -> bool {
        self.actor_known_actor_ids
            .get(actor_id)
            .is_some_and(|actors| actors.contains(target_actor_id))
    }

    pub fn mark_actor_studied_actor(&mut self, actor_id: &str, target_actor_id: &str) {
        self.actor_known_actor_ids
            .entry(actor_id.to_string())
            .or_default()
            .insert(target_actor_id.to_string());
    }

    pub fn push_actor_observation_note(&mut self, actor_id: &str, note: String) {
        let notes = self
            .actor_recent_observation_notes
            .entry(actor_id.to_string())
            .or_default();
        notes.push(note);
        if notes.len() > 6 {
            notes.drain(..notes.len() - 6);
        }
    }

    pub fn actor_recent_observation_notes(&self, actor_id: &str) -> &[String] {
        self.actor_recent_observation_notes
            .get(actor_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn actor_has_observed_room(&self, actor_id: &str, room_id: &str) -> bool {
        self.actor_observed_room_ids
            .get(actor_id)
            .is_some_and(|rooms| rooms.contains(room_id))
    }

    pub fn has_item(&self, item_id: &str) -> bool {
        self.player_inventory
            .get(item_id)
            .copied()
            .unwrap_or(0)
            > 0
    }

    pub fn item_count(&self, item_id: &str) -> u32 {
        self.player_inventory
            .get(item_id)
            .copied()
            .unwrap_or(0)
    }

    pub fn add_item(&mut self, item_id: &str) {
        *self
            .player_inventory
            .entry(item_id.to_string())
            .or_insert(0) += 1;
    }

    pub fn remove_item(&mut self, item_id: &str) -> bool {
        let mut has = false;
        if let Some(count) = self.player_inventory.get_mut(item_id) {
            if *count > 0 {
                *count -= 1;
                has = true;
            }
        }
        if let Some(0) = self.player_inventory.get(item_id) {
            self.player_inventory.remove(item_id);
        }
        has
    }

    pub fn mark_actor_observed_room(&mut self, actor_id: &str, room_id: &str) {
        self.actor_observed_room_ids
            .entry(actor_id.to_string())
            .or_default()
            .insert(room_id.to_string());
    }
}

fn feature_key(room_id: &str, feature_id: &str) -> String {
    format!("{room_id}::{feature_id}")
}

fn consumable_key(room_id: &str, feature_id: &str, consumable_id: &str) -> String {
    format!("{room_id}::{feature_id}::{consumable_id}")
}

pub const MAX_CONVERSATION_RECENT_LINES: usize = 4;
pub const CONVERSATION_SUMMARY_TRIGGER_LINES: usize = 4;
const MINUTES_PER_DAY: u32 = 24 * 60;

fn format_clock_time(total_minutes: u32) -> String {
    let hour24 = (total_minutes / 60) % 24;
    let minute = total_minutes % 60;
    let meridiem = if hour24 >= 12 { "PM" } else { "AM" };
    let hour12 = match hour24 % 12 {
        0 => 12,
        hour => hour,
    };
    format!("{hour12}:{minute:02} {meridiem}")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnOutcome {
    pub text: String,
    pub game_over: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSnapshot {
    pub turn_number: u32,
    pub current_room_id: String,
}

mod seeding;
use seeding::{seeded_actor_stats, seeded_feature_consumable_stock, seeded_pair_stats};
mod appointments;
pub use appointments::{
    advance_to_next_appointment, current_appointment_intro, current_patient_name,
    display_actor_name, initialize_appointment_state, render_dynamic_story_text,
    resolved_actor_prompt_context, AppointmentFeedbackSummary, AppointmentHistoryEntry,
    AppointmentSeriesState, PatientRecord,
};
