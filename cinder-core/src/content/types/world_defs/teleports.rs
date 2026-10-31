use super::super::Condition;
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

/// A permanent teleport platform. It is only available while its `when`
/// conditions hold (set by pack content, e.g. a `player.moved` hook).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeleportAnchorDefinition {
    /// Destination room id.
    pub room_id: String,
    /// Conditions that must hold for the anchor to be available.
    pub when: Vec<Condition>,
    /// Title shown in the teleport panel.
    #[serde(default)]
    pub title: String,
    /// Extra names a player may use to target this anchor.
    #[serde(default)]
    pub aliases: Vec<String>,
}
