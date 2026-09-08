//! Playback of content-driven scripted conversation sequences.
//!
//! A running sequence surfaces exactly one line per advancing turn. The turn
//! planner appends the next step's events (a narration line, or a message on
//! a declared channel) plus a `ScriptedSequenceStepPlayed` commit event; the
//! reducer advances the playhead. Playback is strictly deterministic — the
//! same state always emits the same next line — so the opening exchange and
//! any beat-triggered exchange are replayable from the event log.

use crate::content::types::{
    AdvanceEffect, ContentPack, LOCAL_CHANNEL_ID, MessagingChannel, ScriptedLine, ScriptedSequence,
};
use crate::engine::events::WorldEvent;
use crate::engine::messaging::ChannelAudience;
use crate::engine::state::{WorldState, display_actor_name};

/// Emits the content events for the next step of every running scripted
/// sequence, followed by each sequence's playhead-commit event. Reads only; the
/// reducer commits playheads via the returned events.
pub(crate) fn advance_scripted_sequences(
    content: &ContentPack,
    state: &WorldState,
) -> Vec<WorldEvent> {
    let mut events = Vec::new();
    for (sequence_id, playhead) in &state.scripted_sequences {
        if !playhead.running || playhead.finished {
            continue;
        }
        let Some(sequence) = content.sequence(sequence_id) else {
            eprintln!("[cinder] running scripted sequence '{sequence_id}' missing from content");
            events.push(progress_event(sequence_id, playhead.next_step, true));
            continue;
        };
        if !sequence_gates_hold(sequence, state) {
            continue;
        }
        let Some(next_step) = sequence.steps.get(playhead.next_step) else {
            events.push(progress_event(sequence_id, playhead.next_step, true));
            continue;
        };
        let delivered = match next_step {
            ScriptedLine::Narrate { line } => {
                events.push(WorldEvent::NarrativeLine { text: line.clone() });
                true
            }
            ScriptedLine::Channel {
                channel_id,
                speaker_id,
                recipient_id,
                line,
            } => deliver_channel_step(
                content,
                state,
                channel_id,
                speaker_id,
                recipient_id.as_deref(),
                line,
                &mut events,
            ),
        };
        if !delivered {
            continue;
        }
        let next_index = playhead.next_step + 1;
        let finished = next_index >= sequence.steps.len();
        if finished {
            events.extend(completion_effect_events(sequence));
        }
        events.push(progress_event(sequence_id, next_index, finished));
    }
    events
}

fn sequence_gates_hold(sequence: &ScriptedSequence, state: &WorldState) -> bool {
    sequence
        .gates
        .iter()
        .all(|gate| state.story_vars.get(&gate.story_var) == Some(gate.expected.as_str()))
}

fn deliver_channel_step(
    content: &ContentPack,
    state: &WorldState,
    channel_id: &str,
    speaker_id: &str,
    recipient_id: Option<&str>,
    line: &str,
    events: &mut Vec<WorldEvent>,
) -> bool {
    let channel = if channel_id == LOCAL_CHANNEL_ID {
        MessagingChannel::local()
    } else {
        let Some(channel) = content.channel(channel_id) else {
            eprintln!("[cinder] scripted sequence channel '{channel_id}' missing from content");
            return false;
        };
        channel.clone()
    };
    if !channel.is_available(state) {
        return false;
    }
    let room_id = if channel_id == LOCAL_CHANNEL_ID {
        let Some(speaker) = content.actor(speaker_id) else {
            return false;
        };
        Some(
            state
                .actor_room_id(speaker_id, &speaker.room_id)
                .to_string(),
        )
    } else {
        None
    };
    let audience = match recipient_id {
        Some(recipient_id) => {
            let Some(recipient) = content.actor(recipient_id) else {
                return false;
            };
            ChannelAudience::Targeted {
                recipient_id: recipient_id.to_string(),
                recipient_name: display_actor_name(state, recipient),
            }
        }
        None => ChannelAudience::Broadcast,
    };
    let message = channel.deliver(
        content,
        state,
        speaker_id,
        audience,
        line.to_string(),
        room_id.as_deref(),
    );
    if message.delivery.recipients.is_empty()
        || recipient_id.is_some_and(|recipient| {
            !message
                .delivery
                .recipients
                .iter()
                .any(|actor_id| actor_id == recipient)
        })
    {
        return false;
    }
    events.push(WorldEvent::ChannelMessage { message });
    true
}

fn completion_effect_events(sequence: &ScriptedSequence) -> Vec<WorldEvent> {
    sequence
        .completion_effects
        .iter()
        .map(|effect| match effect {
            AdvanceEffect::AdjustActorStat {
                actor_id,
                stat,
                delta,
            } => WorldEvent::ActorStatAdjusted {
                actor_id: actor_id.clone(),
                stat: stat.clone(),
                delta: *delta,
            },
            AdvanceEffect::AdjustPairStat {
                participant_a_id,
                participant_b_id,
                stat,
                delta,
            } => WorldEvent::PairStatAdjusted {
                participant_a_id: participant_a_id.clone(),
                participant_b_id: participant_b_id.clone(),
                stat: stat.clone(),
                delta: *delta,
            },
            AdvanceEffect::SetStoryVar { key, value } => WorldEvent::StoryVarSet {
                key: key.clone(),
                value: value.clone(),
            },
        })
        .collect()
}

fn progress_event(sequence_id: &str, next_step: usize, finished: bool) -> WorldEvent {
    WorldEvent::ScriptedSequenceStepPlayed {
        sequence_id: sequence_id.to_string(),
        next_step,
        finished,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::types::{
        ChannelAvailability, ChannelKind, ChannelPrivacy,
    };
    use crate::engine::events::TimestampedWorldEvent;
    use crate::engine::reducer::apply_events;
    use crate::engine::test_fixtures::load_test_pack_with_files;

    const SETTINGS: &str = r#"{
      "combat": { "player_actor_id": "blair" },
      "channels": [{
        "id": "comms",
        "kind": "direct",
        "privacy": "private",
        "availability": "always",
        "participants": ["blair", "casey"]
      }]
    }"#;

    const OPENING: &str = r#"{
      "id": "opening",
      "title": "Test Opening",
      "start_room_id": "lounge",
      "start_time_minutes": 1080,
      "intro_text": "Intro",
      "opening_sequence_id": "opening-call",
      "help_text": "Help"
    }"#;

    const SEQUENCES: &str = r#"{
      "sequences": [{
        "id": "opening-call",
        "gates": [{ "story_var": "comms_ready", "expected": "true" }],
        "steps": [
          {
            "voice": "channel",
            "channel_id": "comms",
            "speaker_id": "blair",
            "recipient_id": "casey",
            "line": "Who are you?"
          },
          {
            "voice": "narrate",
            "line": "Static answers before the voice does."
          },
          {
            "voice": "channel",
            "channel_id": "comms",
            "speaker_id": "casey",
            "recipient_id": "blair",
            "line": "Your assigned handler."
          }
        ],
        "completion_effects": [
          {
            "kind": "adjust_actor_stat",
            "actor_id": "blair",
            "stat": "confidence",
            "delta": 2
          },
          {
            "kind": "adjust_pair_stat",
            "participant_a_id": "blair",
            "participant_b_id": "casey",
            "stat": "connection",
            "delta": 1
          },
          {
            "kind": "set_story_var",
            "key": "opening_call_complete",
            "value": "true"
          }
        ]
      }]
    }"#;

    fn scripted_pack() -> ContentPack {
        load_test_pack_with_files(&[
            ("settings.json", SETTINGS),
            ("locales/en/opening.json", OPENING),
            ("locales/en/sequences.json", SEQUENCES),
        ])
    }

    fn apply_world_events(state: &mut WorldState, content: &ContentPack, events: &[WorldEvent]) {
        let timestamped = events
            .iter()
            .cloned()
            .map(|event| TimestampedWorldEvent {
                timestamp_ms: 0,
                event,
            })
            .collect::<Vec<_>>();
        apply_events(state, content, &timestamped);
    }

    #[test]
    fn gates_pause_without_consuming_the_next_step() {
        let content = scripted_pack();
        let mut state = WorldState::new(&content);

        assert!(advance_scripted_sequences(&content, &state).is_empty());
        assert_eq!(
            state
                .scripted_sequence_playhead("opening-call")
                .unwrap()
                .next_step,
            0
        );

        state.story_vars.set_unchecked("comms_ready", "true");
        let events = advance_scripted_sequences(&content, &state);
        let WorldEvent::ChannelMessage { message } = &events[0] else {
            panic!("expected the player's authored channel line");
        };
        assert_eq!(message.speaker_id, "blair");
        assert_eq!(message.delivery.recipients, ["casey"]);
        assert_eq!(message.text, "Who are you?");
    }

    #[test]
    fn playback_preserves_order_and_applies_completion_effects_once() {
        let content = scripted_pack();
        let mut state = WorldState::new(&content);
        state.story_vars.set_unchecked("comms_ready", "true");
        let initial_confidence = state.actor_stat("blair", "confidence");
        let initial_connection = state.pair_stat("blair", "casey", "connection");

        let first = advance_scripted_sequences(&content, &state);
        assert_eq!(first.len(), 2);
        apply_world_events(&mut state, &content, &first);

        let second = advance_scripted_sequences(&content, &state);
        assert!(matches!(
            &second[0],
            WorldEvent::NarrativeLine { text }
                if text == "Static answers before the voice does."
        ));
        apply_world_events(&mut state, &content, &second);

        let third = advance_scripted_sequences(&content, &state);
        let WorldEvent::ChannelMessage { message } = &third[0] else {
            panic!("expected handler channel line");
        };
        assert_eq!(message.speaker_id, "casey");
        assert_eq!(message.delivery.recipients, ["blair"]);
        assert_eq!(third.len(), 5);
        apply_world_events(&mut state, &content, &third);

        assert_eq!(
            state.actor_stat("blair", "confidence"),
            initial_confidence + 2
        );
        assert_eq!(
            state.pair_stat("blair", "casey", "connection"),
            initial_connection + 1
        );
        assert_eq!(state.story_vars.get("opening_call_complete"), Some("true"));
        assert!(advance_scripted_sequences(&content, &state).is_empty());
    }

    #[test]
    fn unavailable_channels_pause_instead_of_losing_lines() {
        let mut content = scripted_pack();
        content.sequences.sequences[0].gates.clear();
        content.settings.channels[0].availability = ChannelAvailability::StoryVarGate {
            story_var: "channel_open".to_string(),
            expected: "true".to_string(),
        };
        let mut state = WorldState::new(&content);

        assert!(advance_scripted_sequences(&content, &state).is_empty());
        assert_eq!(
            state
                .scripted_sequence_playhead("opening-call")
                .unwrap()
                .next_step,
            0
        );

        state.story_vars.set_unchecked("channel_open", "true");
        assert!(!advance_scripted_sequences(&content, &state).is_empty());
    }

    #[test]
    fn local_scripted_lines_require_the_recipient_to_share_the_room() {
        let mut content = scripted_pack();
        content.sequences.sequences[0].gates.clear();
        content.sequences.sequences[0].steps = vec![ScriptedLine::Channel {
            channel_id: LOCAL_CHANNEL_ID.to_string(),
            speaker_id: "blair".to_string(),
            recipient_id: Some("casey".to_string()),
            line: "Stay close.".to_string(),
        }];
        content.settings.channels = vec![MessagingChannel {
            id: "unused".to_string(),
            kind: ChannelKind::Direct,
            privacy: ChannelPrivacy::Private,
            availability: ChannelAvailability::Always,
            participants: vec!["blair".to_string(), "casey".to_string()],
            label: None,
        }];
        let mut state = WorldState::new(&content);
        state
            .actor_room_overrides
            .insert("casey".to_string(), "kitchen".to_string());

        assert!(advance_scripted_sequences(&content, &state).is_empty());
        state
            .actor_room_overrides
            .insert("casey".to_string(), "lounge".to_string());
        assert!(!advance_scripted_sequences(&content, &state).is_empty());
    }
}
