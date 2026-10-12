//! 10-level Skill Proficiency (SP) system and pure scalar parameter scaling.

/// Cumulative SP thresholds for skill levels 1 through 10.
/// Deliberately arithmetic (+10, +15, +20, +25, +30, +35, +40, +45, +50):
/// 0, 10, 25, 45, 70, 100, 135, 175, 220, 270.
pub const SKILL_SP_THRESHOLDS: [u32; 10] = [0, 10, 25, 45, 70, 100, 135, 175, 220, 270];

/// Calculates the skill level (1..=10) achieved for a given cumulative SP.
pub fn skill_level_from_sp(sp: u32) -> u32 {
    let mut level = 1;
    for (idx, &threshold) in SKILL_SP_THRESHOLDS.iter().enumerate().skip(1) {
        if sp >= threshold {
            level = (idx + 1) as u32;
        } else {
            break;
        }
    }
    level
}

/// Cumulative SP required to reach `level` (1..=10).
pub fn skill_sp_for_level(level: u32) -> u32 {
    let idx = (level.max(1).min(10) - 1) as usize;
    SKILL_SP_THRESHOLDS[idx]
}

/// Cumulative SP required for the next level after `level`. At level 10, returns 270.
pub fn skill_next_level_sp(level: u32) -> u32 {
    if level >= 10 {
        SKILL_SP_THRESHOLDS[9]
    } else {
        SKILL_SP_THRESHOLDS[level as usize]
    }
}

/// Follower cap for charm: 2 * skill_level (2 to 20).
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

/// Active sigil capacity for trace:
/// Starts at 4 at Level 1 and scales: 4, 5, 6, 7, 8, 9, 10, 11, 12, 14.
pub fn trace_sigil_capacity(level: u32) -> usize {
    match level.clamp(1, 10) {
        1 => 4,
        2 => 5,
        3 => 6,
        4 => 7,
        5 => 8,
        6 => 9,
        7 => 10,
        8 => 11,
        9 => 12,
        _ => 14,
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

/// MP cost for teleporting: 6 MP down to 1 MP.
pub fn teleport_mp_cost(level: u32) -> u32 {
    match level.clamp(1, 10) {
        1 => 6,
        2..=3 => 5,
        4..=5 => 4,
        6..=7 => 3,
        8..=9 => 2,
        _ => 1,
    }
}

/// Pure scalar summary string for UI display.
pub fn skill_scalar_summary(skill_id: &str, level: u32) -> String {
    let lvl = level.clamp(1, 10);
    match skill_id {
        "charm" => format!("Follower cap: {}", charm_follower_cap(lvl)),
        "strike" => format!("Bonus damage: +{}", strike_bonus_damage(lvl)),
        "heal" => format!("Restores: {} HP", heal_amount(lvl)),
        "intercept" => format!("Damage soaked: -{}", intercept_damage_soaked(lvl)),
        "trace" => format!("Active sigils: {}", trace_sigil_capacity(lvl)),
        "drain" => format!(
            "Damage: {} · Charges: {}",
            drain_damage(lvl),
            drain_charges(lvl)
        ),
        "teleport" => format!(
            "Anchors: {} · MP cost: {}",
            teleport_anchor_capacity(lvl),
            teleport_mp_cost(lvl)
        ),
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
        assert_eq!(charm_follower_cap(1), 2);
        assert_eq!(charm_follower_cap(10), 20);

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
        assert_eq!(trace_sigil_capacity(1), 4);
        assert_eq!(trace_sigil_capacity(9), 12);
        assert_eq!(trace_sigil_capacity(10), 14);

        // Drain
        assert_eq!(drain_damage(1), 2);
        assert_eq!(drain_damage(10), 11);
        assert_eq!(drain_charges(1), 4);
        assert_eq!(drain_charges(10), 13);

        // Teleport
        assert_eq!(teleport_anchor_capacity(1), 1);
        assert_eq!(teleport_anchor_capacity(10), 6);
        assert_eq!(teleport_mp_cost(1), 6);
        assert_eq!(teleport_mp_cost(10), 1);
    }
}
