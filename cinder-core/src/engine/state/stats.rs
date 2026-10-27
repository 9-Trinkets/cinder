use std::collections::{BTreeMap, BTreeSet};

use super::*;
use crate::content::types::{ContentPack, ItemDefinition, StatDefinition};
use serde::{Deserialize, Serialize};

/// Combat and progression state: stat definitions and current/initial values
/// for actors and pairs, experience, levels, acquired skills, and
/// hostile-strike scheduling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatState {
    pub actor_stat_defs: BTreeMap<String, StatDefinition>,
    pub pair_stat_defs: BTreeMap<String, StatDefinition>,
    pub pair_stats: BTreeMap<String, BTreeMap<String, i32>>,
    pub actor_stats: BTreeMap<String, BTreeMap<String, i32>>,
    #[serde(default)]
    pub initial_actor_stats: BTreeMap<String, BTreeMap<String, i32>>,
    #[serde(default)]
    pub initial_pair_stats: BTreeMap<String, BTreeMap<String, i32>>,
    /// Per-actor experience toward their next level. Absent entries read as 0.
    #[serde(default)]
    pub actor_xp: BTreeMap<String, u32>,
    /// Per-actor current level. Absent entries read as 1.
    #[serde(default)]
    pub actor_level: BTreeMap<String, u32>,
    /// Acquired skills per actor.
    #[serde(default)]
    pub actor_skills: BTreeMap<String, BTreeSet<String>>,
    /// Per-actor skill levels (actor_id -> (skill_id -> level 1..=10)).
    #[serde(default)]
    pub actor_skill_levels: BTreeMap<String, BTreeMap<String, u32>>,
    /// Per-actor skill proficiency points (actor_id -> (skill_id -> total SP)).
    #[serde(default)]
    pub actor_skill_sp: BTreeMap<String, BTreeMap<String, u32>>,
    /// Next game-minute at which each hostile actor may autonomously strike.
    /// Absent entries have no pending autonomous strike.
    #[serde(default)]
    pub next_hostile_strike_at: BTreeMap<String, u32>,
}

impl WorldState {
    /// Whether `actor_id` is the player-controlled actor.
    pub fn is_player_actor(&self, actor_id: &str) -> bool {
        actor_id == self.player_actor_id
    }

    /// Current level for an actor; absent entries read as level 1.
    pub fn actor_level(&self, actor_id: &str) -> u32 {
        self.combat
            .actor_level
            .get(actor_id)
            .copied()
            .unwrap_or(1)
            .max(1)
    }

    /// Current level for an actor's skill; absent entries read as level 1.
    pub fn actor_skill_level(&self, actor_id: &str, skill_id: &str) -> u32 {
        self.combat
            .actor_skill_levels
            .get(actor_id)
            .and_then(|skills| skills.get(skill_id))
            .copied()
            .unwrap_or(1)
            .clamp(1, 10)
    }

    /// Current skill proficiency (SP) for an actor's skill.
    pub fn actor_skill_sp(&self, actor_id: &str, skill_id: &str) -> u32 {
        self.combat
            .actor_skill_sp
            .get(actor_id)
            .and_then(|skills| skills.get(skill_id))
            .copied()
            .unwrap_or(0)
    }

    /// Sets the character level for an actor (1..=10).
    pub fn set_actor_level(&mut self, actor_id: &str, level: u32) {
        self.combat
            .actor_level
            .insert(actor_id.to_string(), level.clamp(1, 10));
    }

    /// Sets the skill level for an actor (1..=10).
    pub fn set_actor_skill_level(&mut self, actor_id: &str, skill_id: &str, level: u32) {
        let clamped = level.clamp(1, 10);
        self.combat
            .actor_skill_levels
            .entry(actor_id.to_string())
            .or_default()
            .insert(skill_id.to_string(), clamped);
        let sp = crate::engine::skills::skill_sp_for_level(clamped);
        self.combat
            .actor_skill_sp
            .entry(actor_id.to_string())
            .or_default()
            .insert(skill_id.to_string(), sp);
    }

    /// Awards SP to an actor's skill (1..=10).
    /// Returns true if the skill level increased.
    pub fn award_skill_sp(&mut self, actor_id: &str, skill_id: &str, sp_delta: u32) -> bool {
        let current_sp = self.actor_skill_sp(actor_id, skill_id);
        let new_sp = current_sp.saturating_add(sp_delta);
        self.combat
            .actor_skill_sp
            .entry(actor_id.to_string())
            .or_default()
            .insert(skill_id.to_string(), new_sp);

        let old_level = self.actor_skill_level(actor_id, skill_id);
        let new_level = crate::engine::skills::skill_level_from_sp(new_sp).clamp(1, 10);
        if new_level > old_level {
            self.combat
                .actor_skill_levels
                .entry(actor_id.to_string())
                .or_default()
                .insert(skill_id.to_string(), new_level);
            true
        } else {
            false
        }
    }

    /// Re-evaluates all skills against SP.
    pub fn recheck_actor_skill_levels(&mut self, actor_id: &str) {
        if let Some(skill_sps) = self.combat.actor_skill_sp.get(actor_id).cloned() {
            for (skill_id, sp) in skill_sps {
                let target_level = crate::engine::skills::skill_level_from_sp(sp).clamp(1, 10);
                let current_level = self.actor_skill_level(actor_id, &skill_id);
                if target_level > current_level {
                    self.set_actor_skill_level(actor_id, &skill_id, target_level);
                }
            }
        }
    }

    /// Experience an actor holds toward their next level.
    pub fn actor_xp(&self, actor_id: &str) -> u32 {
        self.combat.actor_xp.get(actor_id).copied().unwrap_or(0)
    }

    /// True only when an explicit entry for the pack's health stat records the
    /// actor at or below zero. Actors without such an entry (never touched by
    /// combat) are not considered defeated, so packs without combat are
    /// unaffected.
    pub fn actor_is_defeated(&self, actor_id: &str, health_stat_id: &str) -> bool {
        self.combat
            .actor_stats
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
        self.combat
            .pair_stats
            .get(&Self::conversation_key(
                first_participant_id,
                second_participant_id,
            ))
            .and_then(|stats| stats.get(stat_key))
            .copied()
            .unwrap_or_else(|| {
                self.combat
                    .pair_stat_defs
                    .get(stat_key)
                    .map(|stat| stat.default)
                    .unwrap_or(0)
            })
    }

    pub fn actor_stat(&self, actor_id: &str, stat_key: &str) -> i32 {
        self.combat
            .actor_stats
            .get(actor_id)
            .and_then(|stats| stats.get(stat_key))
            .copied()
            .unwrap_or_else(|| {
                self.combat
                    .actor_stat_defs
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
    /// Sum of stat bonuses granted by items equipped across any slots. If an
    /// item occupies multiple slots (e.g. both `weapon` and `off-hand` for
    /// a two-hand weapon) pays its bonus once no matter how many slots it
    /// occupies.
    pub fn equipped_stat_bonus(&self, content: &ContentPack, stat_key: &str) -> i32 {
        self.actor_equipped_stat_bonus(content, &content.settings.combat.player_actor_id, stat_key)
    }

    /// Sum of stat bonuses granted by items equipped by a specific actor.
    pub fn actor_equipped_stat_bonus(
        &self,
        content: &ContentPack,
        actor_id: &str,
        stat_key: &str,
    ) -> i32 {
        let eq = if actor_id == content.settings.combat.player_actor_id {
            &self.inventory.equipment
        } else {
            self.actor_equipment(actor_id)
        };
        let mut seen = std::collections::BTreeSet::new();
        eq.values()
            .filter(|item_id| seen.insert(item_id.as_str()))
            .filter_map(|item_id| content.item(item_id))
            .filter_map(|item| item.stat_bonuses.get(stat_key))
            .sum()
    }

    pub fn actor_stats_snapshot(&self, actor_id: &str) -> BTreeMap<String, i32> {
        self.combat
            .actor_stat_defs
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
        let definition = self.combat.actor_stat_defs.get(stat_key).cloned();
        let initial = self
            .combat
            .initial_actor_stats
            .get(actor_id)
            .and_then(|stats| stats.get(stat_key))
            .copied()
            .unwrap_or_else(|| definition.as_ref().map(|stat| stat.default).unwrap_or(0));
        let level = self
            .combat
            .actor_level
            .get(actor_id)
            .copied()
            .unwrap_or(1)
            .max(1);
        let initial_level = content
            .actor(actor_id)
            .map(|actor| actor.level.max(1))
            .unwrap_or(1);
        let growth = (initial_level..level)
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
        value += self.actor_equipped_stat_bonus(content, actor_id, stat_key);
        value
    }

    pub fn effective_actor_stat(
        &self,
        content: &ContentPack,
        actor_id: &str,
        stat_key: &str,
    ) -> i32 {
        let mut value = self.actor_stat(actor_id, stat_key);
        value += self.actor_equipped_stat_bonus(content, actor_id, stat_key);
        value
    }

    /// Effective combat attack rating: the actor's effective attack attribute
    /// (e.g. `strength`) plus any equipment bonuses to `"attack"`.
    pub fn actor_combat_attack(&self, content: &ContentPack, actor_id: &str) -> i32 {
        let base =
            self.effective_actor_stat(content, actor_id, &content.settings.combat.attack_stat_id);
        let weapon_bonus = if content.settings.combat.attack_stat_id != "attack" {
            self.actor_equipped_stat_bonus(content, actor_id, "attack")
        } else {
            0
        };
        let strike_bonus = if self.actor_has_skill(actor_id, "strike") {
            let lvl = self.actor_skill_level(actor_id, "strike");
            content
                .skill("strike")
                .and_then(|s| s.param_value("bonus_damage", lvl))
                .unwrap_or_else(|| crate::engine::skills::strike_bonus_damage(lvl) as i64)
                as i32
        } else {
            0
        };
        base + weapon_bonus + strike_bonus
    }

    /// Total count of active trace mark items across all room inventories.
    pub fn total_active_trace_marks(&self, content: &ContentPack) -> usize {
        self.inventory
            .room_item_stock
            .iter()
            .filter(|(key, count)| {
                if **count == 0 {
                    return false;
                }
                let item_id = key.split("::").nth(1).unwrap_or(key.as_str());
                content.item(item_id).is_some_and(|item| item.trace_mark)
            })
            .map(|(_, count)| *count as usize)
            .sum()
    }

    /// Effective combat defense rating: the actor's effective defense attribute
    /// (e.g. `endurance`) plus any equipment bonuses to `"defense"`.
    pub fn actor_combat_defense(&self, content: &ContentPack, actor_id: &str) -> i32 {
        let base =
            self.effective_actor_stat(content, actor_id, &content.settings.combat.defense_stat_id);
        let armor_bonus = if content.settings.combat.defense_stat_id != "defense" {
            self.actor_equipped_stat_bonus(content, actor_id, "defense")
        } else {
            0
        };
        base + armor_bonus
    }

    /// Item id equipped in `slot_id`, if any.
    pub fn equipped_item(&self, slot_id: &str) -> Option<&str> {
        self.inventory.equipment.get(slot_id).map(String::as_str)
    }

    /// Equipment for any actor (for non-player actors, reads `actor_equipment`).
    pub fn actor_equipment(&self, actor_id: &str) -> &BTreeMap<String, String> {
        static EMPTY: std::sync::LazyLock<BTreeMap<String, String>> =
            std::sync::LazyLock::new(BTreeMap::new);
        self.inventory
            .actor_equipment
            .get(actor_id)
            .unwrap_or(&EMPTY)
    }

    /// Item id equipped in `slot_id` for `actor_id`, if any.
    pub fn actor_equipped_item<'a>(&'a self, actor_id: &str, slot_id: &str) -> Option<&'a str> {
        if self.is_player_actor(actor_id) {
            self.equipped_item(slot_id)
        } else {
            self.actor_equipment(actor_id)
                .get(slot_id)
                .map(String::as_str)
        }
    }

    /// Whether `item` currently occupies every slot it requires (i.e. it is
    /// fully equipped). Multi-slot items are only "equipped" when all their
    /// slots hold this exact item.
    pub fn item_is_equipped(&self, item: &ItemDefinition) -> bool {
        let slots = item.occupied_slots();
        !slots.is_empty()
            && slots.iter().all(|slot| {
                self.inventory.equipment.get(slot).map(String::as_str) == Some(item.id.as_str())
            })
    }

    /// Whether `item` is equipped by `actor_id`.
    pub fn actor_item_is_equipped(
        &self,
        content: &ContentPack,
        actor_id: &str,
        item: &ItemDefinition,
    ) -> bool {
        if actor_id == content.settings.combat.player_actor_id {
            self.item_is_equipped(item)
        } else {
            let slots = item.occupied_slots();
            let eq = self.actor_equipment(actor_id);
            !slots.is_empty()
                && slots
                    .iter()
                    .all(|slot| eq.get(slot).map(String::as_str) == Some(item.id.as_str()))
        }
    }

    pub fn pair_stats_snapshot(
        &self,
        first_participant_id: &str,
        second_participant_id: &str,
    ) -> BTreeMap<String, i32> {
        self.combat
            .pair_stat_defs
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
        let initial = self.combat.initial_actor_stats.get(actor_id)?;
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
        let initial = self.combat.initial_pair_stats.get(&key)?;
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
            .combat
            .pair_stat_defs
            .get(stat_key)
            .ok_or_else(|| format!("unknown pair stat '{stat_key}'"))?
            .clone();
        let stats = self
            .combat
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
        content: &ContentPack,
        actor_id: &str,
        stat_key: &str,
        delta: i32,
    ) -> Result<(), String> {
        let actor_id = actor_id.to_string();
        let definition = self
            .combat
            .actor_stat_defs
            .get(stat_key)
            .ok_or_else(|| format!("unknown actor stat '{stat_key}'"))?
            .clone();
        let adjusted = self.actor_stat(&actor_id, stat_key) + delta;
        let clamped = if stat_key == content.settings.combat.health_stat_id || stat_key == "mp" {
            // The health and mana pools are resources that can be depleted below their
            // natural maximum (seed + level growth) but never refilled above
            // it. Clamping positive deltas against the declared content max
            // lets a heal or regen overshoot the value displayed as max, so cap it at
            // the actor's natural maximum instead.
            adjusted.clamp(
                definition.min.unwrap_or(i32::MIN),
                self.actor_stat_maximum(content, &actor_id, stat_key),
            )
        } else {
            definition.clamp(adjusted)
        };
        self.combat
            .actor_stats
            .entry(actor_id)
            .or_default()
            .insert(stat_key.to_string(), clamped);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_level_is_not_capped_by_character_level() {
        let content = crate::engine::test_fixtures::minimal_test_pack();
        let mut state = WorldState::new(&content);
        state.set_actor_level("player", 1);
        assert_eq!(state.actor_level("player"), 1);

        // Can set skill level above character level
        state.set_actor_skill_level("player", "strike", 5);
        assert_eq!(state.actor_skill_level("player", "strike"), 5);

        // SP progression advances skill level regardless of character level
        state.set_actor_skill_level("player", "heal", 1);
        assert_eq!(state.actor_skill_level("player", "heal"), 1);
        // 25 SP reaches level 3
        state.award_skill_sp("player", "heal", 25);
        assert_eq!(state.actor_skill_level("player", "heal"), 3);
        assert_eq!(state.actor_level("player"), 1);
    }
}
