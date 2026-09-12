use std::collections::BTreeMap;

use super::*;
use crate::content::types::{ContentPack, ItemDefinition};

impl WorldState {
    /// Current level for an actor; absent entries read as level 1.
    pub fn actor_level(&self, actor_id: &str) -> u32 {
        self.actor_level.get(actor_id).copied().unwrap_or(1).max(1)
    }

    /// Experience an actor holds toward their next level.
    pub fn actor_xp(&self, actor_id: &str) -> u32 {
        self.actor_xp.get(actor_id).copied().unwrap_or(0)
    }

    /// True only when an explicit entry for the pack's health stat records the
    /// actor at or below zero. Actors without such an entry (never touched by
    /// combat) are not considered defeated, so packs without combat are
    /// unaffected.
    pub fn actor_is_defeated(&self, actor_id: &str, health_stat_id: &str) -> bool {
        let actor_id = remap_story_actor_id(self, actor_id);
        self.actor_stats
            .get(actor_id)
            .and_then(|stats| stats.get(health_stat_id))
            .is_some_and(|hp| *hp <= 0)
    }

    pub fn pair_stat(
        &self,
        first_participant_id: &str,
        second_participant_id: &str,
        stat_key: &str,
    ) -> i32 {
        self.pair_stats
            .get(&Self::conversation_key(
                first_participant_id,
                second_participant_id,
            ))
            .and_then(|stats| stats.get(stat_key))
            .copied()
            .unwrap_or_else(|| {
                self.pair_stat_defs
                    .get(stat_key)
                    .map(|stat| stat.default)
                    .unwrap_or(0)
            })
    }

    pub fn actor_stat(&self, actor_id: &str, stat_key: &str) -> i32 {
        let actor_id = remap_story_actor_id(self, actor_id);
        self.actor_stats
            .get(actor_id)
            .and_then(|stats| stats.get(stat_key))
            .copied()
            .unwrap_or_else(|| {
                self.actor_stat_defs
                    .get(stat_key)
                    .map(|stat| stat.default)
                    .unwrap_or(0)
            })
    }

    pub fn actor_stat_u32(&self, actor_id: &str, stat_key: &str) -> u32 {
        self.actor_stat(actor_id, stat_key).max(0) as u32
    }

    /// Sum of stat bonuses granted by the player's equipped items. Equipment
    /// is player-scoped; other actors always get zero. A multi-slot item (e.g.
    /// a two-hand weapon) pays its bonus once no matter how many slots it
    /// occupies.
    pub fn equipped_stat_bonus(&self, content: &ContentPack, stat_key: &str) -> i32 {
        let mut seen = std::collections::BTreeSet::new();
        self.equipment
            .values()
            .filter(|item_id| seen.insert(item_id.as_str()))
            .filter_map(|item_id| content.item(item_id))
            .filter_map(|item| item.stat_bonuses.get(stat_key))
            .sum()
    }

    pub fn actor_stats_snapshot(&self, actor_id: &str) -> BTreeMap<String, i32> {
        let actor_id = remap_story_actor_id(self, actor_id);
        self.actor_stat_defs
            .keys()
            .map(|stat_key| (stat_key.clone(), self.actor_stat(actor_id, stat_key)))
            .collect()
    }

    /// Stat value as gameplay should see it: stored base plus the player's
    /// equipped bonuses. Gameplay reads (combat rolls, strike damage, defeat
    /// checks) must use this instead of `actor_stat`. The write path still
    /// clamps the base to the stat's declared min/max; equipment can push the
    /// effective value past those bounds by design.
    /// Full (undamaged) value for a stat: the actor's seeded value plus the
/// growth granted by every level already reached. Damage only drives the
/// stored stat below this, so it is the natural "max" to display alongside
/// the current value.
pub fn actor_stat_maximum(&self, content: &ContentPack, actor_id: &str, stat_key: &str) -> i32 {
    let definition = self.actor_stat_defs.get(stat_key).cloned();
    let initial = self
        .initial_actor_stats
        .get(actor_id)
        .and_then(|stats| stats.get(stat_key))
        .copied()
        .unwrap_or_else(|| definition.as_ref().map(|stat| stat.default).unwrap_or(0));
    let level = self.actor_level.get(actor_id).copied().unwrap_or(1).max(1);
    let growth = (1..level)
        .filter_map(|prior_level| content.level_definition(actor_id, prior_level))
        .filter_map(|definition| definition.stat_changes.get(stat_key))
        .copied()
        .sum::<i32>();
    let full = initial.saturating_add(growth);
    definition
        .map(|definition| definition.clamp(full))
        .unwrap_or(full)
}

/// Full stat value the player can display: the natural maximum plus the
/// equipped bonuses the current value itself also carries.
pub fn effective_actor_stat_maximum(
    &self,
    content: &ContentPack,
    actor_id: &str,
    stat_key: &str,
) -> i32 {
    let mut value = self.actor_stat_maximum(content, actor_id, stat_key);
    if actor_id == content.settings.combat.player_actor_id {
        value += self.equipped_stat_bonus(content, stat_key);
    }
    value
}

pub fn effective_actor_stat(
        &self,
        content: &ContentPack,
        actor_id: &str,
        stat_key: &str,
    ) -> i32 {
        let mut value = self.actor_stat(actor_id, stat_key);
        if actor_id == content.settings.combat.player_actor_id {
            value += self.equipped_stat_bonus(content, stat_key);
        }
        value
    }

    /// Item id equipped in `slot_id`, if any.
    pub fn equipped_item(&self, slot_id: &str) -> Option<&str> {
        self.equipment.get(slot_id).map(String::as_str)
    }

    /// Whether `item` currently occupies every slot it requires (i.e. it is
    /// fully equipped). Multi-slot items are only "equipped" when all their
    /// slots hold this exact item.
    pub fn item_is_equipped(&self, item: &ItemDefinition) -> bool {
        let slots = item.occupied_slots();
        !slots.is_empty()
            && slots
                .iter()
                .all(|slot| self.equipment.get(slot).map(String::as_str) == Some(item.id.as_str()))
    }

    pub fn pair_stats_snapshot(
        &self,
        first_participant_id: &str,
        second_participant_id: &str,
    ) -> BTreeMap<String, i32> {
        self.pair_stat_defs
            .keys()
            .map(|stat_key| {
                (
                    stat_key.clone(),
                    self.pair_stat(first_participant_id, second_participant_id, stat_key),
                )
            })
            .collect()
    }

    pub fn actor_stat_deltas(&self, actor_id: &str) -> Option<BTreeMap<String, i32>> {
        let actor_id = remap_story_actor_id(self, actor_id);
        let initial = self.initial_actor_stats.get(actor_id)?;
        Some(
            initial
                .keys()
                .map(|stat_key| {
                    let current = self.actor_stat(actor_id, stat_key);
                    let init = initial.get(stat_key).copied().unwrap_or(0);
                    (stat_key.clone(), current - init)
                })
                .collect(),
        )
    }

    pub fn pair_stat_deltas(
        &self,
        first_id: &str,
        second_id: &str,
    ) -> Option<BTreeMap<String, i32>> {
        let key = Self::conversation_key(first_id, second_id);
        let initial = self.initial_pair_stats.get(&key)?;
        Some(
            initial
                .keys()
                .map(|stat_key| {
                    let current = self.pair_stat(first_id, second_id, stat_key);
                    let init = initial.get(stat_key).copied().unwrap_or(0);
                    (stat_key.clone(), current - init)
                })
                .collect(),
        )
    }

    pub fn pair_stat_u32(
        &self,
        first_participant_id: &str,
        second_participant_id: &str,
        stat_key: &str,
    ) -> u32 {
        self.pair_stat(first_participant_id, second_participant_id, stat_key)
            .max(0) as u32
    }

    pub fn adjust_pair_stat(
        &mut self,
        first_participant_id: &str,
        second_participant_id: &str,
        stat_key: &str,
        delta: i32,
    ) -> Result<(), String> {
        let definition = self
            .pair_stat_defs
            .get(stat_key)
            .ok_or_else(|| format!("unknown pair stat '{stat_key}'"))?
            .clone();
        let stats = self
            .pair_stats
            .entry(Self::conversation_key(
                first_participant_id,
                second_participant_id,
            ))
            .or_default();
        let value = stats
            .entry(stat_key.to_string())
            .or_insert(definition.default);
        *value = definition.clamp(*value + delta);
        Ok(())
    }

    pub fn adjust_actor_stat(
        &mut self,
        actor_id: &str,
        stat_key: &str,
        delta: i32,
    ) -> Result<(), String> {
        let actor_id = remap_story_actor_id(self, actor_id).to_string();
        let definition = self
            .actor_stat_defs
            .get(stat_key)
            .ok_or_else(|| format!("unknown actor stat '{stat_key}'"))?
            .clone();
        let stats = self.actor_stats.entry(actor_id).or_default();
        let value = stats
            .entry(stat_key.to_string())
            .or_insert(definition.default);
        *value = definition.clamp(*value + delta);
        Ok(())
    }
}
