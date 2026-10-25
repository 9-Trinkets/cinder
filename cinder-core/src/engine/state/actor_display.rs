//! How an actor is named and described to the dialogue model right now.

use super::WorldState;
use crate::content::types::{ActorDefinition, ActorPromptContext, ContentPack};

/// The name the player sees for an actor, after any rename.
pub fn display_actor_name(state: &WorldState, actor: &ActorDefinition) -> String {
    state
        .actors
        .actor_name_overrides
        .get(&actor.id)
        .cloned()
        .unwrap_or_else(|| actor.name.clone())
}

/// The actor's prompt context, replaced by the rename's own once the actor has
/// been transformed.
pub fn resolved_actor_prompt_context(
    content: &ContentPack,
    state: &WorldState,
    actor: &ActorDefinition,
) -> ActorPromptContext {
    let Some(rename) = content
        .applied_transformation(state, &actor.id)
        .and_then(|transformation| transformation.rename.as_ref())
    else {
        return actor.prompt_context.clone();
    };
    if let Some(prompt_context) = &rename.prompt_context {
        return prompt_context.clone();
    }
    let mut base = actor.prompt_context.clone();
    base.character_notes.push(format!(
        "You have awakened and remembered who you are: {}, {}.",
        rename.name, rename.who
    ));
    if !rename.fragment.is_empty() {
        base.subtext_notes.push(format!(
            "Your true memory has returned. Fragment: '{}'",
            rename.fragment
        ));
    }
    base.response_notes.push(
        "You can speak freely in human language. Speak with your true voice and share your memories and guidance with the party."
            .to_string(),
    );
    base
}

/// Story text with story variables and `{actor.<id>.<stat>}` filled in.
pub fn render_dynamic_story_text(template: &str, state: &WorldState) -> String {
    let mut rendered = state.story_vars.render_template(template);
    for (actor_id, stats) in &state.combat.actor_stats {
        for (stat_key, stat_value) in stats {
            rendered = rendered.replace(
                &format!("{{actor.{actor_id}.{stat_key}}}"),
                &stat_value.to_string(),
            );
        }
    }
    rendered
}
