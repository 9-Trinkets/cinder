use super::{ActorStance, WorldState};
use crate::content::types::{ContentPack, PartyController, PartyOrderKind};
use crate::engine::orders::{OrderKind, parse_order};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Party id used for the player's command group when a pack authors none.
pub const PLAYER_PARTY_ID: &str = "player";

/// A runtime command group. The player party's roster is assembled from allies
/// at runtime; AI parties carry content-authored members.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Party {
    pub id: String,
    pub controller: PartyController,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leader: Option<String>,
    #[serde(default)]
    pub members: BTreeSet<String>,
}

impl Party {
    pub fn is_player_controlled(&self) -> bool {
        self.controller == PartyController::Player
    }
}

/// Seeds the party registry from content: authored members for every party,
/// plus every initially-allied or following actor in the player party.
pub(crate) fn seeded_parties(content: &ContentPack) -> BTreeMap<String, Party> {
    let mut parties = BTreeMap::new();
    for definition in &content.settings.parties {
        let id = if definition.id.is_empty() {
            PLAYER_PARTY_ID.to_string()
        } else {
            definition.id.clone()
        };
        parties.insert(
            id.clone(),
            Party {
                id,
                controller: definition.controller,
                leader: definition.leader.clone(),
                members: definition.members.iter().cloned().collect(),
            },
        );
    }
    if !parties.values().any(Party::is_player_controlled) {
        parties.insert(
            PLAYER_PARTY_ID.to_string(),
            Party {
                id: PLAYER_PARTY_ID.to_string(),
                controller: PartyController::Player,
                leader: None,
                members: BTreeSet::new(),
            },
        );
    }
    let player_actor_id = content.settings.combat.player_actor_id.as_str();
    for actor in &content.actors {
        if actor.id == player_actor_id {
            continue;
        }
        let relationship = actor.initial_relationship.unwrap_or_default();
        if relationship.stance == ActorStance::Allied || relationship.follows_player {
            for party in parties
                .values_mut()
                .filter(|party| party.is_player_controlled())
            {
                party.members.insert(actor.id.clone());
            }
        }
    }
    parties
}

impl WorldState {
    /// Returns the party that contains `actor_id`, if any. When an actor is both
    /// an authored member of an AI party and an allied follower gathered into
    /// the player party, the player party wins so reactions and orders treat it
    /// as an ally.
    pub fn party_of_actor(&self, actor_id: &str) -> Option<&Party> {
        self.parties
            .values()
            .find(|party| party.is_player_controlled() && party.members.contains(actor_id))
            .or_else(|| {
                self.parties
                    .values()
                    .find(|party| party.members.contains(actor_id))
            })
    }

    /// The player-controlled party, if the registry has one.
    pub fn player_party(&self) -> Option<&Party> {
        self.parties
            .values()
            .find(|party| party.is_player_controlled())
    }

    /// Whether two actors belong to the same command group.
    pub fn share_party(&self, left: &str, right: &str) -> bool {
        let left = left;
        let right = right;
        self.parties
            .values()
            .any(|party| party.members.contains(left) && party.members.contains(right))
    }

    /// Sets a content-authored order for any actor, bypassing the allied-only
    /// checks so a pack can drive an AI party (`controller: ai`) reactively.
    pub fn set_authored_party_order(&mut self, actor_id: &str, order: PartyOrderKind) {
        let actor_id = actor_id.to_string();
        self.relations.party_orders.insert(actor_id, order);
    }

    /// Sets a content-authored order for every onstage living actor carrying
    /// `tag`. Returns the ids ordered. Lets a pack move a whole role group.
    pub fn set_party_order_by_tag(
        &mut self,
        content: &ContentPack,
        tag: &str,
        order: &str,
    ) -> Vec<String> {
        let health_stat_id = &content.settings.combat.health_stat_id;
        let ids: Vec<String> = self
            .actors(content)
            .filter(|actor| !content.is_player_actor(&actor.id))
            .filter(|actor| actor.tags.iter().any(|actor_tag| actor_tag.as_str() == tag))
            .map(|actor| actor.id.clone())
            .collect();
        let mut changed = Vec::new();
        for actor_id in ids {
            if self.actor_current_room_id(content, &actor_id).is_empty() {
                continue;
            }
            if self.actor_is_defeated(&actor_id, health_stat_id) {
                continue;
            }
            self.set_authored_party_order(&actor_id, order.to_string());
            changed.push(actor_id);
        }
        changed
    }

    /// Adds or removes an actor from the player party, creating the player
    /// party on demand so deserialized states self-heal. `is_member` is the
    /// authoritative allied-or-following signal.
    pub fn sync_player_party_membership(&mut self, actor_id: &str, is_member: bool) {
        let actor_id = actor_id.to_string();
        if is_member && !self.parties.values().any(Party::is_player_controlled) {
            self.parties.insert(
                PLAYER_PARTY_ID.to_string(),
                Party {
                    id: PLAYER_PARTY_ID.to_string(),
                    controller: PartyController::Player,
                    leader: None,
                    members: BTreeSet::new(),
                },
            );
        }
        for party in self
            .parties
            .values_mut()
            .filter(|party| party.is_player_controlled())
        {
            if is_member {
                party.members.insert(actor_id.clone());
            } else {
                party.members.remove(&actor_id);
            }
        }
    }

    pub fn is_party_member(&self, content: &ContentPack, actor_id: &str) -> bool {
        if content.is_player_actor(actor_id) {
            return false;
        }
        if self.actor_is_offstage(content, actor_id) {
            return false;
        }
        if self.actor_is_defeated(actor_id, &content.settings.combat.health_stat_id) {
            return false;
        }
        self.parties
            .values()
            .any(|party| party.is_player_controlled() && party.members.contains(actor_id))
    }

    pub fn party_order(&self, content: &ContentPack, actor_id: &str) -> Option<PartyOrderKind> {
        if let Some(order) = self.relations.party_orders.get(actor_id) {
            return Some(order.clone());
        }
        if let Some(order) = authored_party_order(content, actor_id) {
            return Some(order);
        }
        if self.stance(actor_id) == ActorStance::Allied && !content.is_player_actor(actor_id) {
            Some("follow".to_string())
        } else {
            None
        }
    }

    pub fn assign_party_order(
        &mut self,
        content: &ContentPack,
        actor_id: &str,
        order: PartyOrderKind,
    ) -> Result<(), String> {
        let actor_id = actor_id.to_string();
        validate_party_member(content, self, &actor_id)?;
        let holds_position = matches!(
            parse_order(&order),
            Some(OrderKind::Hold | OrderKind::Patrol | OrderKind::Deploy(_))
        );
        self.relations.party_orders.insert(actor_id.clone(), order);
        self.set_follows_player(&actor_id, !holds_position);
        self.set_followed_actor(&actor_id, None);
        Ok(())
    }

    pub fn initialize_party_order(&mut self, content: &ContentPack, actor_id: &str) {
        let actor_id = actor_id.to_string();
        if self.relations.party_orders.contains_key(&actor_id) {
            return;
        }
        let order =
            authored_party_order(content, &actor_id).unwrap_or_else(|| "follow".to_string());
        let holds_position = matches!(
            parse_order(&order),
            Some(OrderKind::Hold | OrderKind::Patrol | OrderKind::Deploy(_))
        );
        if self.stance(&actor_id) == ActorStance::Allied {
            self.set_follows_player(&actor_id, !holds_position);
        }
        self.relations.party_orders.insert(actor_id, order);
    }
}

/// The authored standing order for `actor_id`: exact id first, then the
/// template prefix before the first `-` (so `elf-pawn-1` matches `elf-pawn`).
fn authored_party_order(content: &ContentPack, actor_id: &str) -> Option<PartyOrderKind> {
    let template_id = actor_id
        .split_once('-')
        .map(|(template, _)| template)
        .unwrap_or(actor_id);
    for party in &content.settings.parties {
        if let Some(order) = party.initial_orders.get(actor_id) {
            return Some(order.clone());
        }
        if let Some(order) = party.initial_orders.get(template_id) {
            return Some(order.clone());
        }
    }
    None
}

fn validate_party_member(
    content: &ContentPack,
    state: &WorldState,
    actor_id: &str,
) -> Result<(), String> {
    let actor = state
        .actor(content, actor_id)
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
    if state.actor_is_defeated(actor_id, &content.settings.combat.health_stat_id) {
        return Err(format!("actor '{actor_id}' is defeated"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::types::PartyDefinition;
    use crate::engine::test_fixtures::minimal_test_pack;
    use std::collections::BTreeMap;

    fn allied_state() -> (ContentPack, WorldState) {
        let mut content = minimal_test_pack();
        content.settings.parties = vec![PartyDefinition {
            id: "player".to_string(),
            initial_orders: BTreeMap::from([("blair".to_string(), "guard".to_string())]),
            ..PartyDefinition::default()
        }];
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
            Some("guard".to_string())
        );

        state
            .assign_party_order(&content, "blair", "follow".to_string())
            .unwrap();
        assert_eq!(
            state.party_order(&content, "blair"),
            Some("follow".to_string())
        );
        assert!(state.follows_player("blair"));
    }

    #[test]
    fn orders_require_a_living_allied_member_and_allow_distant_members() {
        let (content, mut state) = allied_state();
        let error = state
            .assign_party_order(&content, "casey", "guard".to_string())
            .unwrap_err();
        assert!(error.contains("not allied"), "{error}");

        state
            .actors
            .actor_room_overrides
            .insert("blair".to_string(), "kitchen".to_string());
        assert!(
            state
                .assign_party_order(&content, "blair", "follow".to_string())
                .is_ok()
        );
    }

    #[test]
    fn old_saves_default_to_content_initial_orders() {
        let (content, state) = allied_state();
        let mut value = serde_json::to_value(&state).unwrap();
        value.as_object_mut().unwrap().remove("party_orders");
        let restored: WorldState = serde_json::from_value(value).unwrap();
        assert_eq!(
            restored.party_order(&content, "blair"),
            Some("guard".to_string())
        );
    }

    #[test]
    fn unauthored_allied_mobs_default_to_follow_order() {
        let content = minimal_test_pack();
        let mut state = WorldState::new(&content);
        state.set_stance("blair", ActorStance::Allied);

        // party_order queries on uninitialized allied mob default to follow
        assert_eq!(
            state.party_order(&content, "blair"),
            Some("follow".to_string())
        );

        // initialize_party_order seeds follow into state
        state.initialize_party_order(&content, "blair");
        assert_eq!(
            state.relations.party_orders.get("blair"),
            Some(&"follow".to_string())
        );
        assert!(state.follows_player("blair"));
    }

    #[test]
    fn is_party_member_identifies_allies_and_followers_and_filters_ineligible() {
        let mut content = minimal_test_pack();
        content.settings.combat.health_stat_id = "stamina".to_string();
        let mut state = WorldState::new(&content);

        // Player is never a party member
        assert!(!state.is_party_member(&content, "player"));

        // Neutral bystander is not a party member
        assert!(!state.is_party_member(&content, "blair"));

        // Allied actor is a party member
        state.set_stance("blair", ActorStance::Allied);
        assert!(state.is_party_member(&content, "blair"));

        // Defeated allied actor is not a party member
        state
            .adjust_actor_stat(
                &content,
                "blair",
                &content.settings.combat.health_stat_id,
                -100,
            )
            .unwrap();
        assert!(!state.is_party_member(&content, "blair"));

        // Follower actor is a party member
        state.set_follows_player("casey", true);
        assert!(state.is_party_member(&content, "casey"));

        // Offstage follower is not a party member
        content.actors[1].room_id.clear();
        assert!(!state.is_party_member(&content, "casey"));
    }
}
