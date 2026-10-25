use rand::Rng;

use crate::content::types::{
    ActorDefinition, ContentPack, OpeningMenuOptionDefinition, RoomDefinition, SkillDefinition,
    SkillKind,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub mod variable_store;
pub use variable_store::{
    VariableDeclaration, VariableError, VariableScope, VariableStore, VariableType,
};

mod anchors;
mod clock;
mod conversation;
mod features;
mod inventory;
mod party;
mod relationships;
mod sequences;
mod spawn;
mod stats;
mod tracking;

use party::seeded_parties;

pub use clock::Clock;
pub use inventory::Inventory;
pub use party::{PLAYER_PARTY_ID, Party};
pub use relationships::RelationshipGraph;
pub use sequences::ScriptedSequencePlayhead;
pub use spawn::{SpawnActorConfig, SpawnActorOutcome, SpawnedActorInfo};
pub use stats::CombatState;
pub use tracking::ActorRegistry;

/// Current persisted `WorldState` format version. Version 1 predates the
/// version field and used floor-conquest milestone naming (`floor_summaries`,
/// `FloorDescentSummary`, `party_at_descent`); version 2 renamed those to
/// area transitions (`transition_summaries`, `TransitionSummary`,
/// `party_at_transition`).
pub const WORLD_STATE_VERSION: u32 = 2;

fn default_state_version() -> u32 {
    WORLD_STATE_VERSION
}

/// Legacy player id used by saves written before the id was recorded in
/// state. All historical packs used the `"player"` actor id.
fn default_player_actor_id() -> String {
    "player".to_string()
}

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
    /// Format version of this persisted state.
    #[serde(default = "default_state_version")]
    pub state_version: u32,
    pub current_room_id: String,
    /// The id of the player-controlled actor, resolved from
    /// `settings.combat.player_actor_id` at construction. Runtime code must
    /// compare identities against this rather than a literal `"player"` so a
    /// pack (e.g. a test fixture using `"blair"`) may name its player anything.
    #[serde(default = "default_player_actor_id")]
    pub player_actor_id: String,
    #[serde(flatten)]
    pub clock: Clock,
    #[serde(default)]
    pub phase: GamePhase,
    pub conversation_event_sequence: u64,
    pub conversation_memory: BTreeMap<String, Vec<ConversationMemoryLine>>,
    pub conversation_summaries: BTreeMap<String, ConversationSummaryState>,
    #[serde(flatten)]
    pub combat: CombatState,
    pub pending_replies: BTreeMap<String, PendingReplyState>,
    pub active_objective_stage_ids: Vec<String>,
    #[serde(flatten)]
    pub actors: ActorRegistry,
    #[serde(default)]
    pub stages_completed: usize,
    #[serde(default)]
    pub completed_stage_ids: BTreeSet<String>,
    /// States of room features that have them, keyed `room::feature`.
    #[serde(default)]
    pub feature_states: BTreeMap<String, String>,
    /// Items held by room features, keyed `room::feature::item`.
    #[serde(default, alias = "feature_consumable_stock")]
    pub feature_item_stock: BTreeMap<String, u32>,
    #[serde(flatten)]
    pub relations: RelationshipGraph,
    /// Faction-agnostic command groups keyed by party id. The player party
    /// gathers allies dynamically; AI parties carry authored members.
    #[serde(default)]
    pub parties: BTreeMap<String, Party>,
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
    #[serde(default)]
    pub player_recent_room_ids: Vec<String>,
    #[serde(default, skip_serializing)]
    pub transcript: Vec<String>,
    #[serde(default)]
    pub last_transcript_line: Option<String>,
    #[serde(flatten)]
    pub inventory: Inventory,
    /// Playheads of content-declared scripted conversation sequences, keyed by
    /// sequence id. Every pack sequence is present at session creation; the
    /// opening sequence starts running.
    #[serde(default)]
    pub scripted_sequences: BTreeMap<String, ScriptedSequencePlayhead>,
    /// Summaries of completed areas / acts upon transition between areas,
    /// keyed by area / act id.
    #[serde(default)]
    pub transition_summaries: BTreeMap<String, TransitionSummary>,
    /// Counter for dynamically spawned actors to ensure unique instance IDs.
    #[serde(default)]
    pub spawn_counter: u32,
    /// Active temporary chalk anchors placed by tracing teleport-sigil (up to 3).
    /// Ordered from oldest to newest.
    #[serde(default)]
    pub chalk_anchors: Vec<String>,
    /// Runtime state for content-authored room protection rules, keyed by stage id.
    #[serde(default)]
    pub protection_rule_states: BTreeMap<String, ProtectionRuleState>,
    /// Runtime tracking for autonomous remote comms dispatches during offscreen party combat.
    #[serde(default)]
    pub offscreen_combat_states: BTreeMap<String, OffscreenCombatState>,
    /// Tracks periodic effect ambient cues emitted in the current turn to prevent
    /// duplicate audio/visual lines when multiple actors trigger in the same room.
    /// Key: `{room_id}::{effect_id}` or `{room_id}::{effect_id}::boss`, Value: turn_number.
    #[serde(default)]
    pub periodic_effects_reported: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct OffscreenCombatState {
    pub engaged_at_turn: u32,
    pub last_dispatch_turn: u32,
    pub contact_dispatched: bool,
    pub low_health_dispatched: bool,
    #[serde(default)]
    pub reported_fallen_allies: BTreeSet<String>,
    #[serde(default)]
    pub newly_fallen_allies: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProtectionRuleState {
    pub next_actor_index: usize,
    pub at_risk_actor_id: Option<String>,
    pub breach_started_minutes: Option<u32>,
}

/// Summary milestone recorded upon transitioning from one area / act to the
/// next.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransitionSummary {
    pub area_id: String,
    pub area_name: String,
    pub completed_turn: u32,
    pub summary_text: String,
    #[serde(default)]
    pub party_at_transition: Vec<String>,
    #[serde(default)]
    pub transcript_line_count: usize,
}

/// Discrete stance of an actor toward the player's faction. Mutual exclusion
/// is inherent: a stance is a single value, not independent flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorStance {
    #[default]
    Neutral,
    Hostile,
    Allied,
}

/// Relationship of one actor to the player's faction.
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

/// The menu an initial stage opens at the start of play, if any. Stages that
/// are entered by advancing open their menu through the reducer; initial
/// stages never advance into themselves, so their menu is opened here.
fn initial_stage_menu_id(content: &ContentPack) -> Option<String> {
    content.beats.initial_stage_ids.iter().find_map(|stage_id| {
        content
            .beats
            .stages
            .iter()
            .find(|stage| stage.id == *stage_id)
            .map(|stage| stage.open_menu.as_str())
            .filter(|menu_id| !menu_id.is_empty())
            .map(str::to_string)
    })
}

impl WorldState {
    /// Deserializes a persisted state JSON blob.
    pub fn from_saved_json(raw: &str) -> Result<Self, String> {
        serde_json::from_str(raw).map_err(|e| format!("failed to deserialize state: {e}"))
    }

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
            state_version: WORLD_STATE_VERSION,
            current_room_id: start_room_id.clone(),
            player_actor_id: content.settings.combat.player_actor_id.clone(),
            clock: Clock {
                turn_number: 0,
                current_time_minutes: content.opening.start_time_minutes,
            },
            phase: GamePhase::Active,
            conversation_event_sequence: 0,
            conversation_memory: BTreeMap::new(),
            conversation_summaries: BTreeMap::new(),
            combat: CombatState {
                actor_stat_defs: content.stats.actor.clone(),
                pair_stat_defs: content.stats.pair.clone(),
                pair_stats: seeded_pair_stats(content, &content.stats.pair),
                actor_stats: seeded_actor_stats(content, &content.stats.actor),
                initial_actor_stats: seeded_actor_stats(content, &content.stats.actor),
                initial_pair_stats: seeded_pair_stats(content, &content.stats.pair),
                actor_xp: BTreeMap::new(),
                actor_level: seeded_actor_levels(content),
                actor_skills: seeded_actor_skills(content),
                actor_skill_levels: seeded_actor_skill_levels(content),
                actor_skill_sp: seeded_actor_skill_sp(content, &seeded_actor_skill_levels(content)),
                next_hostile_strike_at: BTreeMap::new(),
                invisible_until_minute: None,
            },
            pending_replies: BTreeMap::new(),
            active_objective_stage_ids: content.beats.initial_stage_ids.clone(),
            actors: ActorRegistry {
                actor_room_overrides: BTreeMap::new(),
                actor_known_room_ids,
                actor_observed_room_ids: BTreeMap::new(),
                actor_known_feature_ids: BTreeMap::new(),
                actor_known_actor_ids: BTreeMap::new(),
                actor_recent_observation_notes: BTreeMap::new(),
                spawned_actors: BTreeMap::new(),
                actor_name_overrides: BTreeMap::new(),
                transformed_stages: BTreeMap::new(),
            },
            stages_completed: 0,
            completed_stage_ids: BTreeSet::new(),
            feature_states: BTreeMap::new(),
            feature_item_stock: seeded_feature_item_stock(content),
            relations: RelationshipGraph {
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
                followed_actors: BTreeMap::new(),
                followed_actor_id: None,
                party_reaction_ready_at: BTreeMap::new(),
            },
            parties: seeded_parties(content),
            active_menu_id: initial_stage_menu_id(content),
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
            player_recent_room_ids: vec![start_room_id.clone()],
            transcript: Vec::new(),
            last_transcript_line: None,
            inventory: Inventory {
                player_inventory: content
                    .actor(&content.settings.combat.player_actor_id)
                    .map(|actor| seeding::resolve_inventory_specs(&actor.initial_inventory))
                    .unwrap_or_default(),
                room_item_stock: seeded_room_item_stock(content),
                room_item_charges: BTreeMap::new(),
                actor_inventories: seeded_actor_inventories(content),
                actor_equipment: seeded_actor_equipment(content),
                equipment: content
                    .actor(&content.settings.combat.player_actor_id)
                    .map(|actor| actor.initial_equipment.clone())
                    .unwrap_or_default(),
            },
            scripted_sequences,
            transition_summaries: BTreeMap::new(),
            spawn_counter: 0,
            chalk_anchors: Vec::new(),
            protection_rule_states: BTreeMap::new(),
            offscreen_combat_states: BTreeMap::new(),
            periodic_effects_reported: BTreeMap::new(),
        }
    }

    pub fn current_room<'a>(&self, content: &'a ContentPack) -> Option<&'a RoomDefinition> {
        content.room(&self.current_room_id)
    }

    /// Whether the player is currently under an active invisibility effect.
    pub fn is_player_invisible(&self) -> bool {
        self.combat
            .invisible_until_minute
            .is_some_and(|until| self.clock.current_time_minutes < until)
    }

    /// Minutes of invisibility remaining for the player, or 0 if visible.
    pub fn player_invisibility_remaining_minutes(&self) -> u32 {
        self.combat
            .invisible_until_minute
            .map(|until| until.saturating_sub(self.clock.current_time_minutes))
            .unwrap_or(0)
    }

    /// Looks up an actor definition by id, checking runtime spawned actors first
    /// and falling back to static pack actors.
    pub fn actor<'a>(
        &'a self,
        content: &'a ContentPack,
        actor_id: &str,
    ) -> Option<&'a ActorDefinition> {
        self.actors
            .spawned_actors
            .get(actor_id)
            .or_else(|| content.actor(actor_id))
    }

    /// Looks up an actor's display name, checking runtime name overrides first,
    /// then runtime spawned actors, and finally static pack actors.
    pub fn actor_display_name<'a>(
        &'a self,
        content: &'a ContentPack,
        actor_id: &str,
    ) -> Option<&'a str> {
        if let Some(name) = self.actors.actor_name_overrides.get(actor_id) {
            return Some(name.as_str());
        }
        self.actor(content, actor_id)
            .map(|actor| actor.name.as_str())
    }

    /// Whether the actor has applied any transformation stage. This is the
    /// legacy "is awakened" check — awakening is simply a transformation
    /// stage with id `"awakening"`.
    pub fn is_actor_awakened(&self, actor_id: &str) -> bool {
        self.actors
            .transformed_stages
            .get(actor_id)
            .is_some_and(|stages| !stages.is_empty())
    }

    /// Whether the actor has applied the given transformation stage.
    pub fn is_actor_transformed(&self, actor_id: &str, stage_id: &str) -> bool {
        self.actors
            .transformed_stages
            .get(actor_id)
            .is_some_and(|stages| stages.iter().any(|stage| stage == stage_id))
    }

    /// Whether an actor possesses a given skill.
    pub fn actor_has_skill(&self, actor_id: &str, skill_id: &str) -> bool {
        self.combat
            .actor_skills
            .get(actor_id)
            .is_some_and(|skills| skills.contains(skill_id))
    }

    /// Grants a skill to an actor. Returns true if the skill was newly acquired.
    pub fn grant_actor_skill(&mut self, actor_id: &str, skill_id: &str) -> bool {
        let is_new = self
            .combat
            .actor_skills
            .entry(actor_id.to_string())
            .or_default()
            .insert(skill_id.to_string());
        if is_new {
            let char_level = self.actor_level(actor_id);
            let skill_level = if actor_id == self.player_actor_id {
                1
            } else {
                char_level.clamp(1, 10)
            };
            self.combat
                .actor_skill_levels
                .entry(actor_id.to_string())
                .or_default()
                .entry(skill_id.to_string())
                .or_insert(skill_level);
            self.combat
                .actor_skill_sp
                .entry(actor_id.to_string())
                .or_default()
                .entry(skill_id.to_string())
                .or_insert(if actor_id == self.player_actor_id {
                    0
                } else {
                    crate::engine::skills::skill_sp_for_level(skill_level)
                });
        }
        is_new
    }

    /// Backfills authored starting skills into a restored save. Acquired skills
    /// remain untouched, while saves created before actor skill persistence
    /// gain the capabilities their current content definitions require.
    pub fn reconcile_authored_actor_skills(&mut self, content: &ContentPack) {
        let authored = content
            .actors
            .iter()
            .chain(self.actors.spawned_actors.values())
            .map(|actor| {
                (
                    actor.id.clone(),
                    actor
                        .skills
                        .iter()
                        .map(|skill| skill.id().to_string())
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();
        for (actor_id, skills) in authored {
            self.combat
                .actor_skills
                .entry(actor_id)
                .or_default()
                .extend(skills);
        }

        let player_id = content.settings.combat.player_actor_id.clone();
        let story_unlocked_skills = content
            .actions
            .iter()
            .filter(|action| {
                !action.skill_id.is_empty()
                    && !action.available.when.is_empty()
                    && crate::engine::conditions::all_hold(
                        &action.available.when,
                        &crate::engine::conditions::ConditionContext::new(
                            content,
                            self,
                            &self.current_room_id,
                        ),
                    )
            })
            .map(|action| action.skill_id.clone())
            .collect::<Vec<_>>();
        self.combat
            .actor_skills
            .entry(player_id.clone())
            .or_default()
            .extend(story_unlocked_skills);

        if self
            .story_vars
            .get("knows_drain")
            .is_some_and(|v| v == "true")
        {
            self.grant_actor_skill(&player_id, "drain");
        }
        if self
            .story_vars
            .get("knows_teleport")
            .is_some_and(|v| v == "true")
        {
            self.grant_actor_skill(&player_id, "teleport");
        }

        // Ensure any awakened actors possess the comms skill
        let awakened_ids = self
            .actors
            .transformed_stages
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for actor_id in awakened_ids {
            if self.is_actor_awakened(&actor_id) {
                self.grant_actor_skill(&actor_id, "comms");
            }
        }

        // Ensure all registered actor skills have skill levels and SP initialized
        let all_skills = self.combat.actor_skills.clone();
        for (actor_id, skills) in all_skills {
            let char_level = self.actor_level(&actor_id);
            for skill_id in skills {
                let default_lvl = if actor_id == player_id {
                    1
                } else {
                    char_level.clamp(1, 10)
                };
                self.combat
                    .actor_skill_levels
                    .entry(actor_id.clone())
                    .or_default()
                    .entry(skill_id.clone())
                    .or_insert(default_lvl);
                self.combat
                    .actor_skill_sp
                    .entry(actor_id.clone())
                    .or_default()
                    .entry(skill_id)
                    .or_insert(if actor_id == player_id {
                        0
                    } else {
                        crate::engine::skills::skill_sp_for_level(default_lvl)
                    });
            }
        }
    }

    /// The first skill of `kind` this actor owns, resolved through the content
    /// pack. Skills are stored in a `BTreeSet`, so a given kind resolves
    /// deterministically by skill id.
    pub fn actor_skill_of_kind<'a>(
        &self,
        content: &'a ContentPack,
        actor_id: &str,
        kind: SkillKind,
    ) -> Option<&'a SkillDefinition> {
        self.combat
            .actor_skills
            .get(actor_id)?
            .iter()
            .filter_map(|skill_id| content.skill(skill_id))
            .find(|skill| skill.kind == Some(kind))
    }

    /// Transformation stages applied to an actor, in application order.
    pub fn transformed_stages(&self, actor_id: &str) -> &[String] {
        self.actors
            .transformed_stages
            .get(actor_id)
            .map(|stages| stages.as_slice())
            .unwrap_or(&[])
    }

    /// Records that a transformation stage has been applied, also mirroring
    /// it as a truthy `transformed:{actor}:{stage}` story variable.
    pub fn set_transformation_applied(&mut self, actor_id: &str, stage_id: &str) {
        let stages = self
            .actors
            .transformed_stages
            .entry(actor_id.to_string())
            .or_default();
        if !stages.iter().any(|stage| stage == stage_id) {
            stages.push(stage_id.to_string());
        }
        let _ = self
            .story_vars
            .set(&format!("transformed:{actor_id}:{stage_id}"), "true");
    }

    /// Whether the actor is offstage, checking runtime spawned actors and room overrides first.
    pub fn actor_is_offstage(&self, content: &ContentPack, actor_id: &str) -> bool {
        if self
            .actors
            .actor_room_overrides
            .get(actor_id)
            .is_some_and(|r| !r.trim().is_empty())
        {
            return false;
        }
        self.actor(content, actor_id)
            .is_some_and(|actor| actor.is_offstage())
    }

    /// Iterates all onstage actors across static content and runtime spawned actors.
    pub fn onstage_actors<'a>(
        &'a self,
        content: &'a ContentPack,
    ) -> impl Iterator<Item = &'a ActorDefinition> {
        content
            .actors
            .iter()
            .filter(move |actor| !self.actor_is_offstage(content, &actor.id))
            .chain(
                self.actors
                    .spawned_actors
                    .values()
                    .filter(move |actor| !self.actor_is_offstage(content, &actor.id)),
            )
    }

    /// Iterates all actors across static content and runtime spawned actors.
    pub fn actors<'a>(
        &'a self,
        content: &'a ContentPack,
    ) -> impl Iterator<Item = &'a ActorDefinition> {
        content
            .actors
            .iter()
            .chain(self.actors.spawned_actors.values())
    }
}

const MINUTES_PER_DAY: u32 = 24 * 60;

fn feature_key(room_id: &str, feature_id: &str) -> String {
    format!("{room_id}::{feature_id}")
}

fn feature_item_key(room_id: &str, feature_id: &str, item_id: &str) -> String {
    format!("{room_id}::{feature_id}::{item_id}")
}

fn seeded_room_item_stock(content: &ContentPack) -> BTreeMap<String, u32> {
    content
        .rooms
        .iter()
        .flat_map(|room| {
            room.items
                .iter()
                .filter(|(_, count)| **count > 0)
                .map(|(item_id, count)| (room_item_key(&room.id, item_id), *count))
        })
        .collect()
}

fn room_item_key(room_id: &str, item_id: &str) -> String {
    format!("{room_id}::{item_id}")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnOutcome {
    pub phase: GamePhase,
    /// The narrative lines produced by the turn, each tagged with how it
    /// should be styled. This is the single source of truth for narrative output.
    #[serde(default)]
    pub lines: Vec<crate::engine::narrative::NarrativeLine>,
}

impl TurnOutcome {
    pub fn new(phase: GamePhase, lines: Vec<crate::engine::narrative::NarrativeLine>) -> Self {
        Self { phase, lines }
    }

    /// Convenience accessor joining the line texts with double newlines.
    pub fn text(&self) -> String {
        self.lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSnapshot {
    pub turn_number: u32,
    pub current_room_id: String,
}

mod seeding;
use seeding::{
    seeded_actor_equipment, seeded_actor_inventories, seeded_actor_levels,
    seeded_actor_skill_levels, seeded_actor_skill_sp, seeded_actor_skills, seeded_actor_stats,
    seeded_feature_item_stock, seeded_pair_stats,
};
mod actor_display;
pub use actor_display::{
    display_actor_name, render_dynamic_story_text, resolved_actor_prompt_context,
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
            !state.actors.actor_known_room_ids.contains_key("blair"),
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

    #[test]
    fn from_saved_json_accepts_current_v2_state_unchanged() {
        let mut content = crate::engine::test_fixtures::minimal_test_pack();
        content
            .actors
            .iter_mut()
            .find(|actor| actor.id == "blair")
            .unwrap()
            .initial_relationship = Some(ActorRelationship {
            stance: ActorStance::Allied,
            follows_player: true,
        });
        let state = WorldState::new(&content);
        let raw = serde_json::to_string(&state).expect("current state serializes");
        let reloaded = WorldState::from_saved_json(&raw).expect("current v2 roundtrips");
        assert_eq!(reloaded.state_version, WORLD_STATE_VERSION);
        assert_eq!(reloaded.current_room_id, state.current_room_id);
        assert_eq!(reloaded.clock.turn_number, state.clock.turn_number);
        assert_eq!(reloaded.transition_summaries, state.transition_summaries);
        assert_eq!(
            reloaded.relations.relationships,
            state.relations.relationships
        );
    }

    #[test]
    fn restored_state_backfills_authored_skills_without_losing_acquired_skills() {
        let mut content = crate::engine::test_fixtures::minimal_test_pack();
        let actor_id = content.actors[0].id.clone();
        content.actors[0].skills = vec![crate::content::types::ActorSkillAssignment::Id(
            "strike".to_string(),
        )];
        content
            .actions
            .push(crate::content::types::ActionDefinition {
                id: "teleport".to_string(),
                skill_id: "teleport".to_string(),
                available: crate::content::types::ActionAvailability {
                    when: vec![crate::content::types::Condition::StoryVar(
                        "knows_teleport".to_string(),
                    )],
                    ..Default::default()
                },
                ..Default::default()
            });
        let mut state = WorldState::new(&content);
        state.combat.actor_skills.clear();
        state.grant_actor_skill(&actor_id, "learned");
        state.story_vars.set_unchecked("knows_teleport", "true");

        state.reconcile_authored_actor_skills(&content);

        assert!(state.actor_has_skill(&actor_id, "strike"));
        assert!(state.actor_has_skill(&actor_id, "learned"));
        assert!(state.actor_has_skill(&content.settings.combat.player_actor_id, "teleport"));
    }
}
