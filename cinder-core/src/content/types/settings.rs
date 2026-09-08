use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::{BTreeSet};

use super::{
    ActorTickScope, AutonomousHostilityMode, CombatSettingsDefinition, MessagingChannel,
    PeriodicActorEffectDefinition, ThemeDefinition,
};

/// How the "actor surrounded" conversion hook decides whether an encircled
/// non-player actor actually converts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CharmRule {
    /// Every candidate the pack's `actor.surrounded` hook names converts. The
    /// pack is fully responsible for gating (e.g. via hook conditions).
    #[default]
    None,
    /// A candidate converts only when the player out-scores it:
    /// `player_int + player_level >= target_int + 2*target_level`, where int is
    /// the `intelligence` actor stat (player side uses the effective value so
    /// equipped bonuses count). Failures narrate a cold system line instead of
    /// converting. Used by layla so bosses stay un-charmable by numbers, not by
    /// special-cased hook conditions.
    IntAndLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentSettingsDefinition {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub tagline: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_typewriter_char_ms")]
    pub typewriter_char_ms: u64,
    #[serde(default = "default_npc_tick_interval_ms")]
    pub npc_tick_interval_ms: u64,
    #[serde(default = "default_tick_minutes_per_turn")]
    pub tick_minutes_per_turn: u32,
    #[serde(default = "default_default_language")]
    pub default_language: String,
    #[serde(default)]
    pub channel_surfing_only: bool,
    #[serde(default)]
    pub autonomous_actor_dialogue: bool,
    /// Limits which actors participate in background ticks.
    #[serde(default)]
    pub actor_tick_scope: ActorTickScope,
    #[serde(default)]
    pub workflow_id: String,
    #[serde(default)]
    pub closure_perspective_actor_id: String,
    #[serde(default)]
    pub act_member_alias: String,
    #[serde(default)]
    pub fallback_stage_id: String,
    #[serde(default)]
    pub fallback_required_story_vars: Vec<String>,
    #[serde(default = "default_true")]
    pub show_act_closure: bool,
    #[serde(default)]
    pub show_relationship_sidebar: bool,
    /// Whether the sidebar shows the player's Vitals (HP + stats) and Level
    /// sections. Combat packs (e.g. layla) enable these; story packs without a
    /// combat system disable them to keep the sidebar focused.
    #[serde(default = "default_true")]
    pub show_vitals_sidebar: bool,
    /// Story var that additionally gates the Vitals + Level sidebar sections.
    /// Empty keeps the static `show_vitals_sidebar` behavior; non-empty shows
    /// them only once the var is truthy (e.g. layered teaching: stats appear
    /// only after `first_mob_defeated`).
    #[serde(default)]
    pub vitals_sidebar_story_var: String,
    /// Story var that gates the whole minimap widget. Empty keeps the minimap
    /// always rendered; non-empty hides it until the var is truthy (e.g. the
    /// map is a boss-drop reward, revealed once `shaman_defeated`).
    #[serde(default)]
    pub minimap_requires_story_var: String,
    /// Room-id prefix that, once the player has travelled to a room on that
    /// board, reveals party levels in the sidebar. Empty means levels are
    /// always visible. Treats level info as a reward and foreshadows a
    /// hierarchy (e.g. a chess board) when the player descends.
    #[serde(default)]
    pub level_reveal_room_prefix: String,
    /// `behavior.json` defines eligible hostile strikes. `rules` applies them
    /// directly; `llm` asks a validated planner to choose a subset.
    #[serde(default)]
    pub autonomous_hostility_mode: AutonomousHostilityMode,
    /// Content-authored actor effects evaluated after each background tick.
    #[serde(default)]
    pub periodic_actor_effects: Vec<PeriodicActorEffectDefinition>,
    /// Actor messaging channels the pack declares (e.g. the handler's private
    /// remote comms). Same-room speech needs no declaration; the implicit
    /// `local` channel is always available.
    #[serde(default)]
    pub channels: Vec<MessagingChannel>,
    /// Binds the generic strike mechanism to this pack's stat vocabulary.
    #[serde(default)]
    pub combat: CombatSettingsDefinition,
    /// Whether encircling an actor converts it at all, and under what rule
    /// (see [`CharmRule`]).
    #[serde(default)]
    pub charm_rule: CharmRule,
    /// Items the player starts with, item id → count. Seed a finite resource
    /// (e.g. layla's stone markers) here so it can be dropped into rooms and
    /// picked back up.
    #[serde(default)]
    pub starting_items: BTreeMap<String, u32>,
    /// Fixed equipment slot list (e.g. "weapon", "armor", "trinket"). Each
    /// slot holds at most one equipped item; equippable items name one of
    /// these slots and their bonuses feed effective stat reads.
    #[serde(default)]
    pub equipment_slots: BTreeSet<String>,
    #[serde(default)]
    pub theme: ThemeDefinition,
}

fn default_typewriter_char_ms() -> u64 {
    40
}

fn default_npc_tick_interval_ms() -> u64 {
    2_000
}

fn default_tick_minutes_per_turn() -> u32 {
    1
}

pub(crate) fn default_stat_default_value() -> i32 {
    0
}

pub(crate) fn default_actor_targeted_speech() -> String {
    "{actor_name} (to {target_name}): {text}".to_string()
}

fn default_default_language() -> String {
    "en".to_string()
}

pub(super) fn default_true() -> bool {
    true
}

impl Default for ContentSettingsDefinition {
    fn default() -> Self {
        Self {
            title: String::default(),
            tagline: String::default(),
            description: String::default(),
            typewriter_char_ms: default_typewriter_char_ms(),
            npc_tick_interval_ms: default_npc_tick_interval_ms(),
            tick_minutes_per_turn: default_tick_minutes_per_turn(),
            default_language: default_default_language(),
            channel_surfing_only: false,
            autonomous_actor_dialogue: false,
            actor_tick_scope: ActorTickScope::default(),
            closure_perspective_actor_id: String::default(),
            act_member_alias: String::default(),
            fallback_stage_id: String::default(),
            fallback_required_story_vars: Vec::new(),
            workflow_id: String::default(),
            show_act_closure: true,
            show_relationship_sidebar: false,
            show_vitals_sidebar: true,
            vitals_sidebar_story_var: String::default(),
            minimap_requires_story_var: String::default(),
            level_reveal_room_prefix: String::default(),
            autonomous_hostility_mode: AutonomousHostilityMode::Rules,
            periodic_actor_effects: Vec::new(),
            channels: Vec::new(),
            combat: CombatSettingsDefinition::default(),
            charm_rule: CharmRule::None,
            starting_items: BTreeMap::new(),
            equipment_slots: BTreeSet::new(),
            theme: ThemeDefinition::default(),
        }
    }
}
