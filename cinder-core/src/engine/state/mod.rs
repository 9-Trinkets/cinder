use rand::Rng;

use crate::content::types::{
    ContentPack, OpeningMenuOptionDefinition, RoomDefinition, StatDefinition,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub mod variable_store;
pub use variable_store::{
    VariableDeclaration, VariableError, VariableScope, VariableStore, VariableType,
};

mod clock;
mod conversation;
mod inventory;
mod party;
mod relationships;
mod sequences;
mod stats;
mod tracking;

pub use sequences::ScriptedSequencePlayhead;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GamePhase {
    #[default]
    Active,
    ActEnded,
    GameEnded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldState {
    pub current_room_id: String,
    pub turn_number: u32,
    pub current_time_minutes: u32,
    #[serde(default)]
    pub phase: GamePhase,
    pub conversation_event_sequence: u64,
    pub conversation_memory: BTreeMap<String, Vec<ConversationMemoryLine>>,
    pub conversation_summaries: BTreeMap<String, ConversationSummaryState>,
    pub actor_stat_defs: BTreeMap<String, StatDefinition>,
    pub pair_stat_defs: BTreeMap<String, StatDefinition>,
    pub pair_stats: BTreeMap<String, BTreeMap<String, i32>>,
    pub actor_stats: BTreeMap<String, BTreeMap<String, i32>>,
    pub pending_replies: BTreeMap<String, PendingReplyState>,
    pub active_objective_stage_ids: Vec<String>,
    pub actor_room_overrides: BTreeMap<String, String>,
    #[serde(default)]
    pub stages_completed: usize,
    #[serde(default)]
    pub completed_stage_ids: BTreeSet<String>,
    pub feature_consumable_stock: BTreeMap<String, u32>,
    pub followed_actor_id: Option<String>,
    pub active_menu_id: Option<String>,
    #[serde(default)]
    pub pending_menu_selections: Vec<String>,
    #[serde(default)]
    pub generated_menu_options: HashMap<String, Vec<OpeningMenuOptionDefinition>>,
    pub pending_projector_sequence_id: Option<String>,
    pub pending_projector_narrative_lines: Vec<String>,
    pub story_vars: VariableStore,
    #[serde(default)]
    pub stage_started_minutes: BTreeMap<String, u32>,
    pub actor_known_room_ids: BTreeMap<String, BTreeSet<String>>,
    pub actor_observed_room_ids: BTreeMap<String, BTreeSet<String>>,
    pub actor_known_feature_ids: BTreeMap<String, BTreeSet<String>>,
    pub actor_known_actor_ids: BTreeMap<String, BTreeSet<String>>,
    pub actor_recent_observation_notes: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub initial_actor_stats: BTreeMap<String, BTreeMap<String, i32>>,
    #[serde(default)]
    pub initial_pair_stats: BTreeMap<String, BTreeMap<String, i32>>,
    #[serde(default, skip_serializing)]
    pub transcript: Vec<String>,
    #[serde(default)]
    pub last_transcript_line: Option<String>,
    #[serde(default)]
    pub player_inventory: HashMap<String, u32>,
    #[serde(default)]
    pub room_item_stock: BTreeMap<String, u32>,
    /// Remaining activations of charged room items, keyed like `room_item_stock`
    /// (`room::item`). Absent entries mean the item is at full capacity; the
    /// counter is cleared whenever a room stops holding the item.
    #[serde(default)]
    pub room_item_charges: BTreeMap<String, u32>,
    #[serde(default)]
    pub act_series: Option<ActSeriesState>,
    /// Per-actor relationship toward the player. Absent entries mean
    /// `ActorRelationship::default()` (neutral, not following), so packs that
    /// never touch relationships carry no state.
    #[serde(default)]
    pub relationships: BTreeMap<String, ActorRelationship>,
    /// Persistent Guard/Assist directive for each allied party member.
    #[serde(default, deserialize_with = "deserialize_party_orders")]
    pub party_orders: BTreeMap<String, crate::content::types::PartyOrderKind>,
    /// Earliest game minute at which each party member may react again.
    #[serde(default)]
    pub party_reaction_ready_at: BTreeMap<String, u32>,
    /// Next game-minute at which each hostile actor may autonomously strike
    /// the player. Only meaningful while the stance is hostile; entries are
    /// seeded when a mob wakes and cleared when it leaves hostility.
    #[serde(default)]
    pub next_hostile_strike_at: BTreeMap<String, u32>,
    /// Player equipment, slot id → item id. Slot ids are the pack's fixed
    /// `settings.equipment_slots` keys. Bonuses feed effective stat reads.
    #[serde(default)]
    pub equipment: BTreeMap<String, String>,
    /// Per-actor inventory (non-player actors), actor id → item id → count.
    #[serde(default)]
    pub actor_inventories: BTreeMap<String, HashMap<String, u32>>,
    /// Per-actor equipment (non-player actors), actor id → slot id → item id.
    #[serde(default)]
    pub actor_equipment: BTreeMap<String, BTreeMap<String, String>>,
    /// Per-actor experience toward their next level. Absent entries are 0.
    /// When a mob is defeated its full XP drop is awarded to every party
    /// member (the player and every follower), so each advances on their own
    /// curve.
    #[serde(default)]
    pub actor_xp: BTreeMap<String, u32>,
    /// Per-actor current level. Absent entries read as 1.
    #[serde(default)]
    pub actor_level: BTreeMap<String, u32>,
    /// Playheads of content-declared scripted conversation sequences, keyed by
    /// sequence id. Every pack sequence is present at session creation; the
    /// opening sequence starts running.
    #[serde(default)]
    pub scripted_sequences: BTreeMap<String, ScriptedSequencePlayhead>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum SavedPartyOrder {
    Current(crate::content::types::PartyOrderKind),
    Legacy { kind: String },
}

fn deserialize_party_orders<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<String, crate::content::types::PartyOrderKind>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let saved = BTreeMap::<String, SavedPartyOrder>::deserialize(deserializer)?;
    Ok(saved
        .into_iter()
        .filter_map(|(actor_id, order)| {
            let order = match order {
                SavedPartyOrder::Current(order) => Some(order),
                SavedPartyOrder::Legacy { kind } if kind.is_empty() => None,
                SavedPartyOrder::Legacy { kind } => Some(kind),
            };
            order.map(|order| (actor_id, order))
        })
        .collect())
}

/// Discrete stance of an actor toward the player. Mutual exclusion is inherent:
/// a stance is a single value, not independent flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorStance {
    #[default]
    Neutral,
    Hostile,
    Allied,
}

/// Relationship of one actor to the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ActorRelationship {
    #[serde(default)]
    pub stance: ActorStance,
    #[serde(default)]
    pub follows_player: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMemoryLine {
    pub turn_number: u32,
    #[serde(default)]
    pub event_sequence: u64,
    pub speaker_id: String,
    pub speaker_name: String,
    #[serde(default)]
    pub kind: ConversationMemoryKind,
    #[serde(default)]
    pub target_label: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConversationSummaryState {
    pub summary: Option<String>,
    pub pending_line_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PendingReplyState {
    pub speaker_id: String,
    pub listener_id: String,
    pub room_id: String,
    pub turn_number: u32,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConversationMemoryKind {
    #[default]
    Speech,
    Action,
}

impl WorldState {
    pub fn new(content: &ContentPack) -> Self {
        let start_room_id = if content.opening.start_room_ids.is_empty() {
            content.opening.start_room_id.clone()
        } else {
            let ids = &content.opening.start_room_ids;
            let index = rand::thread_rng().gen_range(0..ids.len());
            ids[index].clone()
        };
        let mut actor_known_room_ids = content
            .onstage_actors()
            .map(|actor| (actor.id.clone(), BTreeSet::from([actor.room_id.clone()])))
            .collect::<BTreeMap<_, _>>();
        actor_known_room_ids.insert(
            content.settings.combat.player_actor_id.clone(),
            BTreeSet::from([start_room_id.clone()]),
        );
        let mut scripted_sequences = content
            .sequences
            .sequences
            .iter()
            .map(|sequence| (sequence.id.clone(), ScriptedSequencePlayhead::default()))
            .collect::<BTreeMap<_, _>>();
        if let Some(opening_sequence_id) = content.opening.opening_sequence_id.as_deref()
            && let Some(playhead) = scripted_sequences.get_mut(opening_sequence_id)
        {
            *playhead = ScriptedSequencePlayhead::queued();
        }
        Self {
            current_room_id: start_room_id,
            turn_number: 0,
            current_time_minutes: content.opening.start_time_minutes,
            phase: GamePhase::Active,
            conversation_event_sequence: 0,
            conversation_memory: BTreeMap::new(),
            conversation_summaries: BTreeMap::new(),
            actor_stat_defs: content.stats.actor.clone(),
            pair_stat_defs: content.stats.pair.clone(),
            pair_stats: seeded_pair_stats(content, &content.stats.pair),
            actor_stats: seeded_actor_stats(content, &content.stats.actor),
            pending_replies: BTreeMap::new(),
            active_objective_stage_ids: content.beats.initial_stage_ids.clone(),
            actor_room_overrides: BTreeMap::new(),
            stages_completed: 0,
            completed_stage_ids: BTreeSet::new(),
            feature_consumable_stock: seeded_feature_consumable_stock(content),
            followed_actor_id: None,
            active_menu_id: None,
            pending_menu_selections: Vec::new(),
            generated_menu_options: HashMap::new(),
            pending_projector_sequence_id: None,
            pending_projector_narrative_lines: Vec::new(),
            story_vars: VariableStore::new(content.variables.clone()),
            stage_started_minutes: content
                .beats
                .initial_stage_ids
                .iter()
                .map(|stage_id| (stage_id.clone(), content.opening.start_time_minutes))
                .collect(),
            actor_known_room_ids,
            actor_observed_room_ids: BTreeMap::new(),
            actor_known_feature_ids: BTreeMap::new(),
            actor_known_actor_ids: BTreeMap::new(),
            actor_recent_observation_notes: BTreeMap::new(),
            initial_actor_stats: seeded_actor_stats(content, &content.stats.actor),
            initial_pair_stats: seeded_pair_stats(content, &content.stats.pair),
            transcript: Vec::new(),
            last_transcript_line: None,
            player_inventory: content
                .actor(&content.settings.combat.player_actor_id)
                .map(|actor| actor.initial_inventory.clone().into_iter().collect())
                .unwrap_or_default(),
            room_item_stock: BTreeMap::new(),
            room_item_charges: BTreeMap::new(),
            act_series: None,
            relationships: content
                .actors
                .iter()
                .filter_map(|actor| {
                    actor
                        .initial_relationship
                        .map(|relationship| (actor.id.clone(), relationship))
                })
                .chain(
                    content
                        .actors
                        .iter()
                        .filter(|actor| {
                            actor.initial_hostile && actor.initial_relationship.is_none()
                        })
                        .map(|actor| {
                            (
                                actor.id.clone(),
                                ActorRelationship {
                                    stance: ActorStance::Hostile,
                                    follows_player: false,
                                },
                            )
                        }),
                )
                .collect(),
            party_orders: BTreeMap::new(),
            party_reaction_ready_at: BTreeMap::new(),
            next_hostile_strike_at: BTreeMap::new(),
            equipment: content
                .actor(&content.settings.combat.player_actor_id)
                .map(|actor| actor.initial_equipment.clone())
                .unwrap_or_default(),
            actor_inventories: seeded_actor_inventories(content),
            actor_equipment: seeded_actor_equipment(content),
            actor_xp: BTreeMap::new(),
            actor_level: seeded_actor_levels(content),
            scripted_sequences,
        }
    }

    pub fn current_room<'a>(&self, content: &'a ContentPack) -> Option<&'a RoomDefinition> {
        content.room(&self.current_room_id)
    }
}

const MINUTES_PER_DAY: u32 = 24 * 60;

fn feature_key(room_id: &str, feature_id: &str) -> String {
    format!("{room_id}::{feature_id}")
}

fn consumable_key(room_id: &str, feature_id: &str, consumable_id: &str) -> String {
    format!("{room_id}::{feature_id}::{consumable_id}")
}

fn room_item_key(room_id: &str, item_id: &str) -> String {
    format!("{room_id}::{item_id}")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnOutcome {
    pub text: String,
    pub phase: GamePhase,
    /// The narrative lines that produced `text`, each tagged with how it
    /// should be styled. Present so the server/client never have to parse
    /// styling hints out of the prose.
    #[serde(default)]
    pub lines: Vec<crate::engine::narrative::NarrativeLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSnapshot {
    pub turn_number: u32,
    pub current_room_id: String,
}

mod seeding;
use seeding::{
    seeded_actor_equipment, seeded_actor_inventories, seeded_actor_levels, seeded_actor_stats,
    seeded_feature_consumable_stock, seeded_pair_stats,
};
mod act_cast;
pub use act_cast::{
    ActFeedbackSummary, ActHistoryEntry, ActSeriesState, CastMemberRecord, advance_to_next_act,
    current_act_intro, current_cast_member_actor_id, current_cast_member_name, display_actor_name,
    initialize_act_state, remap_story_actor_id, render_dynamic_story_text,
    resolved_actor_prompt_context, story_actor_matches,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offstage_actor_has_no_seeded_home_room_but_keeps_identity() {
        let mut content = crate::engine::test_fixtures::minimal_test_pack();
        content
            .actors
            .iter_mut()
            .find(|actor| actor.id == "blair")
            .unwrap()
            .room_id
            .clear();
        let state = WorldState::new(&content);

        assert!(content.actor("blair").unwrap().is_offstage());
        assert!(
            !state.actor_known_room_ids.contains_key("blair"),
            "offstage actor must not be seeded a (nonexistent) home room"
        );
        assert!(
            !state.actor_stats_snapshot("blair").is_empty(),
            "offstage actor must still carry its authored identity and stats"
        );
    }

    #[test]
    fn authored_initial_relationship_seeds_allied_over_shorthand_hostile() {
        let mut content = crate::engine::test_fixtures::minimal_test_pack();
        let blair = content
            .actors
            .iter_mut()
            .find(|actor| actor.id == "blair")
            .unwrap();
        blair.initial_hostile = true;
        blair.initial_relationship = Some(ActorRelationship {
            stance: ActorStance::Allied,
            follows_player: false,
        });
        let state = WorldState::new(&content);

        let relationship = state.relationship("blair");
        assert_eq!(relationship.stance, ActorStance::Allied);
        assert!(!relationship.follows_player);
    }
}
