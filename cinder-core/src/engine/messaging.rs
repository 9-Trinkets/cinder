//! A unified model for actor messaging: the shared vocabulary behind both
//! local speech (same-room talk) and remote communication (e.g. the handler's
//! comms channel).
//!
//! The engine previously had only two speech events — targeted and
//! room-broadcast, both implicitly local — and no concept of a channel, so a
//! remote speaker would have had to be either treated as physically present or
//! hardcoded as a system voice. Modelling speech as messages on channels gives
//! both cases one vocabulary while keeping their delivery rules different:
//!
//! - [`ChannelKind::Local`] channels deliver only while the speaker and its
//!   audience share a room (the existing same-room Speak).
//! - [`ChannelKind::Direct`] channels deliver to a fixed roster regardless of
//!   spatial presence (the handler talks while offstage).
//!
//! Channels are typed separately from messages so a message can be described
//! once and resolved per-state (availability, who is present) at delivery
//! time. Local rooms behave as anonymous public channels: the engine describes
//! same-room speech with a local channel and resolves membership from
//! presence. Private channels (the handler's comms) carry an explicit roster.
//!
//! Existing same-room speech maps onto a public local channel and keeps its
//! transcript and conversation-memory behaviour. This module only defines the
//! model; migrating the speak path onto it is a separate change.

use crate::content::types::ContentPack;
use crate::engine::state::{WorldState, display_actor_name};
use serde::{Deserialize, Serialize};

pub use crate::content::types::{
    ChannelAvailability, ChannelKind, ChannelPrivacy, LOCAL_CHANNEL_ID, MessagingChannel,
};

/// Who a channel message is addressed to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "audience", rename_all = "snake_case")]
pub enum ChannelAudience {
    /// Directed at one named participant.
    Targeted {
        recipient_id: String,
        recipient_name: String,
    },
    /// Announced to the whole hearing audience (everyone present for local
    /// speech, the full roster for a direct channel).
    Broadcast,
}

impl MessagingChannel {
    /// Whether the channel can currently be used, honouring its availability
    /// gate (e.g. a story var).
    pub fn is_available(&self, state: &WorldState) -> bool {
        match &self.availability {
            ChannelAvailability::Always => true,
            ChannelAvailability::StoryVarGate {
                story_var,
                expected,
            } => state.story_vars.get(story_var) == Some(expected.as_str()),
        }
    }

    /// Actor ids that hear a message spoken by `speaker_id` on this channel
    /// from `room_id` (`None` for non-spatial direct channels), given the
    /// current world state. Local channels resolve membership from presence;
    /// direct channels resolve membership from the private roster.
    pub fn hearing_audience(
        &self,
        content: &ContentPack,
        state: &WorldState,
        speaker_id: &str,
        room_id: Option<&str>,
    ) -> Vec<String> {
        if !self.is_available(state) {
            return Vec::new();
        }
        match self.kind {
            ChannelKind::Local => room_hearing_audience(content, state, speaker_id, room_id),
            ChannelKind::Direct => {
                let health_stat_id = &content.settings.combat.health_stat_id;
                content
                    .actors
                    .iter()
                    .filter(|actor| self.participants.iter().any(|id| id == &actor.id))
                    .filter(|actor| {
                        actor.id != speaker_id
                            && !state.actor_is_defeated(&actor.id, health_stat_id)
                    })
                    .map(|actor| actor.id.clone())
                    .collect()
            }
        }
    }

    /// Build a channel message for `speaker_id`, resolving the hearing
    /// audience and the speaker's display name from the current world state.
    pub fn deliver(
        &self,
        content: &ContentPack,
        state: &WorldState,
        speaker_id: &str,
        audience: ChannelAudience,
        text: String,
        room_id: Option<&str>,
    ) -> ChannelMessage {
        let mut recipients = self.hearing_audience(content, state, speaker_id, room_id);
        if let ChannelAudience::Targeted { recipient_id, .. } = &audience {
            recipients.retain(|actor_id| actor_id == recipient_id);
        }
        ChannelMessage {
            channel_id: self.id.clone(),
            speaker_id: speaker_id.to_string(),
            speaker_name: content
                .actor(speaker_id)
                .map(|actor| display_actor_name(state, actor))
                .unwrap_or_else(|| speaker_id.to_string()),
            audience,
            in_reply_to: None,
            text,
            delivery: ChannelDelivery {
                kind: self.kind,
                room_id: room_id.map(str::to_string),
                recipients,
            },
        }
    }
}

/// Delivery context of a channel message: where it originates, its transport,
/// and the participants that heard it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelDelivery {
    /// The transport semantics, denormalized onto the delivery so a serialized
    /// message is self-describing for presentation without a channel lookup.
    pub kind: ChannelKind,
    /// The room the message was spoken from; `None` for direct channels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room_id: Option<String>,
    /// Actor ids that received the message (the audience that heard it).
    #[serde(default)]
    pub recipients: Vec<String>,
}

/// A typed channel-aware message event unifying local speech and remote
/// communication.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelMessage {
    pub channel_id: String,
    pub speaker_id: String,
    pub speaker_name: String,
    pub audience: ChannelAudience,
    /// The recipient's preceding line this message answers, when the speaker
    /// is replying (e.g. the player's words an NPC responds to). Used to
    /// avoid duplicating the recipient's line into conversation memory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub in_reply_to: Option<String>,
    pub text: String,
    pub delivery: ChannelDelivery,
}

impl ChannelMessage {
    /// A same-room message directed at one named participant (shared-room
    /// presence enforced by the reducer).
    pub fn targeted(
        speaker: (&str, &str),
        recipient: (&str, &str),
        text: &str,
        room_id: &str,
        in_reply_to: Option<&str>,
    ) -> Self {
        Self {
            channel_id: LOCAL_CHANNEL_ID.to_string(),
            speaker_id: speaker.0.to_string(),
            speaker_name: speaker.1.to_string(),
            audience: ChannelAudience::Targeted {
                recipient_id: recipient.0.to_string(),
                recipient_name: recipient.1.to_string(),
            },
            in_reply_to: in_reply_to.map(str::to_string),
            text: text.to_string(),
            delivery: ChannelDelivery {
                kind: ChannelKind::Local,
                room_id: Some(room_id.to_string()),
                recipients: vec![recipient.0.to_string()],
            },
        }
    }

    /// A same-room announcement to everyone present in `room_id`. The hearing
    /// audience is resolved through the local channel, so shared-room
    /// presence is enforced here.
    pub fn room_broadcast(
        content: &ContentPack,
        state: &WorldState,
        speaker: (&str, &str),
        text: &str,
        room_id: &str,
    ) -> Self {
        Self {
            channel_id: LOCAL_CHANNEL_ID.to_string(),
            speaker_id: speaker.0.to_string(),
            speaker_name: speaker.1.to_string(),
            audience: ChannelAudience::Broadcast,
            in_reply_to: None,
            text: text.to_string(),
            delivery: ChannelDelivery {
                kind: ChannelKind::Local,
                room_id: Some(room_id.to_string()),
                recipients: MessagingChannel::local().hearing_audience(
                    content,
                    state,
                    speaker.0,
                    Some(room_id),
                ),
            },
        }
    }
}

impl MessagingChannel {
    /// The implicit public local channel used for same-room speech.
    pub fn local() -> Self {
        Self {
            id: LOCAL_CHANNEL_ID.to_string(),
            kind: ChannelKind::Local,
            privacy: ChannelPrivacy::Public,
            availability: ChannelAvailability::Always,
            participants: vec![],
            label: None,
        }
    }
}

/// The actors in `room_id` that hear a message: everyone present except the
/// speaker and defeated actors.
fn room_hearing_audience(
    content: &ContentPack,
    state: &WorldState,
    speaker_id: &str,
    room_id: Option<&str>,
) -> Vec<String> {
    let Some(room_id) = room_id else {
        return Vec::new();
    };
    let health_stat_id = &content.settings.combat.health_stat_id;
    content
        .actors
        .iter()
        .filter(|actor| {
            let id = &actor.id;
            id != speaker_id
                && !state.actor_is_defeated(id, health_stat_id)
                && state.actor_is_in_room(content, id, room_id)
        })
        .map(|actor| actor.id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::types::ContentPack;
    use crate::engine::state::WorldState;
    use crate::engine::test_fixtures::minimal_test_pack;

    fn open_roster_state() -> (ContentPack, WorldState) {
        let content = minimal_test_pack();
        let mut state = WorldState::new(&content);
        state.story_vars.set_unchecked("comms_open", "true");
        (content, state)
    }

    fn local_channel() -> MessagingChannel {
        MessagingChannel {
            id: "room".to_string(),
            kind: ChannelKind::Local,
            privacy: ChannelPrivacy::Public,
            availability: ChannelAvailability::Always,
            participants: vec![],
            label: None,
        }
    }

    fn direct_channel() -> MessagingChannel {
        MessagingChannel {
            id: "comms".to_string(),
            kind: ChannelKind::Direct,
            privacy: ChannelPrivacy::Private,
            availability: ChannelAvailability::StoryVarGate {
                story_var: "comms_open".to_string(),
                expected: "true".to_string(),
            },
            participants: vec!["blair".to_string(), "casey".to_string()],
            label: Some("Blair's comms".to_string()),
        }
    }

    #[test]
    fn channels_and_messages_round_trip_through_json() {
        let channel = direct_channel();
        let restored_channel: MessagingChannel =
            serde_json::from_value(serde_json::to_value(&channel).unwrap()).unwrap();
        assert_eq!(channel, restored_channel);

        let (content, state) = open_roster_state();
        let message = channel.deliver(
            &content,
            &state,
            "blair",
            ChannelAudience::Broadcast,
            "Can you hear me?".to_string(),
            None,
        );
        let restored: ChannelMessage =
            serde_json::from_value(serde_json::to_value(&message).unwrap()).unwrap();
        assert_eq!(message, restored);
    }

    #[test]
    fn direct_channel_respects_story_var_gate() {
        let content = minimal_test_pack();
        let mut state = WorldState::new(&content);
        let channel = direct_channel();
        // Story var unset → no message can flow (no audience, empty delivery).
        assert!(!channel.is_available(&state));
        let audience = channel.hearing_audience(&content, &state, "blair", None);
        assert!(audience.is_empty());
        // Story var holds the gate value → roster minus speaker and defeated.
        state.story_vars.set_unchecked("comms_open", "true");
        assert!(channel.is_available(&state));
        let audience = channel.hearing_audience(&content, &state, "blair", None);
        assert_eq!(audience, vec!["casey".to_string()]);
    }

    #[test]
    fn local_channel_audience_is_room_presence() {
        // Casey is in the kitchen while Blair speaks from the lounge: a local
        // message does not reach her, but the direct channel still does.
        let (content, mut state) = open_roster_state();
        state
            .actor_room_overrides
            .insert("casey".to_string(), "kitchen".to_string());
        let room_id = state.current_room_id.clone();

        let local = local_channel();
        let audience = local.hearing_audience(&content, &state, "blair", Some(&room_id));
        let remote = direct_channel().hearing_audience(&content, &state, "blair", None);
        assert!(audience.is_empty());
        assert_eq!(remote, vec!["casey".to_string()]);
    }

    #[test]
    fn local_audience_excludes_offstage_actors_even_when_overridden_into_the_room() {
        let mut content = minimal_test_pack();
        let casey = content
            .actors
            .iter_mut()
            .find(|actor| actor.id == "casey")
            .unwrap();
        casey.room_id.clear();
        let mut state = WorldState::new(&content);
        // A runtime override tries to place the offstage actor in the room:
        // offstage still excludes them from local presence.
        state
            .actor_room_overrides
            .insert("casey".to_string(), "lounge".to_string());
        let audience = local_channel().hearing_audience(&content, &state, "blair", Some("lounge"));
        assert!(audience.is_empty());
    }

    #[test]
    fn direct_channel_delivery_always_carries_recipients() {
        let (content, state) = open_roster_state();
        let channel = direct_channel();
        let message = channel.deliver(
            &content,
            &state,
            "blair",
            ChannelAudience::Broadcast,
            "Channel's live. Can you hear me?".to_string(),
            None,
        );
        assert_eq!(message.delivery.kind, ChannelKind::Direct);
        assert_eq!(message.delivery.room_id, None);
        assert_eq!(message.delivery.recipients, vec!["casey".to_string()]);
        assert_eq!(message.channel_id, "comms");
    }
}
