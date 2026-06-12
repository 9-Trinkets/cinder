use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObservationMode {
    Summary,
    Detailed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    ActorSpoke {
        actor_id: String,
        actor_name: String,
        other_person_id: String,
        other_person_name: String,
        other_person_message: Option<String>,
        room_id: String,
        text: String,
    },
    ActorSpokeToRoom {
        actor_id: String,
        actor_name: String,
        audience_actor_ids: Vec<String>,
        room_id: String,
        text: String,
    },
    PairStatAdjusted {
        participant_a_id: String,
        participant_b_id: String,
        stat: String,
        delta: i32,
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
    NarrativeLine {
        text: String,
    },
    ActionRejected {
        message: String,
    },
    HelpShown,
    UnknownInput {
        raw_input: String,
    },
    SessionEnded,
    ContentEvent {
        event_id: String,
        payload: BTreeMap<String, String>,
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
