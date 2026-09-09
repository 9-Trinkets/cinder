use super::{ActorStance, WorldState, remap_story_actor_id};
use crate::content::types::{
    ContentPack, PartyOrder, PartyOrderKind, PartyOrderStatus, PartyOrderTarget,
};

impl PartyOrder {
    pub fn is_active(&self) -> bool {
        self.status == PartyOrderStatus::Active
    }
}

impl WorldState {
    pub fn party_order(&self, actor_id: &str) -> Option<&PartyOrder> {
        let actor_id = remap_story_actor_id(self, actor_id);
        self.party_orders.get(actor_id)
    }

    pub fn active_party_order(&self, actor_id: &str) -> Option<&PartyOrder> {
        self.party_order(actor_id).filter(|order| order.is_active())
    }

    pub fn assign_party_order(
        &mut self,
        content: &ContentPack,
        actor_id: &str,
        kind: PartyOrderKind,
        target: PartyOrderTarget,
    ) -> Result<(), String> {
        let actor_id = remap_story_actor_id(self, actor_id).to_string();
        validate_party_member(content, self, &actor_id)?;
        validate_order_target(content, &actor_id, kind, &target)?;
        let now = self.current_time_minutes;
        self.party_orders.insert(
            actor_id,
            PartyOrder {
                kind,
                target,
                status: PartyOrderStatus::Active,
                issued_at_minutes: now,
                updated_at_minutes: now,
                failure_code: None,
            },
        );
        Ok(())
    }

    pub fn complete_party_order(&mut self, actor_id: &str) -> Result<(), String> {
        self.transition_party_order(actor_id, PartyOrderStatus::Completed, None)
    }

    pub fn cancel_party_order(&mut self, actor_id: &str) -> Result<(), String> {
        self.transition_party_order(actor_id, PartyOrderStatus::Cancelled, None)
    }

    pub fn fail_party_order(&mut self, actor_id: &str, failure_code: &str) -> Result<(), String> {
        if failure_code.trim().is_empty() {
            return Err("party order failure code must not be empty".to_string());
        }
        self.transition_party_order(
            actor_id,
            PartyOrderStatus::Failed,
            Some(failure_code.to_string()),
        )
    }

    fn transition_party_order(
        &mut self,
        actor_id: &str,
        status: PartyOrderStatus,
        failure_code: Option<String>,
    ) -> Result<(), String> {
        let actor_id = remap_story_actor_id(self, actor_id).to_string();
        let order = self
            .party_orders
            .get_mut(&actor_id)
            .ok_or_else(|| format!("actor '{actor_id}' has no party order"))?;
        if !order.is_active() {
            return Err(format!("actor '{actor_id}' party order is not active"));
        }
        order.status = status;
        order.updated_at_minutes = self.current_time_minutes;
        order.failure_code = failure_code;
        Ok(())
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
    Ok(())
}

fn validate_order_target(
    content: &ContentPack,
    ordered_actor_id: &str,
    kind: PartyOrderKind,
    target: &PartyOrderTarget,
) -> Result<(), String> {
    match (kind, target) {
        (
            PartyOrderKind::Follow | PartyOrderKind::Guard | PartyOrderKind::Assist,
            PartyOrderTarget::None,
        ) => Ok(()),
        (PartyOrderKind::Hold | PartyOrderKind::Scout, PartyOrderTarget::Room { room_id }) => {
            content
                .room(room_id)
                .map(|_| ())
                .ok_or_else(|| format!("unknown party order room '{room_id}'"))
        }
        (PartyOrderKind::Hunt, PartyOrderTarget::Actor { actor_id }) => {
            if actor_id == ordered_actor_id {
                return Err("a party member cannot hunt itself".to_string());
            }
            let target = content
                .actor(actor_id)
                .ok_or_else(|| format!("unknown party order target actor '{actor_id}'"))?;
            if target.is_offstage() {
                return Err(format!(
                    "offstage actor '{actor_id}' cannot be a party order target"
                ));
            }
            Ok(())
        }
        _ => Err(format!("invalid target for {kind:?} party order")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::test_fixtures::minimal_test_pack;

    fn allied_state() -> (ContentPack, WorldState) {
        let content = minimal_test_pack();
        let mut state = WorldState::new(&content);
        state.set_stance("blair", ActorStance::Allied);
        state.set_follows_player("blair", true);
        (content, state)
    }

    #[test]
    fn assigns_and_transitions_party_orders() {
        let (content, mut state) = allied_state();
        state.current_time_minutes = 20;
        state
            .assign_party_order(
                &content,
                "blair",
                PartyOrderKind::Scout,
                PartyOrderTarget::Room {
                    room_id: "kitchen".to_string(),
                },
            )
            .unwrap();
        assert_eq!(
            state.active_party_order("blair").map(|order| order.kind),
            Some(PartyOrderKind::Scout)
        );
        assert_eq!(state.party_order("blair").unwrap().issued_at_minutes, 20);

        state.current_time_minutes = 30;
        state.complete_party_order("blair").unwrap();
        let order = state.party_order("blair").unwrap();
        assert_eq!(order.status, PartyOrderStatus::Completed);
        assert_eq!(order.updated_at_minutes, 30);
        assert!(state.active_party_order("blair").is_none());
    }

    #[test]
    fn reassignment_replaces_terminal_state_and_failure_code() {
        let (content, mut state) = allied_state();
        state
            .assign_party_order(
                &content,
                "blair",
                PartyOrderKind::Hold,
                PartyOrderTarget::Room {
                    room_id: "lounge".to_string(),
                },
            )
            .unwrap();
        state.fail_party_order("blair", "route_blocked").unwrap();

        state.current_time_minutes = 40;
        state
            .assign_party_order(
                &content,
                "blair",
                PartyOrderKind::Follow,
                PartyOrderTarget::None,
            )
            .unwrap();
        let order = state.party_order("blair").unwrap();
        assert_eq!(order.status, PartyOrderStatus::Active);
        assert_eq!(order.failure_code, None);
        assert_eq!(order.issued_at_minutes, 40);
    }

    #[test]
    fn cancellation_is_terminal_until_reassigned() {
        let (content, mut state) = allied_state();
        state
            .assign_party_order(
                &content,
                "blair",
                PartyOrderKind::Guard,
                PartyOrderTarget::None,
            )
            .unwrap();
        state.cancel_party_order("blair").unwrap();

        assert_eq!(
            state.party_order("blair").map(|order| order.status),
            Some(PartyOrderStatus::Cancelled)
        );
        let error = state.complete_party_order("blair").unwrap_err();
        assert!(error.contains("not active"), "{error}");
    }

    #[test]
    fn orders_validate_members_and_targets() {
        let (content, mut state) = allied_state();
        let error = state
            .assign_party_order(
                &content,
                "casey",
                PartyOrderKind::Follow,
                PartyOrderTarget::None,
            )
            .unwrap_err();
        assert!(error.contains("not allied"), "{error}");

        let error = state
            .assign_party_order(
                &content,
                "blair",
                PartyOrderKind::Scout,
                PartyOrderTarget::None,
            )
            .unwrap_err();
        assert!(error.contains("invalid target"), "{error}");
    }

    #[test]
    fn alliance_following_and_orders_remain_independent() {
        let (content, mut state) = allied_state();
        state
            .assign_party_order(
                &content,
                "blair",
                PartyOrderKind::Hold,
                PartyOrderTarget::Room {
                    room_id: "lounge".to_string(),
                },
            )
            .unwrap();
        state.set_follows_player("blair", false);

        assert_eq!(state.stance("blair"), ActorStance::Allied);
        assert!(!state.follows_player("blair"));
        assert_eq!(
            state.active_party_order("blair").map(|order| order.kind),
            Some(PartyOrderKind::Hold)
        );
    }

    #[test]
    fn old_saves_default_to_no_party_orders() {
        let (_, state) = allied_state();
        let mut value = serde_json::to_value(&state).unwrap();
        value.as_object_mut().unwrap().remove("party_orders");

        let restored: WorldState = serde_json::from_value(value).unwrap();
        assert!(restored.party_orders.is_empty());
    }
}
