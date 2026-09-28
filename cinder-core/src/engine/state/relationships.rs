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
            self.seed_allied_drops_to_inventory(content, actor_id);
        }
    }

    /// When an actor becomes allied (e.g. charmed), ensures that any drops defined on
    /// the actor are also present in their inventory so the player can take them via party commands.
    pub fn seed_allied_drops_to_inventory(&mut self, content: &ContentPack, actor_id: &str) {
        let Some(actor) = content.actor(actor_id) else {
            return;
        };
        for (item_id, spec) in &actor.drops {
            let drop_count = resolve_drop_spec_count(self, content, spec);
            let current_count = self.actor_item_count(actor_id, item_id);
            if drop_count > current_count {
                for _ in 0..(drop_count - current_count) {
                    self.actor_add_item(actor_id, item_id);
                }
            }
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

fn resolve_drop_spec_count(
    state: &WorldState,
    content: &ContentPack,
    spec: &crate::content::types::DropSpec,
) -> u32 {
    match spec {
        crate::content::types::DropSpec::Always(count) => *count,
        crate::content::types::DropSpec::Conditional(cond) => {
            if !cond.skip_when_story_var.is_empty()
                && content.story_var_is_truthy(state, &cond.skip_when_story_var)
            {
                0
            } else {
                cond.count
            }
        }
        _ => 0,
    }
}
