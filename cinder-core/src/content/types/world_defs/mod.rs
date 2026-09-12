//! World-shape definitions for a content pack, split by concern: movement,
//! behavior, speech, beat objectives, presentation, rooms, actors, drops, and
//! stat metadata.

mod actor_defs;
mod behavior;
mod beat_objectives;
mod drops;
mod movement;
mod presentation;
mod rooms;
mod speech;
mod stats;

pub use actor_defs::{ActorDefinition, ActorPromptContext};
pub use behavior::{BehaviorActorDefinition, BehaviorDefinition};
pub use beat_objectives::{
    BeatObjectiveAffordancePriorityDefinition, BeatObjectiveAffordanceTarget,
    BeatObjectiveCompletionDefinition, BeatObjectiveCompletionTrigger,
    BeatObjectiveConditionalGuidanceDefinition, BeatObjectiveDefinition,
    BeatObjectiveGuidanceDefinition, BeatObjectiveProgressDefinition,
    BeatObjectiveProgressKeyDefinition, BeatObjectivesDefinition,
};
pub use drops::{DropChanceSpec, DropConditionSpec, DropPoolEntry, DropPoolSpec, DropSpec};
pub use movement::{
    ActorMovementRulesDefinition, ActorMovementTargetRuleDefinition, MovementConfigDefinition,
    MovementDefaultsDefinition, MovementTargetBehavior, WanderDefinition, WanderMode,
};
pub use presentation::{ErrorTextDefinition, PresentationDefinition, PresentationTextDefinition};
pub use rooms::{ConsumableDefinition, ConsumableKind, RoomDefinition, RoomDescriptionOverride, RoomExitDefinition, RoomFeatureDefinition};
pub use speech::SpeechConfigDefinition;
pub use stats::{StatDefinition, StatsDefinition};