use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SpeechIntentEffect {
    ActorStat { stat: String, delta: i32 },
    PairStat { stat: String, delta: i32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechIntentLabel {
    pub label: String,
    pub description: String,
    #[serde(default)]
    pub effects: Vec<SpeechIntentEffect>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpeechIntentsConfig {
    #[serde(default)]
    pub intents: Vec<SpeechIntentLabel>,
}
