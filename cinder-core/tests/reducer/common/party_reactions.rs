use super::*;
use cinder_core::content::types::{
    ContentPack, PackMessage, PartyCombatDecisionRule, PartyDecisionCondition, PartyDecisionTier,
    PartyOrderKind, PartyPolicyDefinition, PartyReactionAction, PartyReactionCooldown,
    PartyReactionWindow, PartyTargetSelection,
};
use cinder_core::engine::events::{TimestampedWorldEvent, WorldEvent};
use cinder_core::engine::reducer::{ReducerOutput, apply_events};
use cinder_core::engine::state::{ActorStance, WorldState};
use std::collections::BTreeMap;

pub const SECOND_ALLY_ID: &str = "drew";
pub const THIRD_ALLY_ID: &str = "evan";
pub const FOURTH_ALLY_ID: &str = "faye";
pub const FIFTH_ALLY_ID: &str = "gia";
pub const SIXTH_ALLY_ID: &str = "hank";

pub fn rule(
    id: &str,
    tier: PartyDecisionTier,
    action: PartyReactionAction,
    conditions: Vec<PartyDecisionCondition>,
    target: PartyTargetSelection,
) -> PartyCombatDecisionRule {
    PartyCombatDecisionRule {
        id: id.to_string(),
        tier,
        window: PartyReactionWindow::AfterHostileDamage,
        action,
        conditions,
        target,
        candidate_priority: vec![],
        support_effect: None,
        cooldown: PartyReactionCooldown::FixedMinutes { minutes: 5 },
        message: match action {
            PartyReactionAction::Counterattack => "combat.party_counterattack",
            PartyReactionAction::Support => "combat.party_support",
            PartyReactionAction::Hold => "combat.party_holds",
            PartyReactionAction::Intercept => "",
        }
        .to_string(),
    }
}

pub fn reaction_pack(
    initial_orders: BTreeMap<String, PartyOrderKind>,
    rules: Vec<PartyCombatDecisionRule>,
) -> ContentPack {
    let mut pack = reducer_test_pack();
    pack.settings.combat.player_actor_id = ACTOR_A_ID.to_string();
    pack.settings.combat.health_stat_id = "stamina".to_string();
    pack.settings.combat.attack_stat_id = "confidence".to_string();
    pack.settings.combat.defense_stat_id = "hunger".to_string();
    pack.settings.combat.minimum_damage = 1;
    pack.settings.party = PartyPolicyDefinition {
        initial_orders,
        combat_rules: rules,
    };
    pack.messages.insert(
        "combat.party_counterattack".to_string(),
        PackMessage::Narration(
            "{actor} hits {target} for {damage} {kind}. ({remaining} remaining)".to_string(),
        ),
    );
    pack.messages.insert(
        "combat.party_counterattack_group".to_string(),
        PackMessage::Narration(
            "{actors} hit {target} for {damage}. ({remaining} remaining)".to_string(),
        ),
    );
    pack.messages.insert(
        "combat.party_counterattack_large".to_string(),
        PackMessage::Narration(
            "{count} allies hit {target} for {damage}. ({remaining} remaining)".to_string(),
        ),
    );
    pack.messages.insert(
        "combat.party_support".to_string(),
        PackMessage::Narration(
            "{actor} supports {target} for {amount}. ({remaining} remaining)".to_string(),
        ),
    );
    pack.messages.insert(
        "combat.party_support_group".to_string(),
        PackMessage::Narration(
            "{actors} support {target} for {amount}. ({remaining} remaining)".to_string(),
        ),
    );
    pack.messages.insert(
        "combat.party_holds".to_string(),
        PackMessage::Narration("{actor} holds.".to_string()),
    );
    pack.messages.insert(
        "combat.party_holds_group".to_string(),
        PackMessage::Narration("{actors} hold.".to_string()),
    );
    pack.messages.insert(
        "combat.no_effect".to_string(),
        PackMessage::Narration("No effect on {actor} from {kind}.".to_string()),
    );
    for actor in &mut pack.actors {
        let (confidence, stamina, hunger) = match actor.id.as_str() {
            ACTOR_A_ID => (0, 20, 0),
            ACTOR_B_ID => (8, 10, 1),
            ACTOR_C_ID => (4, 20, 2),
            _ => (0, 5, 0),
        };
        actor.initial_stats = BTreeMap::from([
            ("confidence".to_string(), confidence),
            ("stamina".to_string(), stamina),
            ("hunger".to_string(), hunger),
        ]);
    }
    let ally = pack
        .actors
        .iter_mut()
        .find(|actor| actor.id == ACTOR_B_ID)
        .unwrap();
    ally.attack_kind = "fire".to_string();
    let hostile = pack
        .actors
        .iter_mut()
        .find(|actor| actor.id == ACTOR_C_ID)
        .unwrap();
    hostile.resistances = BTreeMap::from([("fire".to_string(), 3)]);
    rebuild_test_pack_indexes(&mut pack);
    pack
}

pub fn combat_state(pack: &ContentPack) -> WorldState {
    let mut state = WorldState::new(pack);
    state
        .actor_room_overrides
        .insert(ACTOR_C_ID.to_string(), LOUNGE_ID.to_string());
    state.set_stance(ACTOR_B_ID, ActorStance::Allied);
    state.set_stance(ACTOR_C_ID, ActorStance::Hostile);
    state
}

pub fn add_second_ally(pack: &mut ContentPack) {
    add_ally(pack, SECOND_ALLY_ID, "Drew", 7);
}

pub fn add_ally(pack: &mut ContentPack, id: &str, name: &str, confidence: i32) {
    let mut ally = pack.actor(ACTOR_B_ID).unwrap().clone();
    ally.id = id.to_string();
    ally.name = name.to_string();
    ally.attack_kind = "cold".to_string();
    ally.initial_stats
        .insert("confidence".to_string(), confidence);
    pack.actors.push(ally);
    rebuild_test_pack_indexes(pack);
}

pub fn hostile_strike(state: &mut WorldState, pack: &ContentPack) -> ReducerOutput {
    apply_events(
        state,
        pack,
        &[TimestampedWorldEvent::now(WorldEvent::HostileStrike {
            actor_id: ACTOR_C_ID.to_string(),
        })],
    )
}
