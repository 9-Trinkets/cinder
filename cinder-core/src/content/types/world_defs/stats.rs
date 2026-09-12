use super::super::default_stat_default_value;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatDefinition {
    #[serde(default = "default_stat_default_value")]
    pub default: i32,
    #[serde(default)]
    pub min: Option<i32>,
    #[serde(default)]
    pub max: Option<i32>,
    #[serde(default)]
    pub time_step_minutes: Option<u32>,
}

impl StatDefinition {
    pub fn clamp(&self, value: i32) -> i32 {
        let lower = self.min.unwrap_or(i32::MIN);
        let upper = self.max.unwrap_or(i32::MAX);
        value.clamp(lower, upper)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatsDefinition {
    #[serde(default)]
    pub actor: BTreeMap<String, StatDefinition>,
    #[serde(default)]
    pub pair: BTreeMap<String, StatDefinition>,
}