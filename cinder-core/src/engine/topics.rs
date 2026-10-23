//! Authored actor topics: the things the player can say from the Talk panel.

use crate::content::types::{ActorDefinition, ActorTopicDefinition, ContentPack};
use crate::engine::state::WorldState;
use crate::engine::turn_policies::story_var_is_truthy;

/// Story variable that records a topic as used.
pub(crate) fn topic_used_var(actor_id: &str, topic_id: &str) -> String {
    format!("topic_used:{actor_id}:{topic_id}")
}

pub(crate) fn topic_is_available(
    state: &WorldState,
    actor: &ActorDefinition,
    topic: &ActorTopicDefinition,
) -> bool {
    (topic.requires_story_var.is_empty() || story_var_is_truthy(state, &topic.requires_story_var))
        && (topic.repeatable || !story_var_is_truthy(state, &topic_used_var(&actor.id, &topic.id)))
}

/// Topics on offer from the actors in the player's room, as `(actor, topic)`.
pub(crate) fn room_topics<'a>(
    content: &'a ContentPack,
    state: &'a WorldState,
) -> Vec<(&'a ActorDefinition, &'a ActorTopicDefinition)> {
    state
        .actors(content)
        .filter(|actor| !content.is_player_actor(&actor.id))
        .filter(|actor| state.actor_room_id(&actor.id, &actor.room_id) == state.current_room_id)
        .flat_map(|actor| actor.topics.iter().map(move |topic| (actor, topic)))
        .filter(|(actor, topic)| topic_is_available(state, actor, topic))
        .collect()
}

/// Finds an offered topic from `actor:topic` or from the label as typed.
pub(crate) fn resolve_topic<'a>(
    content: &'a ContentPack,
    state: &'a WorldState,
    input: &str,
) -> Option<(&'a ActorDefinition, &'a ActorTopicDefinition)> {
    let input = input.trim();
    room_topics(content, state)
        .into_iter()
        .find(|(actor, topic)| {
            format!("{}:{}", actor.id, topic.id).eq_ignore_ascii_case(input)
                || topic.label.eq_ignore_ascii_case(input)
        })
}
