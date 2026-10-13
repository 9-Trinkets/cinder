//! 10-level Skill Proficiency (SP) system and pure scalar parameter scaling.

/// Cumulative SP thresholds for skill levels 1 through 10.
/// Deliberately arithmetic (+10, +15, +20, +25, +30, +35, +40, +45, +50):
/// 0, 10, 25, 45, 70, 100, 135, 175, 220, 270.
pub const SKILL_SP_THRESHOLDS: [u32; 10] = [0, 10, 25, 45, 70, 100, 135, 175, 220, 270];

/// Calculates the skill level (1..=thresholds.len()) achieved for a given cumulative SP using provided thresholds.
pub fn skill_level_from_sp_with_thresholds(sp: u32, thresholds: &[u32]) -> u32 {
    let mut level = 1;
    for (idx, &threshold) in thresholds.iter().enumerate().skip(1) {
        if sp >= threshold {
            level = (idx + 1) as u32;
        } else {
            break;
        }
    }
    level
}

/// Calculates the skill level (1..=10) achieved for a given cumulative SP.
pub fn skill_level_from_sp(sp: u32) -> u32 {
    skill_level_from_sp_with_thresholds(sp, &SKILL_SP_THRESHOLDS)
}

/// Cumulative SP required to reach `level` using provided thresholds.
pub fn skill_sp_for_level_with_thresholds(level: u32, thresholds: &[u32]) -> u32 {
    if thresholds.is_empty() {
        return 0;
    }
    let idx = (level.max(1) as usize - 1).min(thresholds.len() - 1);
    thresholds[idx]
}

/// Cumulative SP required to reach `level` (1..=10).
pub fn skill_sp_for_level(level: u32) -> u32 {
    skill_sp_for_level_with_thresholds(level, &SKILL_SP_THRESHOLDS)
}

/// Cumulative SP required for the next level after `level` using provided thresholds.
pub fn skill_next_level_sp_with_thresholds(level: u32, thresholds: &[u32]) -> u32 {
    if thresholds.is_empty() {
        return 0;
    }
    let idx = (level as usize).min(thresholds.len() - 1);
    thresholds[idx]
}

/// Cumulative SP required for the next level after `level`. At level 10, returns 270.
pub fn skill_next_level_sp(level: u32) -> u32 {
    skill_next_level_sp_with_thresholds(level, &SKILL_SP_THRESHOLDS)
}

/// Charm bonus added to surround conversion check: 2 * skill_level (2 to 20).
pub fn charm_bonus(level: u32) -> u32 {
    2 * level.clamp(1, 10)
}

/// Follower cap for charm (compatibility): 2 * skill_level (2 to 20).
pub fn charm_follower_cap(level: u32) -> usize {
    (2 * level.clamp(1, 10)) as usize
}

/// Flat bonus damage for strike: skill_level - 1 (+0 to +9).
pub fn strike_bonus_damage(level: u32) -> i32 {
    level.clamp(1, 10).saturating_sub(1) as i32
}

/// Health restored by heal: 4 + (skill_level - 1) * 3 (4 to 31).
pub fn heal_amount(level: u32) -> i32 {
    4 + (level.clamp(1, 10).saturating_sub(1) as i32) * 3
}

/// Flat damage soaked by intercept: floor((skill_level - 1) / 2) (0 to 5).
pub fn intercept_damage_soaked(level: u32) -> i32 {
    let lvl = level.clamp(1, 10);
    if lvl >= 10 {
        5
    } else {
        (lvl.saturating_sub(1) / 2) as i32
    }
}

/// MP discount for tracing sigils and sigil spells by Trace skill level (1..=10).
/// Scales 0 to 4 MP discount across levels 1..=10:
/// Levels 1-2: 0 MP
/// Levels 3-4: 1 MP
/// Levels 5-6: 2 MP
/// Levels 7-8: 3 MP
/// Levels 9-10: 4 MP
pub fn trace_mp_discount(level: u32) -> u32 {
    let lvl = level.clamp(1, 10);
    lvl.saturating_sub(1) / 2
}

/// Computes the effective MP cost for tracing a sigil or casting a sigil spell,
/// discounted by Trace skill level, and never reducing below 1 (unless base was 0).
pub fn effective_trace_mp_cost(base_mp: u32, trace_level: u32) -> u32 {
    if base_mp == 0 {
        0
    } else {
        let discount = trace_mp_discount(trace_level);
        base_mp.saturating_sub(discount).max(1)
    }
}

/// Computes the effective MP cost for tracing a sigil or casting a sigil spell,
/// discounted by Trace skill level looking up content configuration when available.
pub fn effective_trace_mp_cost_for_pack(
    content: &crate::content::types::ContentPack,
    base_mp: u32,
    trace_level: u32,
) -> u32 {
    if base_mp == 0 {
        0
    } else {
        let discount = content
            .skill("trace")
            .and_then(|s| s.param_value("mp_discount", trace_level))
            .map(|d| d as u32)
            .unwrap_or_else(|| trace_mp_discount(trace_level));
        base_mp.saturating_sub(discount).max(1)
    }
}

/// Damage dealt per drain trigger: 2 + (skill_level - 1) (2 to 11).
pub fn drain_damage(level: u32) -> i32 {
    2 + (level.clamp(1, 10).saturating_sub(1) as i32)
}

/// Charges per drain sigil: 3 + skill_level (4 to 13).
pub fn drain_charges(level: u32) -> u32 {
    3 + level.clamp(1, 10)
}

/// Temporary chalk anchor capacity for teleport: 1 to 6.
pub fn teleport_anchor_capacity(level: u32) -> usize {
    match level.clamp(1, 10) {
        1..=2 => 1,
        3..=4 => 2,
        5..=6 => 3,
        7..=8 => 4,
        9 => 5,
        _ => 6,
    }
}

/// Base MP cost for casting teleportation.
pub const TELEPORT_BASE_MP_COST: u32 = 6;

/// Pure scalar summary string for UI display.
pub fn skill_scalar_summary(skill_id: &str, level: u32) -> String {
    let lvl = level.clamp(1, 10);
    match skill_id {
        "charm" => format!("Charm bonus: +{}", charm_bonus(lvl)),
        "strike" => format!("Bonus damage: +{}", strike_bonus_damage(lvl)),
        "heal" => format!("Restores: {} HP", heal_amount(lvl)),
        "intercept" => format!("Damage soaked: -{}", intercept_damage_soaked(lvl)),
        "trace" => {
            let discount = trace_mp_discount(lvl);
            if discount == 0 {
                "Base MP cost".to_string()
            } else {
                format!("-{} Tracing MP", discount)
            }
        }
        "drain" => format!(
            "Damage: {} · Charges: {}",
            drain_damage(lvl),
            drain_charges(lvl)
        ),
        "teleport" => format!("Anchors: {}", teleport_anchor_capacity(lvl)),
        "hold" => "Holds ground in room".to_string(),
        "comms" => "Telepathic link with awakened allies".to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sp_thresholds_and_levels() {
        assert_eq!(skill_level_from_sp(0), 1);
        assert_eq!(skill_level_from_sp(9), 1);
        assert_eq!(skill_level_from_sp(10), 2);
        assert_eq!(skill_level_from_sp(24), 2);
        assert_eq!(skill_level_from_sp(25), 3);
        assert_eq!(skill_level_from_sp(270), 10);
        assert_eq!(skill_level_from_sp(500), 10);

        assert_eq!(skill_sp_for_level(1), 0);
        assert_eq!(skill_sp_for_level(2), 10);
        assert_eq!(skill_sp_for_level(10), 270);

        assert_eq!(skill_next_level_sp(1), 10);
        assert_eq!(skill_next_level_sp(9), 270);
        assert_eq!(skill_next_level_sp(10), 270);
    }

    #[test]
    fn scalar_values_match_spec() {
        // Charm
        assert_eq!(charm_bonus(1), 2);
        assert_eq!(charm_bonus(10), 20);

        // Strike
        assert_eq!(strike_bonus_damage(1), 0);
        assert_eq!(strike_bonus_damage(10), 9);

        // Heal
        assert_eq!(heal_amount(1), 4);
        assert_eq!(heal_amount(10), 31);

        // Intercept
        assert_eq!(intercept_damage_soaked(1), 0);
        assert_eq!(intercept_damage_soaked(10), 5);

        // Trace
        assert_eq!(trace_mp_discount(1), 0);
        assert_eq!(trace_mp_discount(2), 0);
        assert_eq!(trace_mp_discount(3), 1);
        assert_eq!(trace_mp_discount(5), 2);
        assert_eq!(trace_mp_discount(10), 4);

        assert_eq!(effective_trace_mp_cost(1, 1), 1); // Never drops below 1
        assert_eq!(effective_trace_mp_cost(1, 10), 1);
        assert_eq!(effective_trace_mp_cost(5, 5), 3); // 5 - 2 = 3
        assert_eq!(effective_trace_mp_cost(5, 10), 1); // 5 - 4 = 1

        // Drain
        assert_eq!(drain_damage(1), 2);
        assert_eq!(drain_damage(10), 11);
        assert_eq!(drain_charges(1), 4);
        assert_eq!(drain_charges(10), 13);

        // Teleport
        assert_eq!(teleport_anchor_capacity(1), 1);
        assert_eq!(teleport_anchor_capacity(10), 6);
    }

    #[test]
    fn content_declarative_skill_scaling_and_summaries() {
        let pack = crate::loader::load_named_pack("layla", Some("en")).expect("layla loads");

        // Global pack SP thresholds
        assert_eq!(pack.skill_sp_thresholds("strike"), &SKILL_SP_THRESHOLDS);

        // Strike
        let strike = pack.skill("strike").expect("strike exists");
        assert_eq!(strike.param_value("bonus_damage", 1), Some(0));
        assert_eq!(strike.param_value("bonus_damage", 5), Some(4));
        assert_eq!(strike.param_value("bonus_damage", 10), Some(9));
        assert_eq!(
            strike.format_summary(1),
            Some("Bonus damage: +0".to_string())
        );
        assert_eq!(
            strike.format_summary(10),
            Some("Bonus damage: +9".to_string())
        );

        // Charm
        let charm = pack.skill("charm").expect("charm exists");
        assert_eq!(charm.param_value("charm_bonus", 1), Some(2));
        assert_eq!(charm.param_value("charm_bonus", 10), Some(20));
        assert_eq!(charm.format_summary(1), Some("Charm bonus: +2".to_string()));
        assert_eq!(
            charm.format_summary(10),
            Some("Charm bonus: +20".to_string())
        );

        // Heal
        let heal = pack.skill("heal").expect("heal exists");
        assert_eq!(heal.param_value("amount", 1), Some(4));
        assert_eq!(heal.param_value("amount", 10), Some(31));
        assert_eq!(heal.format_summary(1), Some("Restores: 4 HP".to_string()));
        assert_eq!(heal.format_summary(10), Some("Restores: 31 HP".to_string()));

        // Intercept
        let intercept = pack.skill("intercept").expect("intercept exists");
        assert_eq!(intercept.param_value("damage_soaked", 1), Some(0));
        assert_eq!(intercept.param_value("damage_soaked", 2), Some(0));
        assert_eq!(intercept.param_value("damage_soaked", 3), Some(1));
        assert_eq!(intercept.param_value("damage_soaked", 10), Some(5));
        assert_eq!(
            intercept.format_summary(1),
            Some("Damage soaked: -0".to_string())
        );
        assert_eq!(
            intercept.format_summary(10),
            Some("Damage soaked: -5".to_string())
        );

        // Trace
        let trace = pack.skill("trace").expect("trace exists");
        assert_eq!(trace.param_value("mp_discount", 1), Some(0));
        assert_eq!(trace.param_value("mp_discount", 2), Some(0));
        assert_eq!(trace.param_value("mp_discount", 3), Some(1));
        assert_eq!(trace.param_value("mp_discount", 10), Some(4));
        assert_eq!(trace.format_summary(1), Some("Base MP cost".to_string()));
        assert_eq!(trace.format_summary(2), Some("Base MP cost".to_string()));
        assert_eq!(trace.format_summary(3), Some("-1 Tracing MP".to_string()));
        assert_eq!(trace.format_summary(10), Some("-4 Tracing MP".to_string()));

        // Teleport
        let teleport = pack.skill("teleport").expect("teleport exists");
        assert_eq!(teleport.param_value("anchor_capacity", 1), Some(1));
        assert_eq!(teleport.param_value("anchor_capacity", 10), Some(6));
        assert_eq!(teleport.format_summary(1), Some("Anchors: 1".to_string()));
        assert_eq!(teleport.format_summary(10), Some("Anchors: 6".to_string()));

        // Drain
        let drain = pack.skill("drain").expect("drain exists");
        assert_eq!(drain.param_value("damage", 1), Some(2));
        assert_eq!(drain.param_value("damage", 10), Some(11));
        assert_eq!(drain.param_value("charges", 1), Some(4));
        assert_eq!(drain.param_value("charges", 10), Some(13));
        assert_eq!(
            drain.format_summary(1),
            Some("Damage: 2 · Charges: 4".to_string())
        );
        assert_eq!(
            drain.format_summary(10),
            Some("Damage: 11 · Charges: 13".to_string())
        );

        // Static skills
        let hold = pack.skill("hold").expect("hold exists");
        assert_eq!(
            hold.format_summary(1),
            Some("Holds ground in room".to_string())
        );
        let comms = pack.skill("comms").expect("comms exists");
        assert_eq!(
            comms.format_summary(1),
            Some("Telepathic link with awakened allies".to_string())
        );
    }
}
