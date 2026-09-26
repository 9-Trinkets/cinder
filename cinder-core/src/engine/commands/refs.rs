use crate::content::types::{ActorDefinition, ContentPack};
use crate::engine::state::{WorldState, current_cast_member_actor_id, display_actor_name};

#[derive(Debug)]
pub(crate) struct ResolvedActorReferenceInput {
    pub actor_id: String,
    pub actor_name: String,
    pub player_message: Option<String>,
    pub actor_in_room: bool,
}

pub(crate) fn resolve_actor_reference_input(
    content: &ContentPack,
    state: &WorldState,
    current_room_id: &str,
    remainder: &str,
) -> Option<ResolvedActorReferenceInput> {
    match_actor_reference(
        state,
        state.onstage_actors(content).filter(|actor| {
            !content.is_player_actor(&actor.id)
                && !state.actor_is_defeated(&actor.id, &content.settings.combat.health_stat_id)
                && state.actor_is_in_room(content, &actor.id, current_room_id)
        }),
        remainder,
        &content.settings.act_member_alias,
    )
    .map(|(actor, player_message)| ResolvedActorReferenceInput {
        actor_id: actor.id.clone(),
        actor_name: display_actor_name(state, actor),
        player_message,
        actor_in_room: true,
    })
    .or_else(|| {
        match_actor_reference(
            state,
            state.actors(content).filter(|actor| {
                !content.is_player_actor(&actor.id)
                    && !state.actor_is_defeated(&actor.id, &content.settings.combat.health_stat_id)
            }),
            remainder,
            &content.settings.act_member_alias,
        )
        .map(|(actor, player_message)| ResolvedActorReferenceInput {
            actor_id: actor.id.clone(),
            actor_name: display_actor_name(state, actor),
            player_message,
            actor_in_room: state.actor_is_in_room(content, &actor.id, current_room_id),
        })
    })
}

pub(crate) fn unknown_target_token(remainder: &str) -> String {
    remainder
        .split_whitespace()
        .next()
        .unwrap_or(remainder)
        .trim()
        .to_string()
}

fn match_actor_reference<'a>(
    state: &WorldState,
    actors: impl IntoIterator<Item = &'a ActorDefinition>,
    remainder: &str,
    act_member_alias: &str,
) -> Option<(&'a ActorDefinition, Option<String>)> {
    let trimmed = remainder.trim();
    let lower = trimmed.to_ascii_lowercase();
    let mut best: Option<(&'a ActorDefinition, usize, Option<String>)> = None;
    for actor in actors {
        for reference in actor_references(state, actor, act_member_alias) {
            let reference_lower = reference.to_ascii_lowercase();
            let exact = lower == reference_lower;
            let prefix = lower
                .strip_prefix(&reference_lower)
                .and_then(|rest| rest.strip_prefix(' '));
            if exact || prefix.is_some() {
                let player_message = if exact {
                    None
                } else {
                    let tail = trimmed[reference.len()..].trim();
                    if tail.is_empty() {
                        None
                    } else {
                        Some(tail.to_string())
                    }
                };
                let reference_len = reference.len();
                if best
                    .as_ref()
                    .is_none_or(|(_, best_len, _)| reference_len > *best_len)
                {
                    best = Some((actor, reference_len, player_message));
                }
            }
        }
    }
    best.map(|(actor, _, player_message)| (actor, player_message))
}

fn actor_references(
    state: &WorldState,
    actor: &ActorDefinition,
    act_member_alias: &str,
) -> Vec<String> {
    let mut refs = vec![
        actor.name.clone(),
        actor.id.clone(),
        display_actor_name(state, actor),
    ];
    if current_cast_member_actor_id(state).is_some_and(|actor_id| actor_id == actor.id)
        && !act_member_alias.is_empty()
    {
        refs.push(act_member_alias.to_string());
    }
    refs.extend(actor.aliases.iter().cloned());
    refs
}