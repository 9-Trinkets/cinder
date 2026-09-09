pub use super::text_defs::{
    ActClosureDefinition, ActClosureSectionDefinition, ActClosureSource, ShellMenuDefinition,
    ShellMenuItem, SystemTextDefinition, UiTextDefinition,
};

use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

mod messages;
pub use messages::{PackMessage, PackMessageVoice};

mod settings;
pub use settings::{CharmRule, ContentSettingsDefinition};
pub(super) use settings::{default_actor_targeted_speech, default_stat_default_value};

mod channels;
pub use channels::{
    ChannelAvailability, ChannelKind, ChannelPrivacy, LOCAL_CHANNEL_ID, MessagingChannel,
};

mod sequences;
pub use sequences::{ScriptedLine, ScriptedSequence, ScriptedSequenceGate, SequencesDefinition};

mod party_defs;
pub use party_defs::{
    PartyCandidatePriority, PartyCombatDecisionRule, PartyDecisionCondition, PartyDecisionTier,
    PartyOrder, PartyOrderKind, PartyOrderStatus, PartyOrderTarget, PartyPolicyDefinition,
    PartyReactionAction, PartyReactionCooldown, PartyReactionWindow, PartyRoleDefinition,
    PartySupportEffect, PartyTargetSelection,
};

mod leveling;
pub use leveling::{LevelDefinition, LevelTable, LevelingDefinition};

mod map_defs;
pub use map_defs::{MapDefinition, MapRevealCondition, MapRoomDefinition};

mod speech;
pub use speech::{SpeechIntentEffect, SpeechIntentLabel, SpeechIntentsConfig};

mod story_defs;
pub use story_defs::{
    ActCastMember, ActorRelocationDefinition, AdvanceCondition, AdvanceEffect, AdvanceSignal,
    BeatDefinition, BeatsDefinition, MenuTriggerMode, OpeningDefinition, OpeningMenuDefinition,
    OpeningMenuOptionDefinition, OpeningMovieDefinition, OpeningMovieFrameDefinition,
    OpeningPromptContext, StageAssignmentDefinition,
};

mod theme;
pub use theme::ThemeDefinition;
mod combat_defs;
pub use combat_defs::*;
mod actor_tick_defs;
pub use actor_tick_defs::*;

mod world_defs;
pub use world_defs::{
    ActorDefinition, ActorMovementRulesDefinition, ActorMovementTargetRuleDefinition,
    ActorPromptContext, BeatObjectiveAffordancePriorityDefinition, BeatObjectiveAffordanceTarget,
    BeatObjectiveCompletionDefinition, BeatObjectiveCompletionTrigger,
    BeatObjectiveConditionalGuidanceDefinition, BeatObjectiveDefinition,
    BeatObjectiveGuidanceDefinition, BeatObjectiveProgressDefinition,
    BeatObjectiveProgressKeyDefinition, BeatObjectivesDefinition, BehaviorActorDefinition,
    BehaviorDefinition, ConsumableDefinition, ConsumableKind, DropConditionSpec, DropSpec,
    ErrorTextDefinition, MovementConfigDefinition, MovementDefaultsDefinition,
    MovementTargetBehavior, PresentationDefinition, PresentationTextDefinition, RoomDefinition,
    RoomDescriptionOverride, RoomExitDefinition, RoomFeatureDefinition, SpeechConfigDefinition,
    StatDefinition, StatsDefinition, WanderDefinition, WanderMode,
};

mod command_defs;
pub use command_defs::{
    BeatObjectiveProgressRef, CommandEffect, CommandInputMode, CommandOutcomeMode,
    CommandTargetMode, ContentEventDefinition, ItemConsumerTarget, ItemStorageTarget,
    PlayerCommandInputMetadata, PlayerCommandMetadata, PlayerCommandTargetMode,
};

mod action_defs;
pub use action_defs::{
    ActionAvailability, ActionContentEvent, ActionDefinition, ActionItemConsumerTarget,
    ActionItemCreation, ActionItemStorageTarget, ActionNpc, ActionPlayerCommand, ActionPlayerInput,
    ActionUi, ActionsDefinition, PanelConfig, PanelDataSource, PanelSelectAction,
};

mod content_pack;
pub use content_pack::{ContentPack, RoomConsumableRef};

mod item_defs;
pub use item_defs::{ItemDefinition, ItemKind};
