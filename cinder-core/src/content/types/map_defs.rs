use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MapDefinition {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub rooms: Vec<MapRoomDefinition>,
    #[serde(default)]
    pub reveal_conditions: Vec<MapRevealCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapRoomDefinition {
    pub room_id: String,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MapRevealCondition {
    Always,
    ActorDefeated { actor_id: String },
    StoryVarTruthy { key: String },
}
