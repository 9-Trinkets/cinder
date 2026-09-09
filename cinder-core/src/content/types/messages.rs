//! The pack-authored engine message catalog (`messages.json`).
//!
//! Entries are keyed by stable engine message ids. Each entry defaults to
//! world narration (a plain string); a pack can tag operational feedback as
//! automated system output or as an explicit handler takeover.

use serde::{Deserialize, Deserializer, Serialize};

/// Who "speaks" a pack-authored engine message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PackMessageVoice {
    /// The world narrates the line as ordinary prose.
    #[default]
    Narration,
    /// Automated operational feedback rendered in the pack's system style.
    System,
    /// The handler delivers the line over the handler's comms channel.
    Handler,
}

/// A single pack-authored engine message. Plain strings read as world
/// narration; the object form tags the message's delivery so a pack can
/// distinguish automated system feedback, handler commentary, and prose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum PackMessage {
    /// World narration: `"item.used": "Warmth spreads from the {item}."`
    Narration(String),
    /// Explicitly delivered operational feedback:
    /// `{ "voice": "system", "text": "INVENTORY UPDATED: {label}." }`
    /// or `{ "voice": "handler", "text": "Keep that close." }`.
    Voiced {
        voice: PackMessageVoice,
        text: String,
    },
    /// Compatibility with handler messages serialized before voice selection
    /// was represented explicitly. New content should use `voice: handler`.
    LegacyHandler { text: String },
}

impl PackMessage {
    pub fn text(&self) -> &str {
        match self {
            PackMessage::Narration(text)
            | PackMessage::Voiced { text, .. }
            | PackMessage::LegacyHandler { text } => text,
        }
    }

    pub fn voice(&self) -> PackMessageVoice {
        match self {
            PackMessage::Narration(_) => PackMessageVoice::Narration,
            PackMessage::Voiced { voice, .. } => *voice,
            PackMessage::LegacyHandler { .. } => PackMessageVoice::Handler,
        }
    }
}

impl<'de> Deserialize<'de> for PackMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Representation {
            Narration(String),
            Object {
                #[serde(default)]
                voice: Option<PackMessageVoice>,
                text: String,
            },
        }

        match Representation::deserialize(deserializer)? {
            Representation::Narration(text) => Ok(Self::Narration(text)),
            Representation::Object {
                voice: Some(voice),
                text,
            } => Ok(Self::Voiced { voice, text }),
            Representation::Object { voice: None, text } => Ok(Self::LegacyHandler { text }),
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
        let message: PackMessage =
            serde_json::from_str(r#"{"voice": "handler", "text": "Got it. {label} is ready."}"#)
                .expect("voice-tagged message");
        assert_eq!(message.text(), "Got it. {label} is ready.");
        assert_eq!(message.voice(), PackMessageVoice::Handler);
    }

    #[test]
    fn system_voiced_objects_deserialize_as_system_messages() {
        let message: PackMessage =
            serde_json::from_str(r#"{"voice": "system", "text": "INVENTORY UPDATED: {label}."}"#)
                .expect("system-voiced message");
        assert_eq!(message.text(), "INVENTORY UPDATED: {label}.");
        assert_eq!(message.voice(), PackMessageVoice::System);
    }

    #[test]
    fn legacy_unvoiced_objects_remain_handler_messages() {
        let message: PackMessage = serde_json::from_str(r#"{"text": "Legacy handler line."}"#)
            .expect("legacy handler message");
        assert_eq!(message.text(), "Legacy handler line.");
        assert_eq!(message.voice(), PackMessageVoice::Handler);
    }

    #[test]
    fn malformed_voice_entries_are_rejected() {
        let error = serde_json::from_str::<PackMessage>(r#"{"voice": "handler"}"#).unwrap_err();
        assert!(error.is_data(), "{error}");

        let error = serde_json::from_str::<PackMessage>(r#"{"voice": "robot", "text": "No."}"#)
            .unwrap_err();
        assert!(error.is_data(), "{error}");
    }
}
