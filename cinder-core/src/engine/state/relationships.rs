use super::*;
use crate::content::types::PartyOrderKind;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Relationships of actors toward the player's faction, plus the party's
/// follow target, standing orders, and reaction cooldowns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipGraph {
    /// Per-actor relationship toward the player's faction. Absent entries are
    /// neutral.
    #[serde(default)]
    pub relationships: BTreeMap<String, ActorRelationship>,
    /// Persistent Guard/Assist directive for each allied party member.
    #[serde(default)]
    pub party_orders: BTreeMap<String, PartyOrderKind>,
    /// Target actor that this actor is following, actor id → followed actor id.
    #[serde(default)]
    pub followed_actors: BTreeMap<String, String>,
    pub followed_actor_id: Option<String>,
    /// Earliest game minute at which each party member may react again.
    #[serde(default)]
    pub party_reaction_ready_at: BTreeMap<String, u32>,
}

impl WorldState {
    /// Relationship of an actor toward the player's faction; absent entries
    /// are neutral.
    pub fn relationship(&self, actor_id: &str) -> ActorRelationship {
        let actor_id = remap_story_actor_id(self, actor_id);
        self.relations
            .relationships
            .get(actor_id)
            .copied()
            .unwrap_or_default()
    }

    pub fn stance(&self, actor_id: &str) -> ActorStance {
        self.relationship(actor_id).stance
    }

    pub fn follows_player(&self, actor_id: &str) -> bool {
        self.relationship(actor_id).follows_player
    }

    /// Writes a full relationship snapshot. Writing the default removes the
    /// entry entirely, keeping state sparse.
    pub fn set_relationship(&mut self, actor_id: &str, relationship: ActorRelationship) {
        let actor_id = remap_story_actor_id(self, actor_id).to_string();
        if relationship == ActorRelationship::default() {
            self.relations.relationships.remove(actor_id.as_str());
        } else {
            self.relations
                .relationships
                .insert(actor_id.clone(), relationship);
        }
        if relationship.stance != ActorStance::Hostile {
            self.combat.next_hostile_strike_at.remove(actor_id.as_str());
        }
    }

    pub fn set_stance(&mut self, actor_id: &str, stance: ActorStance) {
        let mut relationship = self.relationship(actor_id);
        relationship.stance = stance;
        self.set_relationship(actor_id, relationship);
    }

    pub fn set_follows_player(&mut self, actor_id: &str, follows_player: bool) {
        let mut relationship = self.relationship(actor_id);
        relationship.follows_player = follows_player;
        self.set_relationship(actor_id, relationship);
    }

    pub fn followed_actor(&self, actor_id: &str) -> Option<&str> {
        let actor_id = remap_story_actor_id(self, actor_id);
        self.relations
            .followed_actors
            .get(actor_id)
            .map(String::as_str)
    }

    pub fn set_followed_actor(&mut self, actor_id: &str, followed_id: Option<&str>) {
        let actor_id = remap_story_actor_id(self, actor_id).to_string();
        if let Some(target) = followed_id {
            self.relations
                .followed_actors
                .insert(actor_id, target.to_string());
        } else {
            self.relations.followed_actors.remove(&actor_id);
        }
    }

    pub fn followers_of(&self, leader_id: &str) -> Vec<String> {
        let leader_id = remap_story_actor_id(self, leader_id);
        self.relations
            .followed_actors
            .iter()
            .filter(|(_, target)| target.as_str() == leader_id)
            .map(|(follower, _)| follower.clone())
            .collect()
    }

    /// Sets an actor's stance and follower state, and initializes party order if Allied.
    pub fn set_actor_stance(
        &mut self,
        content: &ContentPack,
        actor_id: &str,
        stance: ActorStance,
        follows_player: bool,
    ) {
        let mut relationship = self.relationship(actor_id);
        relationship.stance = stance;
        relationship.follows_player = follows_player;
        self.set_relationship(actor_id, relationship);
        if stance == ActorStance::Allied {
            self.initialize_party_order(content, actor_id);
        } else if !follows_player {
            self.relations.party_orders.remove(actor_id);
        }
    }

    /// Sets the stance of all living non-player actors carrying `tag`.
    /// Returns the (id, name) of all actors whose stance was changed.
    pub fn set_actor_stance_by_tag(
        &mut self,
        content: &ContentPack,
        tag: &str,
        stance: ActorStance,
        from_stances: &[ActorStance],
        follows_player: bool,
    ) -> Vec<(String, String)> {
        let health_stat_id = &content.settings.combat.health_stat_id;
        let tagged_actors: Vec<(String, String)> = self
            .actors(content)
            .filter(|actor| actor.tags.iter().any(|actor_tag| actor_tag.as_str() == tag))
            .map(|actor| (actor.id.clone(), actor.name.clone()))
            .collect();

        let mut changed = Vec::new();
        for (actor_id, actor_name) in tagged_actors {
            if content.is_player_actor(&actor_id) {
                continue;
            }
            if self.actor_current_room_id(content, &actor_id).is_empty() {
                continue;
            }
            if self.actor_is_defeated(&actor_id, health_stat_id) {
                continue;
            }
            let relationship = self.relationship(&actor_id);
            if !from_stances.is_empty() && !from_stances.contains(&relationship.stance) {
                continue;
            }
            if relationship.stance == stance {
                continue;
            }
            self.set_actor_stance(content, &actor_id, stance, follows_player);
            changed.push((actor_id, actor_name));
        }
        changed
    }

    /// Defeats every living non-player actor carrying `tag`.
    /// Returns the IDs of the defeated actors.
    pub fn defeat_actors_by_tag(&mut self, content: &ContentPack, tag: &str) -> Vec<String> {
        let health_stat_id = &content.settings.combat.health_stat_id;
        let tagged_actor_ids: Vec<String> = self
            .actors(content)
            .filter(|actor| {
                !content.is_player_actor(&actor.id)
                    && actor.tags.iter().any(|actor_tag| actor_tag.as_str() == tag)
            })
            .map(|actor| actor.id.clone())
            .collect();

        for actor_id in &tagged_actor_ids {
            let _ = self.adjust_actor_stat(content, actor_id, health_stat_id, i32::MIN / 2);
        }
        tagged_actor_ids
    }
}
