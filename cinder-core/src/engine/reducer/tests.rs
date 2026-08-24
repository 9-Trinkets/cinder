use super::beat_advance::{advance_conditions_met, evaluate_advance_condition};
use super::*;
use crate::content::types::{
    ActionDefinition, ActionItemCreation, ActionItemStorageTarget, ActorDefinition,
    ActorPromptContext, AdvanceCondition, AdvanceSignal, BeatDefinition, BeatsDefinition,
    CombatSettingsDefinition, CommandEffect, CommandInputMode, CommandTargetMode, ContentPack,
    ContentSettingsDefinition, ConversionTrigger, ItemDefinition, ItemStorageTarget,
    OpeningDefinition, PresentationDefinition, RoomDefinition, RoomExitDefinition,
    RoomFeatureDefinition, RuleBundleCompletionDefinition, RuleBundleDefinition,
    RuleBundleGuidanceDefinition, RuleBundleProgressDefinition, RuleBundleProgressKeyDefinition,
    RuleBundleProgressRef, RuleBundlesDefinition, StatDefinition, StatsDefinition,
};
use crate::engine::state::{ActorStance, ConversationMemoryKind, GamePhase};
use crate::engine::test_fixtures::minimal_test_pack;
use serde_json::json;
use std::collections::{BTreeMap, HashMap};

const ACTOR_A_ID: &str = "alex";
const ACTOR_A_NAME: &str = "Alex";
const ACTOR_B_ID: &str = "blair";
const ACTOR_B_NAME: &str = "Blair";
const ACTOR_C_ID: &str = "casey";
const ACTOR_C_NAME: &str = "Casey";
const LOUNGE_ID: &str = "lounge";
const LOUNGE_TITLE: &str = "Lounge";
const KITCHEN_ID: &str = "kitchen";
const KITCHEN_TITLE: &str = "Kitchen";
const SOFA_ID: &str = "sofa";
const SOFA_LABEL: &str = "long sofa";

fn reducer_test_pack() -> ContentPack {
    let mut pack = minimal_test_pack();
    pack.settings = ContentSettingsDefinition {
        tick_minutes_per_turn: 1,
        ..ContentSettingsDefinition::default()
    };
    pack.opening = OpeningDefinition {
        id: "reducer-test".to_string(),
        start_room_id: LOUNGE_ID.to_string(),
        ..OpeningDefinition::default()
    };
    pack.beats = Default::default();
    pack.menus.clear();
    pack.movies.clear();
    pack.items.clear();
    pack.speech_intents = Default::default();
    pack.presentation = reducer_test_presentation();
    pack.rooms = vec![
        RoomDefinition {
            id: LOUNGE_ID.to_string(),
            title: LOUNGE_TITLE.to_string(),
            summary: "A comfortable lounge.".to_string(),
            inspect_text: "The lounge feels lived in.".to_string(),
            allow_rest: true,
            features: vec![RoomFeatureDefinition {
                id: SOFA_ID.to_string(),
                label: SOFA_LABEL.to_string(),
                aliases: vec!["sofa".to_string()],
                allow_rest: true,
                consumables: vec![],
                inspect_text: "The sofa looks like the room's best place to rest.".to_string(),
            }],
            exits: vec![RoomExitDefinition {
                room_id: KITCHEN_ID.to_string(),
                label: KITCHEN_TITLE.to_string(),
                aliases: vec!["kitchen".to_string()],
                menu_label: None,
            }],
        },
        RoomDefinition {
            id: KITCHEN_ID.to_string(),
            title: KITCHEN_TITLE.to_string(),
            summary: "A quiet kitchen.".to_string(),
            inspect_text: "The kitchen is tidy and bright.".to_string(),
            allow_rest: false,
            features: vec![],
            exits: vec![RoomExitDefinition {
                room_id: LOUNGE_ID.to_string(),
                label: LOUNGE_TITLE.to_string(),
                aliases: vec!["lounge".to_string()],
                menu_label: None,
            }],
        },
    ];
    pack.actors = vec![
        test_actor(ACTOR_A_ID, ACTOR_A_NAME, LOUNGE_ID),
        test_actor(ACTOR_B_ID, ACTOR_B_NAME, LOUNGE_ID),
        test_actor(ACTOR_C_ID, ACTOR_C_NAME, KITCHEN_ID),
    ];
    pack.act_cast.clear();
    pack.stats = reducer_test_stats();
    pack.actions = vec![
        ActionDefinition {
            id: "act".to_string(),
            command: "act".to_string(),
            input_mode: CommandInputMode::FreeformText,
            effects: vec![CommandEffect::RememberInRoom],
            ..ActionDefinition::default()
        },
        ActionDefinition {
            id: "hug".to_string(),
            command: "hug".to_string(),
            target_mode: CommandTargetMode::Actor,
            hook_id: "actor.hugged".to_string(),
            event_text: "{actor_name} hugs {target_actor_name}.".to_string(),
            ..ActionDefinition::default()
        },
        ActionDefinition {
            id: "rest".to_string(),
            command: "rest".to_string(),
            target_mode: CommandTargetMode::ContextLabel,
            hook_id: "actor.rested".to_string(),
            event_text: "{actor_name} takes a quiet moment to rest on the {context_label}."
                .to_string(),
            ..ActionDefinition::default()
        },
        ActionDefinition {
            id: "move".to_string(),
            command: "move".to_string(),
            target_mode: CommandTargetMode::Room,
            effects: vec![CommandEffect::MoveActor],
            event_text: "{actor_name} heads to the {target_room_title}.".to_string(),
            ..ActionDefinition::default()
        },
    ];
    pack.hooks = serde_json::from_value(json!({
        "conversation.shared_room_tick": effect_hook(vec![json!({
            "kind": "adjust_pair_stat",
            "participant_a_id": "$input.participant_a_id",
            "participant_b_id": "$input.participant_b_id",
            "stat": "safety",
            "delta": 1
        })]),
        "conversation.speech": effect_hook(vec![
            json!({
                "kind": "adjust_pair_stat",
                "participant_a_id": "$input.participant_a_id",
                "participant_b_id": "$input.participant_b_id",
                "stat": "connection",
                "delta": 1
            }),
            json!({
                "kind": "adjust_actor_stat",
                "actor_id": "$input.actor_id",
                "stat": "confidence",
                "delta": 1
            })
        ]),
        "conversation.broken_reply": effect_hook(vec![json!({
            "kind": "adjust_pair_stat",
            "participant_a_id": "$input.participant_a_id",
            "participant_b_id": "$input.participant_b_id",
            "stat": "safety",
            "delta": -1
        })]),
        "actor.time_advanced": effect_hook(vec![json!({
            "kind": "adjust_actor_stat",
            "actor_id": "$input.actor_id",
            "stat": "hunger",
            "delta": 1
        })]),
        "actor.rested": effect_hook(vec![json!({
            "kind": "adjust_actor_stat",
            "actor_id": "$input.actor_id",
            "stat": "stamina",
            "delta": 1
        })]),
        "actor.hugged": effect_hook(vec![
            json!({
                "kind": "adjust_pair_stat",
                "participant_a_id": "$input.actor_id",
                "participant_b_id": "$input.target_actor_id",
                "stat": "safety",
                "delta": 1
            }),
            json!({
                "kind": "adjust_pair_stat",
                "participant_a_id": "$input.actor_id",
                "participant_b_id": "$input.target_actor_id",
                "stat": "attraction",
                "delta": 1
            })
        ])
    }))
    .expect("build reducer test hooks");
    rebuild_test_pack_indexes(&mut pack);
    pack
}

fn reducer_test_presentation() -> PresentationDefinition {
    let mut presentation = PresentationDefinition::default();
    presentation.presentation_text.actor_speech = "{actor_name}: {text}".to_string();
    presentation.presentation_text.actor_targeted_speech =
        "{actor_name} (to {target_name}): {text}".to_string();
    presentation.presentation_text.actor_arrived =
        "{actor_name} comes in from the {room_title}.".to_string();
    presentation.presentation_text.actor_departed =
        "{actor_name} heads toward the {room_title}.".to_string();
    presentation.presentation_text.act_ended = "Session ended.".to_string();
    presentation.error_text.room_missing = "missing room".to_string();
    presentation.error_text.actor_unknown = "unknown actor".to_string();
    presentation.error_text.feature_unknown = "unknown feature".to_string();
    presentation.error_text.unknown_input = "unknown input".to_string();
    presentation
}

fn reducer_test_stats() -> StatsDefinition {
    StatsDefinition {
        actor: BTreeMap::from([
            (
                "hunger".to_string(),
                StatDefinition {
                    time_step_minutes: Some(1),
                    ..StatDefinition::default()
                },
            ),
            (
                "stamina".to_string(),
                StatDefinition {
                    default: 5,
                    ..StatDefinition::default()
                },
            ),
            ("confidence".to_string(), StatDefinition::default()),
        ]),
        pair: BTreeMap::from([
            ("safety".to_string(), StatDefinition::default()),
            ("attraction".to_string(), StatDefinition::default()),
            ("connection".to_string(), StatDefinition::default()),
        ]),
    }
}

fn test_actor(id: &str, name: &str, room_id: &str) -> ActorDefinition {
    ActorDefinition {
        id: id.to_string(),
        name: name.to_string(),
        room_id: room_id.to_string(),
        initial_stats: BTreeMap::new(),
        initial_pair_stats: BTreeMap::new(),
        aliases: vec![],
        inspect_text: format!("{name} looks thoughtful."),
        required_consumable_tags: vec![],
        attackable: false,
        conversion_trigger: None,
        attack_interval_minutes: None,
        prompt_context: ActorPromptContext {
            character_notes: vec![],
            subtext_notes: vec![],
            response_notes: vec![],
            behavior_examples: vec![],
        },
        act_cast: None,
        game_data: BTreeMap::new(),
    }
}

fn effect_hook(effects: Vec<serde_json::Value>) -> serde_json::Value {
    json!({
        "rule": "effect_table",
        "rule_config": {
            "cases_path": "rules",
            "next_on_match": "complete",
            "next_on_default": "complete",
            "default_payload_template": {
                "effects": []
            }
        },
        "input_overlay": {
            "rules": effects.into_iter().map(|effect| json!({
                "conditions": [],
                "payload_template": effect
            })).collect::<Vec<_>>()
        }
    })
}

fn rebuild_test_pack_indexes(pack: &mut ContentPack) {
    pack.room_index = pack
        .rooms
        .iter()
        .enumerate()
        .map(|(index, room)| (room.id.clone(), index))
        .collect::<HashMap<_, _>>();
    pack.actor_index = pack
        .actors
        .iter()
        .enumerate()
        .map(|(index, actor)| (actor.id.clone(), index))
        .collect::<HashMap<_, _>>();
    pack.action_index = pack
        .actions
        .iter()
        .enumerate()
        .map(|(index, action)| (action.id.clone(), index))
        .collect::<HashMap<_, _>>();
}

#[test]
fn actor_action_is_injected_into_roommate_recent_memory() {
    let pack = reducer_test_pack();
    let mut state = WorldState::new(&pack);
    state
        .actor_room_overrides
        .insert(ACTOR_A_ID.to_string(), LOUNGE_ID.to_string());
    state
        .actor_room_overrides
        .insert(ACTOR_B_ID.to_string(), LOUNGE_ID.to_string());
    let events = [TimestampedWorldEvent::now(WorldEvent::ActorCommandUsed {
        actor_id: ACTOR_A_ID.to_string(),
        actor_name: ACTOR_A_NAME.to_string(),
        room_id: LOUNGE_ID.to_string(),
        command_id: "act".to_string(),
        target_room_id: None,
        target_actor_id: None,
        target_actor_name: None,
        context_label: None,
        feature_id: None,
        consumable_id: None,
        freeform_text: Some("sits on the couch".to_string()),
    })];

    let output = apply_events(&mut state, &pack, &events);

    assert!(
        output
            .lines
            .iter()
            .any(|line| line == "Alex sits on the couch.")
    );
    let history = state.conversation_history(ACTOR_A_ID, ACTOR_B_ID);
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].kind, ConversationMemoryKind::Action);
    assert_eq!(history[0].text, "Alex sits on the couch.");
}

#[test]
fn hug_increases_attraction_and_safety_for_the_pair() {
    let pack = reducer_test_pack();
    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    let events = [TimestampedWorldEvent::now(WorldEvent::ActorCommandUsed {
        actor_id: ACTOR_A_ID.to_string(),
        actor_name: ACTOR_A_NAME.to_string(),
        room_id: LOUNGE_ID.to_string(),
        command_id: "hug".to_string(),
        target_room_id: None,
        target_actor_id: Some(ACTOR_C_ID.to_string()),
        target_actor_name: Some(ACTOR_C_NAME.to_string()),
        context_label: None,
        feature_id: None,
        consumable_id: None,
        freeform_text: None,
    })];

    let output = apply_events(&mut state, &pack, &events);

    assert!(output.lines.iter().any(|line| line == "Alex hugs Casey."));
    assert_eq!(state.pair_stat(ACTOR_A_ID, ACTOR_C_ID, "safety"), 1);
    assert_eq!(state.pair_stat_u32(ACTOR_A_ID, ACTOR_C_ID, "attraction"), 1);
}

#[test]
fn shared_room_tick_increases_pair_safety() {
    let pack = reducer_test_pack();
    let mut state = WorldState::new(&pack);
    let events = [TimestampedWorldEvent::now(WorldEvent::TurnStarted {
        turn_number: 1,
        raw_input: "tick".to_string(),
        advances_time: true,
    })];

    apply_events(&mut state, &pack, &events);

    assert_eq!(state.pair_stat(ACTOR_A_ID, ACTOR_B_ID, "safety"), 1);
}

#[test]
fn tick_progression_updates_hunger_without_reducing_stamina() {
    let pack = reducer_test_pack();
    let mut state = WorldState::new(&pack);
    state.current_time_minutes = 18 * 60 + 55;
    let starting_hunger = state.actor_stat_u32(ACTOR_A_ID, "hunger");
    let starting_stamina = state.actor_stat_u32(ACTOR_A_ID, "stamina");
    let events = [TimestampedWorldEvent::now(WorldEvent::TurnStarted {
        turn_number: 1,
        raw_input: "tick".to_string(),
        advances_time: true,
    })];

    apply_events(&mut state, &pack, &events);

    assert_eq!(
        state.actor_stat_u32(ACTOR_A_ID, "hunger"),
        starting_hunger + 1
    );
    assert_eq!(
        state.actor_stat_u32(ACTOR_A_ID, "stamina"),
        starting_stamina
    );
}

#[test]
fn rest_recovers_stamina() {
    let pack = reducer_test_pack();
    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    state
        .adjust_actor_stat(ACTOR_A_ID, "stamina", -4)
        .expect("known actor stat");
    let starting_stamina = state.actor_stat_u32(ACTOR_A_ID, "stamina");
    let events = [TimestampedWorldEvent::now(WorldEvent::ActorCommandUsed {
        actor_id: ACTOR_A_ID.to_string(),
        actor_name: ACTOR_A_NAME.to_string(),
        room_id: LOUNGE_ID.to_string(),
        command_id: "rest".to_string(),
        target_room_id: None,
        target_actor_id: None,
        target_actor_name: None,
        context_label: Some(SOFA_LABEL.to_string()),
        feature_id: None,
        consumable_id: None,
        freeform_text: None,
    })];

    let output = apply_events(&mut state, &pack, &events);

    assert!(
        output
            .lines
            .iter()
            .any(|line| line == "Alex takes a quiet moment to rest on the long sofa.")
    );
    assert_eq!(
        state.actor_stat_u32(ACTOR_A_ID, "stamina"),
        starting_stamina + 1
    );
}

#[test]
fn speech_increases_connection_and_confidence() {
    let pack = reducer_test_pack();
    let mut state = WorldState::new(&pack);
    let starting_confidence = state.actor_stat(ACTOR_A_ID, "confidence");
    let events = [TimestampedWorldEvent::now(WorldEvent::ActorSpoke {
        actor_id: ACTOR_A_ID.to_string(),
        actor_name: ACTOR_A_NAME.to_string(),
        other_person_id: ACTOR_B_ID.to_string(),
        other_person_name: ACTOR_B_NAME.to_string(),
        other_person_message: None,
        room_id: LOUNGE_ID.to_string(),
        text: "Hey.".to_string(),
    })];

    apply_events(&mut state, &pack, &events);

    assert_eq!(state.pair_stat(ACTOR_A_ID, ACTOR_B_ID, "connection"), 1);
    assert_eq!(
        state.actor_stat(ACTOR_A_ID, "confidence"),
        starting_confidence + 1
    );
}

#[test]
fn pair_stat_adjusted_event_increases_attraction() {
    let pack = reducer_test_pack();
    let mut state = WorldState::new(&pack);
    let starting_attraction = state.pair_stat(ACTOR_A_ID, ACTOR_B_ID, "attraction");
    let events = [TimestampedWorldEvent::now(WorldEvent::PairStatAdjusted {
        participant_a_id: ACTOR_A_ID.to_string(),
        participant_b_id: ACTOR_B_ID.to_string(),
        stat: "attraction".to_string(),
        delta: 2,
    })];

    apply_events(&mut state, &pack, &events);

    assert_eq!(
        state.pair_stat(ACTOR_A_ID, ACTOR_B_ID, "attraction"),
        starting_attraction + 2
    );
}

#[test]
fn visible_speech_lines_include_target_when_present() {
    let pack = reducer_test_pack();
    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    let events = [TimestampedWorldEvent::now(WorldEvent::ActorSpoke {
        actor_id: ACTOR_A_ID.to_string(),
        actor_name: ACTOR_A_NAME.to_string(),
        other_person_id: ACTOR_B_ID.to_string(),
        other_person_name: ACTOR_B_NAME.to_string(),
        other_person_message: None,
        room_id: LOUNGE_ID.to_string(),
        text: "Hey.".to_string(),
    })];

    let output = apply_events(&mut state, &pack, &events);

    assert!(
        output
            .lines
            .iter()
            .any(|line| line == "Alex (to Blair): Hey.")
    );
}

#[test]
fn leaving_before_reply_on_next_tick_decreases_safety() {
    let pack = reducer_test_pack();
    let mut state = WorldState::new(&pack);
    state.turn_number = 1;
    state.set_pending_reply(ACTOR_A_ID, ACTOR_B_ID, LOUNGE_ID, 1);
    let events = [
        TimestampedWorldEvent::now(WorldEvent::TurnStarted {
            turn_number: 2,
            raw_input: "tick".to_string(),
            advances_time: true,
        }),
        TimestampedWorldEvent::now(WorldEvent::ActorMoved {
            actor_id: ACTOR_A_ID.to_string(),
            from_room_id: LOUNGE_ID.to_string(),
            to_room_id: KITCHEN_ID.to_string(),
        }),
    ];

    apply_events(&mut state, &pack, &events);

    assert_eq!(state.pair_stat(ACTOR_A_ID, ACTOR_B_ID, "safety"), 0);
    assert!(state.pending_reply(ACTOR_A_ID, ACTOR_B_ID).is_none());
}

#[test]
fn offscreen_move_command_shows_arrival_when_actor_enters_current_room() {
    let pack = reducer_test_pack();
    let mut state = WorldState::new(&pack);
    state.current_room_id = KITCHEN_ID.to_string();
    state
        .actor_room_overrides
        .insert(ACTOR_A_ID.to_string(), LOUNGE_ID.to_string());
    let events = [TimestampedWorldEvent::now(WorldEvent::ActorCommandUsed {
        actor_id: ACTOR_A_ID.to_string(),
        actor_name: ACTOR_A_NAME.to_string(),
        room_id: LOUNGE_ID.to_string(),
        command_id: "move".to_string(),
        target_room_id: Some(KITCHEN_ID.to_string()),
        target_actor_id: None,
        target_actor_name: None,
        context_label: None,
        feature_id: None,
        consumable_id: None,
        freeform_text: None,
    })];

    let output = apply_events(&mut state, &pack, &events);

    assert!(
        output
            .lines
            .iter()
            .any(|line| line == "Alex comes in from the Lounge.")
    );
    assert_eq!(state.actor_room_id(ACTOR_A_ID, LOUNGE_ID), KITCHEN_ID);
}

#[test]
fn actor_observation_events_feed_recent_observation_memory() {
    let pack = reducer_test_pack();
    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    let events = [
        TimestampedWorldEvent::now(WorldEvent::ActorObservedRoom {
            actor_id: ACTOR_A_ID.to_string(),
            actor_name: ACTOR_A_NAME.to_string(),
            room_id: LOUNGE_ID.to_string(),
        }),
        TimestampedWorldEvent::now(WorldEvent::ActorObservedFeature {
            actor_id: ACTOR_A_ID.to_string(),
            actor_name: ACTOR_A_NAME.to_string(),
            room_id: LOUNGE_ID.to_string(),
            feature_id: SOFA_ID.to_string(),
        }),
        TimestampedWorldEvent::now(WorldEvent::ActorObservedActor {
            actor_id: ACTOR_A_ID.to_string(),
            actor_name: ACTOR_A_NAME.to_string(),
            room_id: LOUNGE_ID.to_string(),
            target_actor_id: ACTOR_B_ID.to_string(),
            target_actor_name: ACTOR_B_NAME.to_string(),
        }),
    ];

    let output = apply_events(&mut state, &pack, &events);

    assert!(
        output
            .lines
            .iter()
            .any(|line| line == "Alex pauses to take in the Lounge more carefully.")
    );
    assert!(
        output
            .lines
            .iter()
            .any(|line| line == "Alex studies Blair more closely.")
    );
    assert!(
        output
            .lines
            .iter()
            .any(|line| line == "Alex studies the long sofa.")
    );
    assert!(state.actor_has_seen_feature(ACTOR_A_ID, LOUNGE_ID, SOFA_ID));
    assert!(state.actor_has_studied_actor(ACTOR_A_ID, ACTOR_B_ID));
    assert_eq!(state.actor_recent_observation_notes(ACTOR_A_ID).len(), 3);
}

#[test]
fn advance_conditions_met_simple_signal_always_passes() {
    use crate::content::types::AdvanceSignal;

    let pack = reducer_test_pack();
    let state = WorldState::new(&pack);
    let signal = AdvanceSignal::Simple("time_reached:20:00".to_string());
    assert!(advance_conditions_met(&state, signal.conditions()));
}

#[test]
fn advance_conditions_met_conditional_signal_fires_when_condition_met() {
    use crate::content::types::AdvanceCondition;

    let flat_input = serde_json::json!({
        "score": 5
    });
    let flat_cond = AdvanceCondition {
        path: "score".to_string(),
        operator: "gte".to_string(),
        value: serde_json::json!(4),
    };
    assert!(evaluate_advance_condition(&flat_input, &flat_cond));
}

#[test]
fn advance_conditions_met_conditional_signal_blocked_when_condition_not_met() {
    use crate::content::types::AdvanceCondition;

    let flat_input = serde_json::json!({
        "score": 2
    });
    let flat_cond = AdvanceCondition {
        path: "score".to_string(),
        operator: "gte".to_string(),
        value: serde_json::json!(4),
    };
    assert!(!evaluate_advance_condition(&flat_input, &flat_cond));
}

#[test]
fn command_used_signal_can_advance_stage_after_bundle_completion_and_clears_progress() {
    let mut pack = reducer_test_pack();
    pack.beats = BeatsDefinition {
        initial_stage_ids: vec!["dinner-prep".to_string()],
        stages: vec![
            BeatDefinition {
                id: "dinner-prep".to_string(),
                advance_signals: vec![AdvanceSignal::Conditional {
                    signal: "command_used".to_string(),
                    conditions: vec![AdvanceCondition {
                        path: "story_vars.values.rule_bundle:progress:dinner-prep-cook-and-check-in:meal_ready".to_string(),
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
    pack.rule_bundles = RuleBundlesDefinition {
        bundles: vec![RuleBundleDefinition {
            id: "dinner-prep-cook-and-check-in".to_string(),
            stage_ids: vec!["dinner-prep".to_string()],
            progress: RuleBundleProgressDefinition {
                keys: vec![RuleBundleProgressKeyDefinition {
                    key: "meal_ready".to_string(),
                    label: "meal ready".to_string(),
                }],
            },
            completion: RuleBundleCompletionDefinition::default(),
            guidance: RuleBundleGuidanceDefinition::default(),
        }],
    };
    pack.actions.push(ActionDefinition {
        id: "cook".to_string(),
        command: "COOK".to_string(),
        effects: vec![CommandEffect::RememberInRoom],
        event_text: "{actor_name} finishes dinner.".to_string(),
        sets_bundle_progress: vec![RuleBundleProgressRef {
            bundle_id: "dinner-prep-cook-and-check-in".to_string(),
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
            .get("rule_bundle:progress:dinner-prep-cook-and-check-in:meal_ready"),
        None
    );
}

#[test]
fn item_events_can_store_and_consume_items_in_current_room() {
    let mut pack = reducer_test_pack();
    pack.items = vec![ItemDefinition {
        id: "coffee".to_string(),
        label: "coffee".to_string(),
        description: "Fresh coffee.".to_string(),
    }];
    rebuild_test_pack_indexes(&mut pack);
    let mut state = WorldState::new(&pack);
    state.current_room_id = KITCHEN_ID.to_string();

    apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::ItemAcquired {
            item_id: "coffee".to_string(),
            storage: ItemStorageTarget::CurrentRoom,
        })],
    );

    assert!(state.has_item_in_storage("coffee", ItemStorageTarget::CurrentRoom, KITCHEN_ID,));

    apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::ItemConsumed {
            item_id: "coffee".to_string(),
            storage: ItemStorageTarget::CurrentRoom,
            consumer_id: None,
            consumer_name: None,
        })],
    );

    assert!(!state.has_item_in_storage("coffee", ItemStorageTarget::CurrentRoom, KITCHEN_ID,));
}

#[test]
fn item_events_keep_player_inventory_behavior_unchanged() {
    let mut pack = reducer_test_pack();
    pack.items = vec![ItemDefinition {
        id: "tea".to_string(),
        label: "tea".to_string(),
        description: "Hot tea.".to_string(),
    }];
    rebuild_test_pack_indexes(&mut pack);
    let mut state = WorldState::new(&pack);

    let output = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::ItemAcquired {
            item_id: "tea".to_string(),
            storage: ItemStorageTarget::PlayerInventory,
        })],
    );

    assert!(state.has_item("tea"));
    assert!(
        output
            .lines
            .iter()
            .any(|line| line == "You have tea ready.")
    );
}

#[test]
fn actor_commands_can_create_room_items_from_story_vars() {
    let mut pack = reducer_test_pack();
    pack.items = vec![
        ItemDefinition {
            id: "garlic-noodles".to_string(),
            label: "garlic noodles".to_string(),
            description: "Noodles.".to_string(),
        },
        ItemDefinition {
            id: "vegetable-stir-fry".to_string(),
            label: "vegetable stir-fry".to_string(),
            description: "Stir-fry.".to_string(),
        },
    ];
    pack.actions.push(ActionDefinition {
        id: "cook".to_string(),
        command: "COOK".to_string(),
        effects: vec![CommandEffect::RememberInRoom],
        event_text: "{actor_name} finishes dinner.".to_string(),
        item_creation: Some(ActionItemCreation {
            creates_item: "garlic-noodles".to_string(),
            creates_item_story_var: "cook_recipe".to_string(),
            creates_item_resolve_from_target: false,
            storage: ActionItemStorageTarget::CurrentRoom,
        }),
        ..ActionDefinition::default()
    });
    rebuild_test_pack_indexes(&mut pack);
    let mut state = WorldState::new(&pack);
    state.current_room_id = KITCHEN_ID.to_string();
    state
        .story_vars
        .set_unchecked("cook_recipe", "vegetable-stir-fry");

    apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::ActorCommandUsed {
            actor_id: ACTOR_C_ID.to_string(),
            actor_name: ACTOR_C_NAME.to_string(),
            room_id: KITCHEN_ID.to_string(),
            command_id: "cook".to_string(),
            target_room_id: None,
            target_actor_id: None,
            target_actor_name: None,
            context_label: None,
            feature_id: None,
            consumable_id: None,
            freeform_text: None,
        })],
    );

    assert!(state.has_item_in_storage(
        "vegetable-stir-fry",
        ItemStorageTarget::CurrentRoom,
        KITCHEN_ID,
    ));
    assert!(!state.has_item_in_storage(
        "garlic-noodles",
        ItemStorageTarget::CurrentRoom,
        KITCHEN_ID,
    ));
}

#[test]
fn hostile_strike_uses_pack_declared_combat_vocabulary() {
    let mut pack = reducer_test_pack();
    pack.stats.actor.insert(
        "vitality".to_string(),
        StatDefinition {
            default: 10,
            ..StatDefinition::default()
        },
    );
    pack.stats
        .actor
        .insert("fury".to_string(), StatDefinition::default());
    pack.stats
        .actor
        .insert("ward".to_string(), StatDefinition::default());
    for actor in &mut pack.actors {
        actor.initial_stats.insert("vitality".to_string(), 10);
        actor.initial_stats.insert("ward".to_string(), 0);
    }
    pack.settings.combat = CombatSettingsDefinition {
        player_actor_id: ACTOR_A_NAME.to_string(),
        health_stat_id: "vitality".to_string(),
        attack_stat_id: "fury".to_string(),
        defense_stat_id: "ward".to_string(),
        minimum_damage: 2,
        default_attack_interval_minutes: 7,
        ..CombatSettingsDefinition::default()
    };
    let mut state = WorldState::new(&pack);
    state
        .actor_stats
        .entry(ACTOR_A_ID.to_string())
        .or_default()
        .insert("fury".to_string(), 5);
    state.set_stance(ACTOR_A_ID, ActorStance::Hostile);
    let events = [TimestampedWorldEvent::now(WorldEvent::HostileStrike {
        actor_id: ACTOR_A_ID.to_string(),
    })];

    apply_events(&mut state, &pack, &events);

    assert_eq!(state.actor_stat(ACTOR_A_NAME, "vitality"), 5);
    assert_eq!(
        state.next_hostile_strike_at.get(ACTOR_A_ID),
        Some(&(state.current_time_minutes + 7))
    );
}

#[test]
fn player_defeat_narration_and_phase_come_from_content() {
    let mut pack = reducer_test_pack();
    let defeat_text = "The lounge lights fade for good.";
    pack.settings.combat = CombatSettingsDefinition {
        player_actor_id: ACTOR_B_NAME.to_string(),
        health_stat_id: "stamina".to_string(),
        attack_stat_id: "confidence".to_string(),
        minimum_damage: 9,
        player_defeat_text: defeat_text.to_string(),
        ..CombatSettingsDefinition::default()
    };
    let mut state = WorldState::new(&pack);
    // Move Casey into the player's room so the strike is in range.
    state
        .actor_room_overrides
        .insert(ACTOR_C_ID.to_string(), LOUNGE_ID.to_string());
    state.set_stance(ACTOR_C_ID, ActorStance::Hostile);
    // Casey shares the lounge with the player actor (Blair).
    let events = [TimestampedWorldEvent::now(WorldEvent::HostileStrike {
        actor_id: ACTOR_C_ID.to_string(),
    })];

    let output = apply_events(&mut state, &pack, &events);

    assert_eq!(state.phase, GamePhase::GameEnded);
    assert_eq!(output.phase, GamePhase::GameEnded);
    assert!(
        output.lines.iter().any(|line| line == defeat_text),
        "expected pack-authored defeat line, got {:?}",
        output.lines
    );
}

#[test]
fn encirclement_conversion_narration_follows_the_flag_placement() {
    let mut pack = reducer_test_pack();
    pack.actions.push(ActionDefinition {
        id: "place-flag".to_string(),
        command: "place-flag".to_string(),
        target_mode: CommandTargetMode::None,
        effects: vec![CommandEffect::DropItem],
        drop_item: "stone-marker".to_string(),
        event_text: "{actor_name} drives a stone marker into the ground.".to_string(),
        ..ActionDefinition::default()
    });
    pack.messages.insert(
        "conversion.encircled".to_string(),
        "The {actor} turns toward you, no longer hostile.".to_string(),
    );
    pack.messages.insert(
        "conversion.encircled_follows".to_string(),
        "The {actor} falls in behind you.".to_string(),
    );
    // A golem in the lounge converts via the `actor.encircled` hook once its
    // only neighbor (the kitchen) holds a stone marker.
    pack.actors.push(ActorDefinition {
        id: "golem".to_string(),
        name: "dark golem".to_string(),
        room_id: LOUNGE_ID.to_string(),
        conversion_trigger: Some(ConversionTrigger::Encirclement),
        ..test_actor("golem", "dark golem", LOUNGE_ID)
    });
    pack.hooks.insert(
        "actor.encircled".to_string(),
        effect_hook(vec![json!({
            "kind": "convert_actor_to_ally",
            "actor_id": "$input.actor_id",
            "follows_player": true,
            "messages": ["conversion.encircled", "conversion.encircled_follows"],
        })]),
    );
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    state.add_item("stone-marker");
    state.add_item_to_storage("stone-marker", ItemStorageTarget::CurrentRoom, KITCHEN_ID);

    let lines = super::command_effects::handle_actor_command_used(
        &mut state,
        &pack,
        ACTOR_A_ID,
        ACTOR_A_NAME,
        LOUNGE_ID,
        "place-flag",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        &mut Vec::new(),
    )
    .expect("place a flag");

    let placed_at = lines
        .iter()
        .position(|line| line.contains("drives a stone marker"))
        .expect("flag placement narration present");
    let conversion_at = lines
        .iter()
        .position(|line| line.contains("turns toward you"))
        .expect("conversion narration present");
    assert!(
        placed_at < conversion_at,
        "flag placement must precede the conversion, got {lines:?}"
    );
}

#[test]
fn room_observation_annotates_present_actors_by_stance() {
    let mut pack = reducer_test_pack();
    pack.presentation.presentation_text.ally_suffix = " (ally)".to_string();
    pack.presentation.presentation_text.hostile_suffix = " (enemy)".to_string();
    pack.presentation.presentation_text.room_observation =
        "{room_title} {body} {people}".to_string();
    pack.presentation.presentation_text.people = "Here: {people}.".to_string();
    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    state.set_stance(ACTOR_A_ID, ActorStance::Allied);
    state.set_stance(ACTOR_B_ID, ActorStance::Hostile);

    let text = super::observation::render_room_observation(
        &pack,
        &state,
        LOUNGE_ID,
        crate::engine::events::ObservationMode::Summary,
    )
    .expect("room observation");

    assert!(text.contains("Alex (ally)"), "got: {text}");
    assert!(text.contains("Blair (enemy)"), "got: {text}");
}

#[test]
fn drop_and_pick_up_item_move_it_between_inventory_and_room() {
    let mut pack = reducer_test_pack();
    pack.actions.push(ActionDefinition {
        id: "drop-marker".to_string(),
        command: "drop-marker".to_string(),
        target_mode: CommandTargetMode::None,
        effects: vec![CommandEffect::DropItem],
        drop_item: "stone-marker".to_string(),
        event_text: "{actor_name} places the stone marker on the ground.".to_string(),
        ..ActionDefinition::default()
    });
    pack.actions.push(ActionDefinition {
        id: "pick-up-marker".to_string(),
        command: "pick-up-marker".to_string(),
        target_mode: CommandTargetMode::None,
        effects: vec![CommandEffect::PickUpItem],
        drop_item: "stone-marker".to_string(),
        event_text: "{actor_name} picks up the stone marker.".to_string(),
        ..ActionDefinition::default()
    });
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    state.add_item("stone-marker");

    super::command_effects::handle_actor_command_used(
        &mut state,
        &pack,
        ACTOR_A_ID,
        ACTOR_A_NAME,
        LOUNGE_ID,
        "drop-marker",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        &mut Vec::new(),
    )
    .expect("drop marker");

    assert!(!state.has_item("stone-marker"));
    assert_eq!(
        state.loose_room_items(LOUNGE_ID),
        vec![("stone-marker".to_string(), 1)]
    );

    super::command_effects::handle_actor_command_used(
        &mut state,
        &pack,
        ACTOR_A_ID,
        ACTOR_A_NAME,
        LOUNGE_ID,
        "pick-up-marker",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        &mut Vec::new(),
    )
    .expect("pick up marker");

    assert!(state.has_item("stone-marker"));
    assert!(state.loose_room_items(LOUNGE_ID).is_empty());
}
