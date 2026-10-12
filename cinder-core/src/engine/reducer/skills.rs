use crate::content::types::ContentPack;
use crate::engine::narrative::NarrativeLines;
use crate::engine::state::WorldState;

/// Awards SP to an actor's skill and dispatches the `actor.skill_used` hook.
/// Returns true if the skill leveled up.
pub fn award_actor_skill_sp(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    skill_id: &str,
    sp_delta: u32,
    lines: &mut NarrativeLines,
) -> bool {
    let leveled_up = state.award_skill_sp(actor_id, skill_id, sp_delta);
    if let Err(error) = crate::engine::hooks::apply_narrating_world_hook_effects(
        state,
        content,
        crate::engine::hook_ids::ACTOR_SKILL_USED,
        serde_json::json!({
            "actor_id": actor_id,
            "skill_id": skill_id,
            "sp": sp_delta,
            "leveled_up": leveled_up,
            "story_vars": state.story_vars.to_map(),
        }),
        lines,
    ) {
        eprintln!("[cinder] hook warning (actor.skill_used): {error}");
    }
    leveled_up
}
