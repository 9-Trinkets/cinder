use super::common::*;
use cinder_core::content::types::{
    ActionDefinition, AdvanceCondition, AdvanceSignal, BeatDefinition, BeatsDefinition,
    CommandEffect, BeatObjectiveCompletionDefinition, BeatObjectiveDefinition,
    BeatObjectiveGuidanceDefinition, BeatObjectiveProgressDefinition, BeatObjectiveProgressKeyDefinition,
    BeatObjectiveProgressRef, BeatObjectivesDefinition,
};
use cinder_core::engine::events::{TimestampedWorldEvent, WorldEvent};
use cinder_core::engine::reducer::apply_events;
use cinder_core::engine::state::WorldState;
use serde_json::json;

#[test]
fn command_used_signal_can_advance_stage_after_objective_completion_and_clears_progress() {
    let mut pack = reducer_test_pack();
    pack.beats = BeatsDefinition {
        initial_stage_ids: vec!["dinner-prep".to_string()],
        stages: vec![
            BeatDefinition {
                id: "dinner-prep".to_string(),
                advance_signals: vec![AdvanceSignal::Conditional {
                    signal: "command_used".to_string(),
                    conditions: vec![AdvanceCondition {
                        path: "story_vars.values.beat_objective:progress:dinner-prep-cook-and-check-in:meal_ready".to_string(),
                        operator: "equal".to_string(),
                        value: json!("true"),
                    }],
                }],
                next_stage_ids: vec!["share-dinner".to_string()],
                ..BeatDefinition::default()
            },
            BeatDefinition {
                id: "share-dinner".to_string(),
                ..BeatDefinition::default()
            },
        ],
    };
    pack.beat_objectives = BeatObjectivesDefinition {
        objectives: vec![BeatObjectiveDefinition {
            id: "dinner-prep-cook-and-check-in".to_string(),
            stage_ids: vec!["dinner-prep".to_string()],
            progress: BeatObjectiveProgressDefinition {
                keys: vec![BeatObjectiveProgressKeyDefinition {
                    key: "meal_ready".to_string(),
                    label: "meal ready".to_string(),
                }],
            },
            completion: BeatObjectiveCompletionDefinition::default(),
            guidance: BeatObjectiveGuidanceDefinition::default(),
        }],
    };
    pack.actions.push(ActionDefinition {
        id: "cook".to_string(),
        command: "COOK".to_string(),
        effects: vec![CommandEffect::RememberInRoom],
        event_text: "{actor_name} finishes dinner.".to_string(),
        sets_objective_progress: vec![BeatObjectiveProgressRef {
            objective_id: "dinner-prep-cook-and-check-in".to_string(),
            key: "meal_ready".to_string(),
        }],
        ..ActionDefinition::default()
    });
    rebuild_test_pack_indexes(&mut pack);
    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();

    let events = [TimestampedWorldEvent::now(WorldEvent::ActorCommandUsed {
        actor_id: ACTOR_A_ID.to_string(),
        actor_name: ACTOR_A_NAME.to_string(),
        room_id: LOUNGE_ID.to_string(),
        command_id: "cook".to_string(),
        target_room_id: None,
        target_actor_id: None,
        target_actor_name: None,
        context_label: None,
        feature_id: None,
        consumable_id: None,
        freeform_text: None,
    })];

    apply_events(&mut state, &pack, &events);

    assert_eq!(
        state.active_objective_stage_ids,
        vec!["share-dinner".to_string()]
    );
    assert_eq!(
        state
            .story_vars
            .get("beat_objective:progress:dinner-prep-cook-and-check-in:meal_ready"),
        None
    );
}
