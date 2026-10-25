//! The content-config shape of actor messaging channels.
//!
//! A pack declares its remote communication channels (e.g. the handler's
//! private comms) in `settings.json`; the engine loads them into
//! [`MessagingChannel`]s and resolves delivery against the world state.
//! Same-room speech is the implicit `local` channel and does not need to be
//! declared; a pack may never claim the `local` id.

use super::Condition;
use serde::{Deserialize, Serialize};

/// The stable channel id of the implicit local (same-room) speech channel.
pub const LOCAL_CHANNEL_ID: &str = "local";

/// The transport semantics of a messaging channel: what must hold for a
/// message on this channel to reach its audience.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelKind {
    /// Co-presence gated: reaches only the actors sharing the speaker's room.
    Local,
    /// Space-independent: reaches the fixed roster wherever they are.
    Direct,
}

/// Who may participate on a channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelPrivacy {
    /// Open to whoever is present (room speech).
    Public,
    /// Restricted to a fixed roster of participants.
    Private,
}

/// An actor messaging channel declared by a pack in `settings.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagingChannel {
    /// Stable channel id, referenced by message-keeping and scripted speech.
    pub id: String,
    /// Transport semantics: presence-gated speech vs space-independent comms.
    pub kind: ChannelKind,
    /// Whether membership is open to presence or a fixed roster.
    pub privacy: ChannelPrivacy,
    /// Conditions that must hold for the channel to be usable. None means it is
    /// always open, as the handler's persistent comms are.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub when: Vec<Condition>,
    /// Fixed participants (actor ids) for a private channel; ignored for
    /// public channels, whose membership is resolved from presence.
    #[serde(default)]
    pub participants: Vec<String>,
    /// Optional human-readable channel name for presentation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}
