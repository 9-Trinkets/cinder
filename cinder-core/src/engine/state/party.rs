use super::{ActorStance, WorldState, remap_story_actor_id};
use crate::content::types::{ContentPack, PartyOrderKind};

impl WorldState {
    pub fn party_order(&self, content: &ContentPack, actor_id: &str) -> Option<PartyOrderKind> {
        let actor_id = remap_story_actor_id(self, actor_id);
        self.party_orders
            .get(actor_id)
            .copied()
            .or_else(|| content.settings.party.initial_orders.get(actor_id).copied())
    }

    pub fn assign_party_order(
        &mut self,
        content: &ContentPack,
        actor_id: &str,
        order: PartyOrderKind,
    ) -> Result<(), String> {
        let actor_id = remap_story_actor_id(self, actor_id).to_string();
        validate_party_member(content, self, &actor_id)?;
        self.party_orders.insert(actor_id.clone(), order);
        self.set_follows_player(&actor_id, true);
        Ok(())
    }

    pub fn initialize_party_order(&mut self, content: &ContentPack, actor_id: &str) {
        let actor_id = remap_story_actor_id(self, actor_id).to_string();
        if self.party_orders.contains_key(&actor_id) {
            return;
        }
        if let Some(order) = content.settings.party.initial_orders.get(&actor_id) {
            self.party_orders.insert(actor_id, *order);
        }
    }
}

fn validate_party_member(
    content: &ContentPack,
    state: &WorldState,
    actor_id: &str,
) -> Result<(), String> {
    let actor = content
        .actor(actor_id)
        .ok_or_else(|| format!("unknown party actor '{actor_id}'"))?;
    if content.is_player_actor(actor_id) {
        return Err("the player cannot receive a party order".to_string());
    }
    if actor.is_offstage() {
        return Err(format!(
            "offstage actor '{actor_id}' cannot receive a party order"
        ));
    }
    if state.stance(actor_id) != ActorStance::Allied {
        return Err(format!("actor '{actor_id}' is not allied"));
    }
    if !state.actor_is_in_room(content, actor_id, &state.current_room_id) {
        return Err(format!("actor '{actor_id}' is not in the current room"));
    }
    if state.actor_is_defeated(actor_id, &content.settings.combat.health_stat_id) {
        return Err(format!("actor '{actor_id}' is defeated"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::test_fixtures::minimal_test_pack;
    use std::collections::BTreeMap;

    fn allied_state() -> (ContentPack, WorldState) {
        let mut content = minimal_test_pack();
        content.settings.party.initial_orders =
            BTreeMap::from([("blair".to_string(), PartyOrderKind::Guard)]);
        let mut state = WorldState::new(&content);
        state.set_stance("blair", ActorStance::Allied);
        state.set_follows_player("blair", true);
        (content, state)
    }

    #[test]
    fn content_initial_order_applies_until_overridden() {
        let (content, mut state) = allied_state();
        assert_eq!(
            state.party_order(&content, "blair"),
            Some(PartyOrderKind::Guard)
        );

        state
            .assign_party_order(&content, "blair", PartyOrderKind::Assist)
            .unwrap();
        assert_eq!(
            state.party_order(&content, "blair"),
            Some(PartyOrderKind::Assist)
        );
        assert!(state.follows_player("blair"));
    }

    #[test]
    fn orders_require_a_living_allied_member_in_the_current_room() {
        let (content, mut state) = allied_state();
        let error = state
            .assign_party_order(&content, "casey", PartyOrderKind::Guard)
            .unwrap_err();
        assert!(error.contains("not allied"), "{error}");

        state
            .actor_room_overrides
            .insert("blair".to_string(), "kitchen".to_string());
        let error = state
            .assign_party_order(&content, "blair", PartyOrderKind::Assist)
            .unwrap_err();
        assert!(error.contains("not in the current room"), "{error}");
    }

    #[test]
    fn old_saves_default_to_content_initial_orders() {
        let (content, state) = allied_state();
        let mut value = serde_json::to_value(&state).unwrap();
        value.as_object_mut().unwrap().remove("party_orders");
        let restored: WorldState = serde_json::from_value(value).unwrap();
        assert_eq!(
            restored.party_order(&content, "blair"),
            Some(PartyOrderKind::Guard)
        );
    }

    #[test]
    fn legacy_order_objects_restore_supported_directives() {
        let (content, state) = allied_state();
        let mut value = serde_json::to_value(&state).unwrap();
        value["party_orders"] = serde_json::json!({
            "blair": {
                "kind": "assist",
                "target": { "kind": "none" },
                "status": "active",
                "issued_at_minutes": 0,
                "updated_at_minutes": 0
            },
            "casey": {
                "kind": "follow",
                "target": { "kind": "none" },
                "status": "active",
                "issued_at_minutes": 0,
                "updated_at_minutes": 0
            }
        });

        let restored: WorldState = serde_json::from_value(value).unwrap();

        assert_eq!(
            restored.party_order(&content, "blair"),
            Some(PartyOrderKind::Assist)
        );
        assert!(!restored.party_orders.contains_key("casey"));
    }
}
