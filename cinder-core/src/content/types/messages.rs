//! The pack-authored engine message catalog (`messages.json`).
//!
//! Entries are keyed by stable engine message ids. Each entry defaults to
//! world narration (a plain string); a pack can tag an entry as handler-voiced
//! operational feedback so the engine attributes it to the handler's comms
//! channel instead of narrating it.

use serde::{Deserialize, Serialize};

/// Who "speaks" a pack-authored engine message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PackMessageVoice {
    /// The world narrates the line as ordinary prose.
    #[default]
    Narration,
    /// The handler delivers the line over the handler's comms channel.
    Handler,
}

/// A single pack-authored engine message. Plain strings read as world
/// narration; the object form tags the message's voice so a pack can
/// distinguish handler-voiced operational feedback from ordinary narration
/// key by key.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PackMessage {
    /// World narration: `"item.used": "Warmth spreads from the {item}."`
    Narration(String),
    /// Handler-voiced operational feedback:
    /// `{ "voice": "handler", "text": "Got it. {label} is ready." }`
    Handler { text: String },
}

impl PackMessage {
    pub fn text(&self) -> &str {
        match self {
            PackMessage::Narration(text) | PackMessage::Handler { text } => text,
        }
    }

    pub fn voice(&self) -> PackMessageVoice {
        match self {
            PackMessage::Narration(_) => PackMessageVoice::Narration,
            PackMessage::Handler { .. } => PackMessageVoice::Handler,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_strings_deserialize_as_world_narration() {
        let message: PackMessage = serde_json::from_str(r#""Warmth spreads from the {item}.""#)
            .expect("plain string message");
        assert_eq!(message.text(), "Warmth spreads from the {item}.");
        assert_eq!(message.voice(), PackMessageVoice::Narration);
    }

    #[test]
    fn voice_tagged_objects_deserialize_as_handler_messages() {
        let message: PackMessage = serde_json::from_str(
            r#"{"voice": "handler", "text": "Got it. {label} is ready."}"#,
        )
        .expect("voice-tagged message");
        assert_eq!(message.text(), "Got it. {label} is ready.");
        assert_eq!(message.voice(), PackMessageVoice::Handler);
    }

    #[test]
    fn malformed_voice_entries_are_rejected() {
        let error = serde_json::from_str::<PackMessage>(r#"{"voice": "handler"}"#).unwrap_err();
        assert!(error.is_data(), "{error}");
    }
}