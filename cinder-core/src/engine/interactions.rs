//! Interactions: the authored choices a player can make about the actors and
//! features in the room they are standing in.

use crate::content::types::{
    ActorDefinition, CommandEffect, Condition, ContentPack, Interaction, RoomDefinition,
    RoomFeatureDefinition,
};
use crate::engine::conditions::{ConditionContext, all_hold, story_var_is_truthy};
use crate::engine::state::WorldState;

/// What an interaction belongs to.
#[derive(Clone, Copy)]
pub(crate) enum InteractionOwner<'a> {
    Actor(&'a ActorDefinition),
    Feature {
        room: &'a RoomDefinition,
        feature: &'a RoomFeatureDefinition,
    },
}

/// An interaction on offer, with the thing it belongs to.
#[derive(Clone, Copy)]
pub(crate) struct RoomInteraction<'a> {
    pub owner: InteractionOwner<'a>,
    pub interaction: &'a Interaction,
}

impl RoomInteraction<'_> {
    /// The id of the actor or feature that owns it, as typed in a command.
    pub fn owner_id(&self) -> &str {
        match self.owner {
            InteractionOwner::Actor(actor) => &actor.id,
            InteractionOwner::Feature { feature, .. } => &feature.id,
        }
    }

    /// The owner as it travels in events and state: `actor:wren` or
    /// `feature:playroom/lamp`.
    pub fn owner_key(&self) -> String {
        match self.owner {
            InteractionOwner::Actor(actor) => format!("actor:{}", actor.id),
            InteractionOwner::Feature { room, feature } => {
                format!("feature:{}/{}", room.id, feature.id)
            }
        }
    }

    pub fn is_actor(&self) -> bool {
        matches!(self.owner, InteractionOwner::Actor(_))
    }
}

/// The story variable that records an interaction as used.
pub(crate) fn interaction_used_var(owner_key: &str, interaction_id: &str) -> String {
    format!("interaction_used:{owner_key}:{interaction_id}")
}

fn is_available(
    content: &ContentPack,
    state: &WorldState,
    room_id: &str,
    owner_key: &str,
    interaction: &Interaction,
) -> bool {
    all_hold(
        &interaction.when,
        &ConditionContext::new(content, state, room_id),
    ) && (interaction.repeatable
        || !story_var_is_truthy(state, &interaction_used_var(owner_key, &interaction.id)))
}

/// The interactions on offer in the player's room, from its actors and its
/// features.
pub(crate) fn room_interactions<'a>(
    content: &'a ContentPack,
    state: &'a WorldState,
) -> Vec<RoomInteraction<'a>> {
    let room_id = state.current_room_id.as_str();
    let mut offered = Vec::new();
    for actor in state.actors(content) {
        if content.is_player_actor(&actor.id)
            || state.actor_room_id(&actor.id, &actor.room_id) != room_id
        {
            continue;
        }
        for interaction in &actor.interactions {
            let candidate = RoomInteraction {
                owner: InteractionOwner::Actor(actor),
                interaction,
            };
            if is_available(content, state, room_id, &candidate.owner_key(), interaction) {
                offered.push(candidate);
            }
        }
    }
    if let Some(room) = content.room(room_id) {
        for feature in &room.features {
            for interaction in &feature.interactions {
                let candidate = RoomInteraction {
                    owner: InteractionOwner::Feature { room, feature },
                    interaction,
                };
                if is_available(content, state, room_id, &candidate.owner_key(), interaction) {
                    offered.push(candidate);
                }
            }
        }
    }
    offered
}

/// An offered interaction that needs a carried item, as the Use panel lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemInteraction {
    pub label: String,
    /// The command that carries it out, e.g. `do cage:unlock`.
    pub command: String,
}

fn needs_item(conditions: &[Condition], item_id: &str) -> bool {
    conditions.iter().any(|condition| match condition {
        Condition::Carries(id) => id == item_id,
        Condition::All(all) => needs_item(all, item_id),
        _ => false,
    })
}

/// The interactions on offer in the player's room that need `item_id`, such as a
/// key that opens something here.
pub fn item_interactions(
    content: &ContentPack,
    state: &WorldState,
    item_id: &str,
) -> Vec<ItemInteraction> {
    let Some(action) = content
        .actions
        .iter()
        .find(|action| action.player_enabled && action.has_effect(CommandEffect::UseInteraction))
    else {
        return Vec::new();
    };
    let verb = action.phrases.first().map(String::as_str).unwrap_or("do");
    room_interactions(content, state)
        .into_iter()
        .filter(|offered| needs_item(&offered.interaction.when, item_id))
        .map(|offered| ItemInteraction {
            label: offered.interaction.label.clone(),
            command: format!("{verb} {}:{}", offered.owner_id(), offered.interaction.id),
        })
        .collect()
}

/// Finds an offered interaction from `owner:interaction`, or from its label as
/// typed.
pub(crate) fn resolve_interaction<'a>(
    content: &'a ContentPack,
    state: &'a WorldState,
    input: &str,
) -> Option<RoomInteraction<'a>> {
    let input = input.trim();
    room_interactions(content, state)
        .into_iter()
        .find(|offered| {
            format!("{}:{}", offered.owner_id(), offered.interaction.id).eq_ignore_ascii_case(input)
                || offered.interaction.label.eq_ignore_ascii_case(input)
                || offered
                    .interaction
                    .aliases
                    .iter()
                    .any(|alias| alias.eq_ignore_ascii_case(input))
        })
}

/// Where a typed interaction is declared, for explaining why it is not on offer
/// right now: the title of the room whose feature owns it, if it is not the
/// player's room.
pub(crate) fn declared_elsewhere<'a>(
    content: &'a ContentPack,
    current_room_id: &str,
    input: &str,
) -> Option<&'a str> {
    let input = input.trim();
    content
        .rooms
        .iter()
        .filter(|room| room.id != current_room_id)
        .find(|room| {
            room.features.iter().any(|feature| {
                feature.interactions.iter().any(|interaction| {
                    format!("{}:{}", feature.id, interaction.id).eq_ignore_ascii_case(input)
                        || interaction.label.eq_ignore_ascii_case(input)
                        || interaction
                            .aliases
                            .iter()
                            .any(|alias| alias.eq_ignore_ascii_case(input))
                })
            })
        })
        .map(|room| room.title.as_str())
}

/// The interaction an event refers to, whether or not it is still on offer.
pub(crate) fn find_interaction<'a>(
    content: &'a ContentPack,
    owner_key: &str,
    interaction_id: &str,
) -> Option<RoomInteraction<'a>> {
    if let Some(actor_id) = owner_key.strip_prefix("actor:") {
        let actor = content.actor(actor_id)?;
        let interaction = actor.interactions.iter().find(|i| i.id == interaction_id)?;
        return Some(RoomInteraction {
            owner: InteractionOwner::Actor(actor),
            interaction,
        });
    }
    let (room_id, feature_id) = owner_key.strip_prefix("feature:")?.split_once('/')?;
    let room = content.room(room_id)?;
    let feature = room.features.iter().find(|f| f.id == feature_id)?;
    let interaction = feature
        .interactions
        .iter()
        .find(|i| i.id == interaction_id)?;
    Some(RoomInteraction {
        owner: InteractionOwner::Feature { room, feature },
        interaction,
    })
}
