use serde::{Deserialize, Serialize};

/// Pack-declared teleport network. The core engine implements the generic
/// teleport mechanic (resolution, movement, chalk anchors); which destinations
/// are *permanent* platforms vs *temporary* chalk anchors is content-defined.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TeleportNetworkDefinition {
    /// Maximum number of temporary chalk anchors a player may hold at once.
    /// `None` uses the engine default. Set to `0` to disable chalk anchors.
    #[serde(default)]
    pub max_chalk_anchors: Option<usize>,
    /// Platforms that stay active once armed; resolved before chalk anchors.
    #[serde(default)]
    pub permanent_anchors: Vec<TeleportAnchorDefinition>,
}

impl TeleportNetworkDefinition {
    pub fn chalk_capacity(&self) -> usize {
        self.max_chalk_anchors.unwrap_or(3)
    }
}

/// A permanent teleport platform. It is only available while `armed_by` is a
/// truthy story variable (set by pack content, e.g. a `player.moved` hook).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeleportAnchorDefinition {
    /// Destination room id.
    pub room_id: String,
    /// Story var that must be truthy for the anchor to be available.
    pub armed_by: String,
    /// Title shown in the teleport panel.
    #[serde(default)]
    pub title: String,
    /// Extra names a player may use to target this anchor.
    #[serde(default)]
    pub aliases: Vec<String>,
}
