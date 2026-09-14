mod attacks;
mod defeat;
mod periodic;

use crate::content::types::ContentPack;
use crate::engine::state::WorldState;

static VEC_EMPTY_TAGS: Vec<String> = Vec::new();

pub(super) use attacks::apply_attack_target;
pub(super) use defeat::{award_defeat_xp, defeat_actor, defeat_player_if_dead, spawn_defeat_drops};
pub(super) use periodic::handle_periodic_actor_effect_applied;

pub(super) fn actor_display_name(content: &ContentPack, actor_id: &str) -> String {
    content
        .actor(actor_id)
        .map(|actor| actor.name.clone())
        .unwrap_or_else(|| actor_id.to_string())
}

/// Applies a target actor's content-declared resistance to `damage` dealt in
/// `kind`. Each point of resistance removes that much damage; the result is
/// floored at zero so a fully-resisted hit yields true immunity (no minimum
/// damage floor survives resistance). Negative resistance amplifies damage.
pub(super) fn resisted_damage(
    content: &ContentPack,
    target_actor_id: &str,
    kind: &str,
    damage: i32,
) -> i32 {
    let resistance = content
        .actor(target_actor_id)
        .map(|actor| actor.resistances.get(kind).copied().unwrap_or(0))
        .unwrap_or(0);
    (damage - resistance).max(0)
}

fn adjust_actor_stat(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    stat: &str,
    delta: i32,
) -> i32 {
    state
        .adjust_actor_stat(content, actor_id, stat, delta)
        .unwrap_or_else(|error| eprintln!("[cinder] combat stat error: {error}"));
    state.actor_stat(actor_id, stat)
}
