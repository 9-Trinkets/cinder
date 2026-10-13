use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use super::{
    ActorTickScope, AutonomousHostilityMode, CombatSettingsDefinition, MessagingChannel,
    PartyCombatDecisionRule, PartyController, PartyDefinition, PeriodicActorEffectDefinition,
    ThemeDefinition,
};

/// How the "actor surrounded" conversion hook decides whether an encircled
/// non-player actor actually converts. The rule is a pure numeric gate; which
/// text narrates a refused conversion is the pack's `surround.refused` message.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum SurroundRule {
    /// Every candidate the pack's `actor.surrounded` hook names converts. The
    /// pack is fully responsible for gating (e.g. via hook conditions).
    #[default]
    None,
    /// A candidate converts only when the player out-scores it against the
    /// named stat: `player_stat + player_level >= target_stat + 2*target_level`.
    /// Player side uses the effective value, so equipped bonuses count.
    Resistance {
        /// Actor stat the gate compares (e.g. the pack's mental stat).
        #[serde(default = "default_resistance_stat")]
        stat: String,
    },
}

fn default_resistance_stat() -> String {
    "intelligence".to_string()
}

impl<'de> Deserialize<'de> for SurroundRule {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            Plain(String),
            Spec(Spec),
        }
        #[derive(Deserialize)]
        struct Spec {
            mode: String,
            #[serde(default = "default_resistance_stat")]
            stat: String,
        }
        match Repr::deserialize(deserializer)? {
            Repr::Plain(plain) => match plain.as_str() {
                "none" => Ok(SurroundRule::None),
                "int_and_level" => Ok(SurroundRule::Resistance {
                    stat: default_resistance_stat(),
                }),
                other => Err(serde::de::Error::custom(format!(
                    "unknown surround rule '{other}'"
                ))),
            },
            Repr::Spec(spec) => match spec.mode.as_str() {
                "none" => Ok(SurroundRule::None),
                "resistance" => Ok(SurroundRule::Resistance { stat: spec.stat }),
                other => Err(serde::de::Error::custom(format!(
                    "unknown surround rule mode '{other}'"
                ))),
            },
        }
    }
}

/// One sidebar section a pack can show. Presence in
/// [`SidebarDefinition::sections`] means the section may appear at all;
/// its position in the list is the display order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SidebarSectionDefinition {
    /// Section id the web UI's renderer registry keys on (e.g. `"vitals"`).
    /// Unknown ids are skipped by the client.
    pub id: String,
    /// Optional story var that gates the section: it is shown only once this
    /// var is truthy. Absent (or empty) means always visible.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub story_var: Option<String>,
}

/// Ordered declarative list of the sidebar sections a pack shows. The engine
/// resolves it to the visible ids in [`SidebarDefinition::sections`] order,
/// honoring each section's `story_var` gate.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SidebarDefinition {
    #[serde(default)]
    pub sections: Vec<SidebarSectionDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentSettingsDefinition {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub tagline: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "default_npc_tick_interval_ms")]
    pub npc_tick_interval_ms: u64,
    #[serde(default = "default_tick_minutes_per_turn")]
    pub tick_minutes_per_turn: u32,
    #[serde(default = "default_default_language")]
    pub default_language: String,
    #[serde(default)]
    pub autonomous_actor_dialogue: bool,
    /// Whether the web UI shows the player's free-text input box. Spectator
    /// packs (e.g. Aera's control room) set this to false so the player
    /// watches the show instead of typing commands.
    #[serde(default = "default_true")]
    pub show_player_input: bool,
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
    /// Declarative sidebar section list. Presence means a section may appear;
    /// order is display order; a section's optional `story_var` gates it (see
    /// [`SidebarSectionDefinition`]). Omitting a section hides it entirely.
    #[serde(default)]
    pub sidebar: SidebarDefinition,
    /// Skill definitions (or legacy behavior rules in non-strict packs)
    /// define eligible hostile actions. `rules` applies them directly; `llm`
    /// asks a validated planner to choose a subset.
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
    /// Id of the *direct* channel used when a pack message explicitly selects
    /// the `handler` voice. Automated feedback keeps its own error, system, or
    /// narration delivery unless the individual message opts into commentary.
    #[serde(default)]
    pub feedback_channel_id: String,
    /// Binds the generic strike mechanism to this pack's stat vocabulary.
    #[serde(default)]
    pub combat: CombatSettingsDefinition,
    /// Faction-agnostic parties: the player's command group plus any
    /// AI-controlled groups. At most one party may use the player controller.
    #[serde(default)]
    pub parties: Vec<PartyDefinition>,
    /// Whether encircling an actor converts it at all, and under what numeric
    /// gate (see [`SurroundRule`]). Refused conversions narrate the pack's
    /// `surround.refused` message.
    #[serde(default)]
    pub surround_rule: SurroundRule,
    /// Fixed equipment slot list (e.g. "weapon", "armor", "trinket"). Each
    /// slot holds at most one equipped item; equippable items name one of
    /// these slots and their bonuses feed effective stat reads.
    #[serde(default)]
    pub equipment_slots: BTreeSet<String>,
    #[serde(default)]
    pub theme: ThemeDefinition,
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
            tags: Vec::new(),
            npc_tick_interval_ms: default_npc_tick_interval_ms(),
            tick_minutes_per_turn: default_tick_minutes_per_turn(),
            default_language: default_default_language(),
            autonomous_actor_dialogue: false,
            show_player_input: true,
            actor_tick_scope: ActorTickScope::default(),
            closure_perspective_actor_id: String::default(),
            act_member_alias: String::default(),
            fallback_stage_id: String::default(),
            fallback_required_story_vars: Vec::new(),
            workflow_id: String::default(),
            show_act_closure: true,
            sidebar: SidebarDefinition::default(),
            autonomous_hostility_mode: AutonomousHostilityMode::Rules,
            periodic_actor_effects: Vec::new(),
            channels: Vec::new(),
            feedback_channel_id: String::default(),
            combat: CombatSettingsDefinition::default(),
            parties: Vec::new(),
            surround_rule: SurroundRule::None,
            equipment_slots: BTreeSet::new(),
            theme: ThemeDefinition::default(),
        }
    }
}

impl ContentSettingsDefinition {
    /// The party the player controls, if authored.
    pub fn player_party(&self) -> Option<&PartyDefinition> {
        self.parties
            .iter()
            .find(|party| party.controller == PartyController::Player)
    }

    /// Ordered combat rules authored on the player party (empty if none).
    pub fn player_party_combat_rules(&self) -> &[PartyCombatDecisionRule] {
        self.player_party()
            .map_or(&[], |party| party.combat_rules.as_slice())
    }

    /// The party the player controls, creating a bare `"player"` party if the
    /// pack authored none. Loader-authored party reactions attach here.
    pub fn ensure_player_party(&mut self) -> &mut PartyDefinition {
        if self
            .parties
            .iter()
            .all(|party| party.controller != PartyController::Player)
        {
            self.parties.push(PartyDefinition {
                id: "player".to_string(),
                controller: PartyController::Player,
                ..PartyDefinition::default()
            });
        }
        self.parties
            .iter_mut()
            .find(|party| party.controller == PartyController::Player)
            .expect("player party exists after insertion")
    }
}
