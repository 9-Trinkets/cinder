use crate::content::types::ContentPack;
use crate::engine::narrative::NarrativeLines;
use crate::engine::reducer::beat_advance::advance_objective_for_signal;
use crate::engine::state::WorldState;

/// Evaluates whether an actor meets the wisdom threshold to awaken.
/// When triggered, sets the actor as awakened, assigns their true name,
/// attaches them as a free ally following the player, and narrates the awakening.
pub fn check_follower_awakening(
    state: &mut WorldState,
    content: &ContentPack,
    actor_id: &str,
    lines: &mut NarrativeLines,
) -> bool {
    if content.is_player_actor(actor_id) {
        return false;
    }
    if state.is_actor_awakened(actor_id) {
        return false;
    }
    let Some(actor) = content.actor(actor_id) else {
        return false;
    };
    // Hard limit: Shaman and King cannot awaken
    if actor_id == "goblin-shaman" || actor_id.starts_with("elf-king") {
        return false;
    }
    let Some(awakening) = &actor.awakening else {
        return false;
    };
    let wisdom = state.effective_actor_stat(content, actor_id, "wisdom");
    if wisdom < awakening.required_wisdom {
        return false;
    }

    let original_name = actor.name.clone();
    let woken_name = awakening.name.clone();
    let fragment = awakening.fragment.clone();

    state.set_actor_awakened(actor_id, &woken_name);
    state.set_follows_player(actor_id, true);

    lines.narration(format!(
        "The warmth of high wisdom stirs deep within {original_name}. The fog in their mind shatters."
    ));
    if !fragment.is_empty() {
        lines.narration(format!("\"{fragment}\""));
    }
    lines.narration(format!(
        "{original_name} remembers who they are: {original_name} is now {woken_name}."
    ));

    lines.extend_narration(advance_objective_for_signal(
        state,
        content,
        &format!("actor_awakened:{actor_id}"),
    ));
    lines.extend_narration(advance_objective_for_signal(
        state,
        content,
        "actor_awakened",
    ));

    true
}
