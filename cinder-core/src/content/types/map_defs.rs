use super::Condition;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapDefinition {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub floor: Option<String>,
    #[serde(default)]
    pub rooms: Vec<MapRoomDefinition>,
    /// Whether rooms stay hidden until the player has been there. Off for a place
    /// the player already knows, such as a house.
    #[serde(default = "fog_by_default")]
    pub fog: bool,
    /// Conditions that reveal the whole map at once, such as a boss defeated or
    /// a town unlocked.
    #[serde(default)]
    pub reveal_when: Vec<Condition>,
}

fn fog_by_default() -> bool {
    true
}

impl Default for MapDefinition {
    fn default() -> Self {
        Self {
            id: String::new(),
            label: String::new(),
            floor: None,
            rooms: Vec::new(),
            fog: true,
            reveal_when: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapRoomDefinition {
    pub room_id: String,
    pub x: f64,
    pub y: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_stairs: Option<bool>,
}
