//! World-shape definitions for a content pack, split by concern: movement,
//! behavior, speech, beat objectives, presentation, rooms, actors, drops, and
//! stat metadata.

mod actor_defs;
mod beat_objectives;
mod behavior;
mod drops;
mod movement;
mod presentation;
mod rooms;
mod speech;
mod stats;
mod teleports;

pub use actor_defs::{
    ActorActCast, ActorDefinition, ActorPromptContext, ActorTransformation, TransformationRename,
    TransformationTrigger,
};
pub use beat_objectives::{
    BeatObjectiveAffordancePriorityDefinition, BeatObjectiveAffordanceTarget,
    BeatObjectiveCompletionDefinition, BeatObjectiveCompletionTrigger,
    BeatObjectiveConditionalGuidanceDefinition, BeatObjectiveDefinition,
    BeatObjectiveGuidanceDefinition, BeatObjectiveProgressDefinition,
    BeatObjectiveProgressKeyDefinition, BeatObjectivesDefinition,
};
pub use behavior::{BehaviorActorDefinition, BehaviorDefinition};
pub use drops::{DropChanceSpec, DropConditionSpec, DropPoolEntry, DropPoolSpec, DropSpec};
pub use movement::{
    ActorMovementRulesDefinition, ActorMovementTargetRuleDefinition, MovementConfigDefinition,
    MovementDefaultsDefinition, MovementTargetBehavior, WanderDefinition, WanderMode,
};
pub use presentation::{ErrorTextDefinition, PresentationDefinition, PresentationTextDefinition};
pub use rooms::{
    ConsumableDefinition, ConsumableKind, RoomDefinition, RoomDescriptionOverride,
    RoomExitDefinition, RoomFeatureDefinition,
};
pub use speech::SpeechConfigDefinition;
pub use stats::{StatDefinition, StatsDefinition};
pub use teleports::{TeleportAnchorDefinition, TeleportNetworkDefinition};
