use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The single source of truth for speech-selection policy in a content pack.
/// It controls whether actors speak, whether they prefer direct speech, and
/// when room-addressed speech is available.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechConfigDefinition {
    #[serde(default)]
    pub should_speak_when: Option<Value>,
    #[serde(default)]
    pub should_direct_speech_when: Option<Value>,
    #[serde(default = "default_speech_room_min_audience")]
    pub room_min_audience: usize,
}

impl Default for SpeechConfigDefinition {
    fn default() -> Self {
        Self {
            should_speak_when: None,
            should_direct_speech_when: None,
            room_min_audience: default_speech_room_min_audience(),
        }
    }
}

fn default_speech_room_min_audience() -> usize {
    2
}