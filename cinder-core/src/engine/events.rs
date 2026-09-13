use crate::content::types::{ContentPack, ItemStorageTarget, PartyOrderKind, SpeechIntentEffect};
use crate::engine::dialogue::DirectSpeechIntentDecision;
use crate::engine::messaging::ChannelMessage;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ObservationMode {
    Summary,
    Detailed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WorldEvent {
    TurnStarted {
        turn_number: u32,
        raw_input: String,
        advances_time: bool,
    },
    CurrentRoomObserved {
        room_id: String,
        mode: ObservationMode,
    },
    FeatureObserved {
        room_id: String,
        feature_id: String,
    },
    ActorObserved {
        actor_id: String,
    },
    /// A message delivered over a messaging channel. Unifies same-room speech
    /// (the implicit public local channel) with remote communication such as
    /// the handler's comms (a private direct channel). See
    /// [`crate::engine::messaging`].
    ChannelMessage {
        message: ChannelMessage,
    },
    ActorStatAdjusted {
        actor_id: String,
        stat: String,
        delta: i32,
    },
    PairStatAdjusted {
        participant_a_id: String,
        participant_b_id: String,
        stat: String,
        delta: i32,
    },
    StoryVarSet {
        key: String,
        value: String,
    },
    ActorCommandUsed {
        actor_id: String,
        actor_name: String,
        room_id: String,
        command_id: String,
        target_room_id: Option<String>,
        target_actor_id: Option<String>,
        target_actor_name: Option<String>,
        context_label: Option<String>,
        feature_id: Option<String>,
        consumable_id: Option<String>,
        freeform_text: Option<String>,
    },
    ActorObservedRoom {
        actor_id: String,
        actor_name: String,
        room_id: String,
    },
    ActorObservedFeature {
        actor_id: String,
        actor_name: String,
        room_id: String,
        feature_id: String,
    },
    ActorObservedActor {
        actor_id: String,
        actor_name: String,
        room_id: String,
        target_actor_id: String,
        target_actor_name: String,
    },
    ActorRelocated {
        actor_id: String,
        to_room_id: String,
    },
    ActorMoved {
        actor_id: String,
        from_room_id: String,
        to_room_id: String,
    },
    PlayerMoved {
        from_room_id: String,
        to_room_id: String,
    },
    MenuOpened {
        menu_id: String,
    },
    MenuChoiceMade {
        menu_id: String,
        option_id: String,
        title: String,
    },
    MenuSelectionToggled {
        menu_id: String,
        option_id: String,
        selected: bool,
    },
    NarrativeLine {
        text: String,
    },
    ActionRejected {
        message: String,
    },
    PartyOrderAssigned {
        actor_id: String,
        order: PartyOrderKind,
    },
    HelpShown,
    UnknownInput {
        raw_input: String,
    },
    ActEnded,
    /// A player moved a loose item from the current room into their
    /// inventory via the generic `take <item>` command.
    PlayerTookItem {
        item_id: String,
    },
    /// A player moved an inventory item into the current room via the generic
    /// `drop <item>` command.
    PlayerDroppedItem {
        item_id: String,
    },
    PlayerEquippedItem {
        item_id: String,
    },
    PlayerUnequippedItem {
        item_id: String,
    },
    ItemAcquired {
        item_id: String,
        storage: ItemStorageTarget,
    },
    ItemConsumed {
        item_id: String,
        storage: ItemStorageTarget,
        consumer_id: Option<String>,
        consumer_name: Option<String>,
    },
    ItemObserved {
        item_id: String,
    },
    CommandObjectiveProgressApplied {
        command_id: String,
    },
    ContentEvent {
        event_id: String,
        payload: BTreeMap<String, String>,
    },
    ActorRelationshipUpdated {
        actor_id: String,
        relationship: crate::engine::state::ActorRelationship,
    },
    /// A hostile actor strikes the player. Declared by tick policies (rules or
    /// LLM); the reducer resolves the mechanics generically from stats.
    HostileStrike {
        actor_id: String,
    },
    /// A content-configured periodic actor effect applies. The reducer looks
    /// up the effect by id, revalidates its trigger and target, and resolves
    /// the generic mechanics.
    PeriodicActorEffectApplied {
        actor_id: String,
        effect_id: String,
    },
    /// Commits that one step of a content-driven scripted conversation
    /// sequence played. The planner emits this alongside the step's content
    /// event so the playhead advance stays in the deterministic event log.
    ScriptedSequenceStepPlayed {
        sequence_id: String,
        next_step: usize,
        finished: bool,
    },
}

pub(crate) fn render_actor_action_text(actor_name: &str, action: &str) -> Result<String, String> {
    let trimmed = action.trim();
    if trimmed.is_empty() {
        return Err("actor action text cannot be empty".to_string());
    }
    let without_actor_name = trimmed
        .strip_prefix(actor_name)
        .map(str::trim_start)
        .unwrap_or(trimmed);
    let normalized = without_actor_name
        .trim_end_matches(['.', '!', '?', ' '])
        .trim();
    if normalized.is_empty() {
        return Err("actor action text cannot be empty".to_string());
    }
    Ok(format!(
        "{actor_name} {}.",
        lowercase_sentence_start(normalized)
    ))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimestampedWorldEvent {
    pub timestamp_ms: u128,
    pub event: WorldEvent,
}

impl TimestampedWorldEvent {
    pub fn now(event: WorldEvent) -> Self {
        Self {
            timestamp_ms: now_millis(),
            event,
        }
    }
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

pub(crate) fn apply_speech_intent_effects(
    content: &ContentPack,
    decision: &DirectSpeechIntentDecision,
    actor_id: &str,
    other_person_id: &str,
) -> Vec<WorldEvent> {
    let label = &decision.0;
    let Some(intent) = content
        .speech_intents
        .intents
        .iter()
        .find(|i| i.label.eq_ignore_ascii_case(label))
    else {
        return Vec::new();
    };
    intent
        .effects
        .iter()
        .map(|effect| match effect {
            SpeechIntentEffect::ActorStat { stat, delta } => WorldEvent::ActorStatAdjusted {
                actor_id: actor_id.to_string(),
                stat: stat.clone(),
                delta: *delta,
            },
            SpeechIntentEffect::PairStat { stat, delta } => WorldEvent::PairStatAdjusted {
                participant_a_id: actor_id.to_string(),
                participant_b_id: other_person_id.to_string(),
                stat: stat.clone(),
                delta: *delta,
            },
        })
        .collect()
}

fn lowercase_sentence_start(text: &str) -> String {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let rest = chars.collect::<String>();
    format!("{}{}", first.to_lowercase(), rest)
}

#[cfg(test)]
mod tests {
    use super::render_actor_action_text;

    #[test]
    fn actor_action_text_normalizes_sentence_start() {
        assert_eq!(
            render_actor_action_text("Aera", "Adjust a chair at the table").unwrap(),
            "Aera adjust a chair at the table."
        );
    }

    #[test]
    fn actor_action_text_strips_repeated_actor_name() {
        assert_eq!(
            render_actor_action_text("Aera", "Aera smooths the stack of mail.").unwrap(),
            "Aera smooths the stack of mail."
        );
    }
}
