use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutonomousHostilityMode {
    #[default]
    Rules,
    Llm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum XpRecipientMode {
    PlayerOnly,
    #[default]
    PlayerAndFollowers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum XpDistributionMode {
    #[default]
    FullEach,
    SplitEvenly,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct XpDistributionDefinition {
    #[serde(default)]
    pub recipients: XpRecipientMode,
    #[serde(default)]
    pub mode: XpDistributionMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AllyAttackParticipants {
    Disabled,
    FollowersOnly,
    #[default]
    AllAlliesInRoom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AllyAttackMode {
    #[default]
    AttackStat,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllyAttackDefinition {
    #[serde(default)]
    pub participants: AllyAttackParticipants,
    #[serde(default)]
    pub mode: AllyAttackMode,
    #[serde(default = "default_contribution_percent")]
    pub contribution_percent: u32,
    #[serde(default)]
    pub maximum_per_ally: Option<i32>,
}

impl Default for AllyAttackDefinition {
    fn default() -> Self {
        Self {
            participants: AllyAttackParticipants::default(),
            mode: AllyAttackMode::default(),
            contribution_percent: default_contribution_percent(),
            maximum_per_ally: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CombatSettingsDefinition {
    #[serde(default = "default_player_actor_id")]
    pub player_actor_id: String,
    #[serde(default = "default_health_stat_id")]
    pub health_stat_id: String,
    #[serde(default = "default_attack_stat_id")]
    pub attack_stat_id: String,
    #[serde(default = "default_defense_stat_id")]
    pub defense_stat_id: String,
    #[serde(default = "default_minimum_damage")]
    pub minimum_damage: i32,
    #[serde(default = "default_attack_interval_minutes")]
    pub default_attack_interval_minutes: u32,
    #[serde(default = "default_player_defeat_text")]
    pub player_defeat_text: String,
    #[serde(default)]
    pub drain_item_id: Option<String>,
    #[serde(default)]
    pub drain_damage_per_tick: i32,
    #[serde(default)]
    pub xp_distribution: XpDistributionDefinition,
    #[serde(default)]
    pub ally_attack: AllyAttackDefinition,
}

impl Default for CombatSettingsDefinition {
    fn default() -> Self {
        Self {
            player_actor_id: default_player_actor_id(),
            health_stat_id: default_health_stat_id(),
            attack_stat_id: default_attack_stat_id(),
            defense_stat_id: default_defense_stat_id(),
            minimum_damage: default_minimum_damage(),
            default_attack_interval_minutes: default_attack_interval_minutes(),
            player_defeat_text: default_player_defeat_text(),
            drain_item_id: None,
            drain_damage_per_tick: 0,
            xp_distribution: XpDistributionDefinition::default(),
            ally_attack: AllyAttackDefinition::default(),
        }
    }
}

fn default_player_actor_id() -> String {
    "player".to_string()
}

fn default_health_stat_id() -> String {
    "hp".to_string()
}

fn default_attack_stat_id() -> String {
    "strength".to_string()
}

fn default_defense_stat_id() -> String {
    "defense".to_string()
}

fn default_minimum_damage() -> i32 {
    1
}

pub(super) fn default_attack_interval_minutes() -> u32 {
    4
}

fn default_contribution_percent() -> u32 {
    100
}

fn default_player_defeat_text() -> String {
    "The world tilts. Your legs give out. The last thing you feel is the cold stone beneath your palms."
        .to_string()
}
