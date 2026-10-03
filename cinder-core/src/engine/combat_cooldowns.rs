use crate::content::types::ContentPack;
use crate::engine::state::WorldState;

const DEXTERITY_STAT_ID: &str = "dexterity";
const INTELLIGENCE_STAT_ID: &str = "intelligence";
const MAX_STAT_DRIVEN_COOLDOWN_MINUTES: u32 = 4;
const STAT_POINTS_PER_COOLDOWN_STEP: u32 = 3;
const MIN_COOLDOWN_MINUTES: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CombatCooldownKind {
    Physical,
    Spell,
}

pub(crate) fn actor_combat_cooldown_minutes(
    content: &ContentPack,
    state: &WorldState,
    actor_id: &str,
    kind: CombatCooldownKind,
) -> u32 {
    if content.stats.actor.contains_key(DEXTERITY_STAT_ID) {
        let stat_id = match kind {
            CombatCooldownKind::Physical => DEXTERITY_STAT_ID,
            CombatCooldownKind::Spell => INTELLIGENCE_STAT_ID,
        };
        return stat_driven_cooldown_minutes(
            state.effective_actor_stat(content, actor_id, stat_id),
        );
    }

    state
        .actor(content, actor_id)
        .map(|actor| {
            actor.attack_interval_minutes(content.settings.combat.default_attack_interval_minutes)
        })
        .unwrap_or(content.settings.combat.default_attack_interval_minutes)
}

fn stat_driven_cooldown_minutes(stat: i32) -> u32 {
    MAX_STAT_DRIVEN_COOLDOWN_MINUTES
        .saturating_sub(stat.max(0) as u32 / STAT_POINTS_PER_COOLDOWN_STEP)
        .max(MIN_COOLDOWN_MINUTES)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::types::StatDefinition;

    #[test]
    fn stat_cooldowns_are_whole_minutes_with_a_one_minute_floor() {
        assert_eq!(stat_driven_cooldown_minutes(0), 4);
        assert_eq!(stat_driven_cooldown_minutes(3), 3);
        assert_eq!(stat_driven_cooldown_minutes(6), 2);
        assert_eq!(stat_driven_cooldown_minutes(9), 1);
        assert_eq!(stat_driven_cooldown_minutes(30), 1);
    }

    #[test]
    fn dexterity_and_intelligence_drive_their_respective_action_types() {
        let mut content = crate::engine::test_fixtures::minimal_test_pack();
        content.stats.actor.insert(
            DEXTERITY_STAT_ID.to_string(),
            StatDefinition {
                default: 6,
                min: Some(0),
                max: Some(10),
                time_step_minutes: None,
            },
        );
        let actor_id = content.actors[0].id.clone();
        let mut state = WorldState::new(&content);
        state
            .actor_stats
            .entry(actor_id.clone())
            .or_default()
            .insert(DEXTERITY_STAT_ID.to_string(), 9);
        state
            .actor_stats
            .entry(actor_id.clone())
            .or_default()
            .insert(INTELLIGENCE_STAT_ID.to_string(), 3);

        assert_eq!(
            actor_combat_cooldown_minutes(
                &content,
                &state,
                &actor_id,
                CombatCooldownKind::Physical,
            ),
            1
        );
        assert_eq!(
            actor_combat_cooldown_minutes(&content, &state, &actor_id, CombatCooldownKind::Spell,),
            3
        );
    }

    #[test]
    fn packs_without_dexterity_keep_their_authored_attack_interval() {
        let mut content = crate::engine::test_fixtures::minimal_test_pack();
        content.actors[0].attack_interval_minutes = Some(3);
        let actor_id = content.actors[0].id.clone();
        let state = WorldState::new(&content);

        assert_eq!(
            actor_combat_cooldown_minutes(
                &content,
                &state,
                &actor_id,
                CombatCooldownKind::Physical,
            ),
            3
        );
    }
}
