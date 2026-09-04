use super::*;

impl WorldState {
    /// Relationship of an actor toward the player; absent entries are neutral.
    pub fn relationship(&self, actor_id: &str) -> ActorRelationship {
        let actor_id = remap_story_actor_id(self, actor_id);
        self.relationships
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
            self.relationships.remove(actor_id.as_str());
        } else {
            self.relationships.insert(actor_id.clone(), relationship);
        }
        if relationship.stance != ActorStance::Hostile {
            self.next_hostile_strike_at.remove(actor_id.as_str());
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
}
