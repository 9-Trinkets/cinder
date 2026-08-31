use super::beat_advance::{advance_conditions_met, evaluate_advance_condition};
use super::*;
use crate::content::types::{
    ActionDefinition, ActionItemCreation, ActionItemStorageTarget, ActorDefinition,
    ActorPromptContext, AdvanceCondition, AdvanceSignal, BeatDefinition, BeatsDefinition,
    CombatSettingsDefinition, CommandEffect, CommandInputMode, CommandTargetMode, ContentPack,
    ContentSettingsDefinition, ItemDefinition, ItemStorageTarget, OpeningDefinition,
    PresentationDefinition, RoomDefinition, RoomDescriptionOverride, RoomExitDefinition,
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
                requires_story_var: String::new(),
            }],
            descriptions: vec![],
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
                requires_story_var: String::new(),
            }],
            descriptions: vec![],
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
        tags: vec![],
        inspect_text: format!("{name} looks thoughtful."),
        required_consumable_tags: vec![],
        attackable: false,
        guard: false,
        drops: BTreeMap::new(),
        xp_drop: 0,
        attack_interval_minutes: None,
        initial_hostile: false,
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
            .any(|line| line.text == "Alex sits on the couch.")
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

    assert!(
        output
            .lines
            .iter()
            .any(|line| line.text == "Alex hugs Casey.")
    );
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
            .any(|line| line.text == "Alex takes a quiet moment to rest on the long sofa.")
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
            .any(|line| line.text == "Alex (to Blair): Hey.")
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
            .any(|line| line.text == "Alex comes in from the Lounge.")
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
            .any(|line| line.text == "Alex pauses to take in the Lounge more carefully.")
    );
    assert!(
        output
            .lines
            .iter()
            .any(|line| line.text == "Alex studies Blair more closely.")
    );
    assert!(
        output
            .lines
            .iter()
            .any(|line| line.text == "Alex studies the long sofa.")
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
        ..ItemDefinition::default()
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
        ..ItemDefinition::default()
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
            .any(|line| line.text == "You have tea ready.")
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
            ..ItemDefinition::default()
        },
        ItemDefinition {
            id: "vegetable-stir-fry".to_string(),
            label: "vegetable stir-fry".to_string(),
            description: "Stir-fry.".to_string(),
            ..ItemDefinition::default()
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
            craftable_items: Vec::new(),
            craftable_item_gates: BTreeMap::new(),
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
        output.lines.iter().any(|line| line.text == defeat_text),
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
        item_id: "stone-marker".to_string(),
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
    // A golem in the lounge converts via the `actor.surrounded` hook once its
    // only neighbor (the kitchen) holds a stone marker.
    pack.actors.push(ActorDefinition {
        id: "golem".to_string(),
        name: "dark golem".to_string(),
        room_id: LOUNGE_ID.to_string(),
        ..test_actor("golem", "dark golem", LOUNGE_ID)
    });
    pack.hooks.insert(
        "actor.surrounded".to_string(),
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
        .position(|line| line.text.contains("drives a stone marker"))
        .expect("flag placement narration present");
    let conversion_at = lines
        .iter()
        .position(|line| line.text.contains("turns toward you"))
        .expect("conversion narration present");
    assert!(
        placed_at < conversion_at,
        "flag placement must precede the conversion, got {lines:?}"
    );
}

#[test]
fn creating_an_item_in_a_room_can_complete_an_encirclement() {
    let mut pack = reducer_test_pack();
    pack.actions.push(ActionDefinition {
        id: "trace".to_string(),
        command: "trace".to_string(),
        target_mode: CommandTargetMode::None,
        item_creation: Some(ActionItemCreation {
            creates_item: "charm-sigil".to_string(),
            storage: ActionItemStorageTarget::CurrentRoom,
            ..ActionItemCreation::default()
        }),
        event_text: "{actor_name} draws chalk across the floor.".to_string(),
        ..ActionDefinition::default()
    });
    pack.messages.insert(
        "conversion.encircled".to_string(),
        "The {actor} turns toward you, no longer hostile.".to_string(),
    );
    // A golem in the lounge is encircled once its only neighbor (the kitchen)
    // holds a chalk marking created by the trace action.
    pack.actors.push(ActorDefinition {
        id: "golem".to_string(),
        name: "dark golem".to_string(),
        room_id: LOUNGE_ID.to_string(),
        ..test_actor("golem", "dark golem", LOUNGE_ID)
    });
    pack.hooks.insert(
        "actor.surrounded".to_string(),
        effect_hook(vec![json!({
            "kind": "convert_actor_to_ally",
            "actor_id": "$input.actor_id",
            "follows_player": true,
            "messages": ["conversion.encircled"],
        })]),
    );
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.current_room_id = KITCHEN_ID.to_string();

    let lines = super::command_effects::handle_actor_command_used(
        &mut state,
        &pack,
        ACTOR_A_ID,
        ACTOR_A_NAME,
        KITCHEN_ID,
        "trace",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        &mut Vec::new(),
    )
    .expect("trace");

    assert!(state.has_item_in_storage("charm-sigil", ItemStorageTarget::CurrentRoom, KITCHEN_ID));
    assert_eq!(state.stance("golem"), ActorStance::Allied);
    assert!(
        lines
            .iter()
            .any(|line| line.text.contains("turns toward you")),
        "got: {lines:?}"
    );
}

#[test]
fn trace_craftable_is_gated_by_its_story_variable() {
    let mut pack = reducer_test_pack();
    pack.items.push(ItemDefinition {
        id: "drain-sigil".to_string(),
        label: "drain sigil".to_string(),
        description: "A spiral chalk mark.".to_string(),
        ..ItemDefinition::default()
    });
    pack.actions.push(ActionDefinition {
        id: "trace".to_string(),
        command: "trace".to_string(),
        target_mode: CommandTargetMode::None,
        item_creation: Some(ActionItemCreation {
            creates_item: "charm-sigil".to_string(),
            craftable_items: vec!["charm-sigil".to_string(), "drain-sigil".to_string()],
            craftable_item_gates: BTreeMap::from([("drain-sigil".to_string(), "knows_drain".to_string())]),
            storage: ActionItemStorageTarget::CurrentRoom,
            ..ActionItemCreation::default()
        }),
        event_text: "{actor_name} draws chalk across the floor.".to_string(),
        ..ActionDefinition::default()
    });
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.current_room_id = KITCHEN_ID.to_string();

    // Without the story var, choosing the drain sigil falls back to the
    // unlocked charm sigil; the locked craftable is never created.
    super::command_effects::handle_actor_command_used(
        &mut state,
        &pack,
        ACTOR_A_ID,
        ACTOR_A_NAME,
        KITCHEN_ID,
        "trace",
        None,
        None,
        None,
        None,
        None,
        None,
        Some("drain-sigil"),
        &mut Vec::new(),
    )
    .expect("trace");
    assert!(state.has_item_in_storage("charm-sigil", ItemStorageTarget::CurrentRoom, KITCHEN_ID));
    assert!(!state.has_item_in_storage("drain-sigil", ItemStorageTarget::CurrentRoom, KITCHEN_ID));

    // Once the scroll is read (knows_drain set), tracing the drain sigil works.
    state.story_vars.set_unchecked("knows_drain", "true");
    super::command_effects::handle_actor_command_used(
        &mut state,
        &pack,
        ACTOR_A_ID,
        ACTOR_A_NAME,
        KITCHEN_ID,
        "trace",
        None,
        None,
        None,
        None,
        None,
        None,
        Some("drain-sigil"),
        &mut Vec::new(),
    )
    .expect("trace drain-sigil");
    assert!(state.has_item_in_storage("drain-sigil", ItemStorageTarget::CurrentRoom, KITCHEN_ID));
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
    .expect("room observation")
    .to_text();

    assert!(text.contains("Alex (ally)"), "got: {text}");
    assert!(text.contains("Blair (enemy)"), "got: {text}");
}

#[test]
fn room_observation_lists_loose_items_on_the_ground() {
    let mut pack = reducer_test_pack();
    pack.presentation.presentation_text.room_observation =
        "{room_title} {body} {items} {people}".to_string();
    pack.presentation.presentation_text.loose_items = "On the ground: {items}.".to_string();
    pack.presentation.presentation_text.people = "Here: {people}.".to_string();
    pack.items.push(ItemDefinition {
        id: "stone-marker".to_string(),
        label: "stone marker".to_string(),
        description: "A marker.".to_string(),
        ..ItemDefinition::default()
    });
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    state.add_item_to_storage("stone-marker", ItemStorageTarget::CurrentRoom, LOUNGE_ID);

    let text = super::observation::render_room_observation(
        &pack,
        &state,
        LOUNGE_ID,
        crate::engine::events::ObservationMode::Summary,
    )
    .expect("room observation")
    .to_text();

    assert!(text.contains("On the ground: stone marker."), "got: {text}");

    // A room without loose items omits the line entirely.
    state.current_room_id = KITCHEN_ID.to_string();
    let empty = super::observation::render_room_observation(
        &pack,
        &state,
        KITCHEN_ID,
        crate::engine::events::ObservationMode::Summary,
    )
    .expect("kitchen observation")
    .to_text();
    assert!(!empty.contains("On the ground"), "got: {empty}");
}

#[test]
fn drop_and_pick_up_item_move_it_between_inventory_and_room() {
    let mut pack = reducer_test_pack();
    pack.actions.push(ActionDefinition {
        id: "drop-marker".to_string(),
        command: "drop-marker".to_string(),
        target_mode: CommandTargetMode::None,
        effects: vec![CommandEffect::DropItem],
        item_id: "stone-marker".to_string(),
        event_text: "{actor_name} places the stone marker on the ground.".to_string(),
        ..ActionDefinition::default()
    });
    pack.actions.push(ActionDefinition {
        id: "pick-up-marker".to_string(),
        command: "pick-up-marker".to_string(),
        target_mode: CommandTargetMode::None,
        effects: vec![CommandEffect::PickUpItem],
        item_id: "stone-marker".to_string(),
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

#[test]
fn equipping_an_item_with_equip_hook_converts_surviving_tagged_actors() {
    let mut pack = reducer_test_pack();
    pack.settings.combat = crate::content::types::CombatSettingsDefinition {
        player_actor_id: ACTOR_A_ID.to_string(),
        health_stat_id: "stamina".to_string(),
        attack_stat_id: "confidence".to_string(),
        defense_stat_id: "hunger".to_string(),
        ..crate::content::types::CombatSettingsDefinition::default()
    };
    pack.settings.equipment_slots = ["trinket".to_string()].into_iter().collect();
    pack.items.push(crate::content::types::ItemDefinition {
        id: "warden-core".to_string(),
        label: "warden core".to_string(),
        description: "A warm stone heart.".to_string(),
        kind: crate::content::types::ItemKind::Trinket,
        equip_slot: "trinket".to_string(),
        stat_bonuses: BTreeMap::new(),
        use_hook: String::new(),
        equip_hook: "item.core_equipped".to_string(),
        look_description: String::new(),
    });
    pack.messages.insert(
        "conversion.core".to_string(),
        "The {actor} bows its head and falls in behind you.".to_string(),
    );
    pack.hooks.insert(
        "item.core_equipped".to_string(),
        effect_hook(vec![json!({
            "kind": "convert_allies_by_tag",
            "tag": "golem",
            "follows_player": true,
            "messages": ["conversion.core"],
        })]),
    );
    pack.actions.push(ActionDefinition {
        id: "equip-core".to_string(),
        command: "equip-core".to_string(),
        target_mode: CommandTargetMode::None,
        effects: vec![CommandEffect::EquipItem],
        item_id: "warden-core".to_string(),
        event_text: "{actor_name} grasps the warden core.".to_string(),
        ..ActionDefinition::default()
    });
    // Two living golems, one already allied (encircled earlier), one dead.
    let mut golem_living = test_actor("statue-live", "granite statue", KITCHEN_ID);
    golem_living.tags = vec!["golem".to_string()];
    golem_living.initial_stats = BTreeMap::from([("stamina".to_string(), 8)]);
    let mut golem_dead = test_actor("statue-dead", "dust statue", KITCHEN_ID);
    golem_dead.tags = vec!["golem".to_string()];
    golem_dead.initial_stats = BTreeMap::from([("stamina".to_string(), 0)]);
    let mut golem_allied = test_actor("statue-ally", "charmed statue", KITCHEN_ID);
    golem_allied.tags = vec!["golem".to_string()];
    golem_allied.initial_stats = BTreeMap::from([("stamina".to_string(), 8)]);
    pack.actors.extend([golem_living, golem_dead, golem_allied]);
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    state.set_stance("statue-ally", ActorStance::Allied);
    state.add_item("warden-core");

    let lines = super::command_effects::handle_actor_command_used(
        &mut state,
        &pack,
        ACTOR_A_ID,
        ACTOR_A_NAME,
        LOUNGE_ID,
        "equip-core",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        &mut Vec::new(),
    )
    .unwrap_or_default();

    // Living golem converted to an ally follower; dead and already-allied skipped.
    assert_eq!(state.stance("statue-live"), ActorStance::Allied);
    assert!(state.relationship("statue-live").follows_player);
    assert_eq!(state.stance("statue-dead"), ActorStance::Neutral);
    assert_eq!(state.stance("statue-ally"), ActorStance::Allied);
    // Already-allied actors are left untouched (not re-followed by the hook).
    assert!(!state.relationship("statue-ally").follows_player);
    assert!(
        lines
            .iter()
            .any(|line| line.text.contains("granite statue")),
        "got: {lines:?}"
    );
    assert!(
        !lines.iter().any(|line| line.text.contains("dust statue")),
        "got: {lines:?}"
    );
}

fn equipment_test_pack() -> ContentPack {
    let mut pack = reducer_test_pack();
    pack.settings.combat = crate::content::types::CombatSettingsDefinition {
        player_actor_id: ACTOR_A_ID.to_string(),
        health_stat_id: "stamina".to_string(),
        attack_stat_id: "confidence".to_string(),
        defense_stat_id: "hunger".to_string(),
        ..crate::content::types::CombatSettingsDefinition::default()
    };
    pack.settings.equipment_slots = ["weapon".to_string()].into_iter().collect();
    pack.items.push(crate::content::types::ItemDefinition {
        id: "iron-chisel".to_string(),
        label: "iron chisel".to_string(),
        description: "A chisel with a worn grip.".to_string(),
        kind: crate::content::types::ItemKind::Weapon,
        equip_slot: "weapon".to_string(),
        stat_bonuses: BTreeMap::from([("confidence".to_string(), 2)]),
        use_hook: String::new(),
        equip_hook: String::new(),
        look_description: String::new(),
    });
    pack.items.push(crate::content::types::ItemDefinition {
        id: "steel-chisel".to_string(),
        label: "steel chisel".to_string(),
        description: "A finer chisel.".to_string(),
        kind: crate::content::types::ItemKind::Weapon,
        equip_slot: "weapon".to_string(),
        stat_bonuses: BTreeMap::from([("confidence".to_string(), 4)]),
        use_hook: String::new(),
        equip_hook: String::new(),
        look_description: String::new(),
    });
    pack.items.push(crate::content::types::ItemDefinition {
        id: "herb-salve".to_string(),
        label: "herb salve".to_string(),
        description: "A fragrant paste.".to_string(),
        kind: crate::content::types::ItemKind::Potion,
        equip_slot: String::new(),
        stat_bonuses: BTreeMap::new(),
        use_hook: "item.salve_used".to_string(),
        equip_hook: String::new(),
        look_description: String::new(),
    });
    pack.hooks.insert(
        "item.salve_used".to_string(),
        effect_hook(vec![json!({
            "kind": "adjust_actor_stat",
            "actor_id": "$input.actor_id",
            "stat": "stamina",
            "delta": 2
        })]),
    );
    for id in ["equip-chisel", "unequip-chisel", "use-salve"] {
        let (effects, item) = match id {
            "equip-chisel" => (vec![CommandEffect::EquipItem], "iron-chisel"),
            "unequip-chisel" => (vec![CommandEffect::UnequipItem], "iron-chisel"),
            _ => (vec![CommandEffect::UseItem], "herb-salve"),
        };
        pack.actions.push(ActionDefinition {
            id: id.to_string(),
            command: id.to_string(),
            target_mode: CommandTargetMode::None,
            effects,
            item_id: item.to_string(),
            event_text: format!("{{actor_name}} uses the {item}."),
            ..ActionDefinition::default()
        });
    }
    rebuild_test_pack_indexes(&mut pack);
    pack
}

#[test]
fn equip_and_unequip_change_effective_stats_and_inventory() {
    let pack = equipment_test_pack();
    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    // Alex's confidence base.
    let base_attack = state.actor_stat(ACTOR_A_ID, "confidence");
    state.add_item("iron-chisel");

    super::command_effects::handle_actor_command_used(
        &mut state,
        &pack,
        ACTOR_A_ID,
        ACTOR_A_NAME,
        LOUNGE_ID,
        "equip-chisel",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        &mut Vec::new(),
    )
    .expect("equip chisel");

    assert!(!state.has_item("iron-chisel"));
    assert_eq!(state.equipped_item("weapon"), Some("iron-chisel"));
    assert_eq!(
        state.effective_actor_stat(&pack, ACTOR_A_ID, "confidence"),
        base_attack + 2
    );
    // Non-player actors never receive equipment bonuses.
    assert_eq!(
        state.effective_actor_stat(&pack, ACTOR_B_ID, "confidence"),
        state.actor_stat(ACTOR_B_ID, "confidence")
    );

    super::command_effects::handle_actor_command_used(
        &mut state,
        &pack,
        ACTOR_A_ID,
        ACTOR_A_NAME,
        LOUNGE_ID,
        "unequip-chisel",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        &mut Vec::new(),
    )
    .expect("unequip chisel");

    assert!(state.has_item("iron-chisel"));
    assert_eq!(state.equipped_item("weapon"), None);
    assert_eq!(
        state.effective_actor_stat(&pack, ACTOR_A_ID, "confidence"),
        base_attack
    );
}

#[test]
fn equipping_a_second_weapon_replaces_the_first() {
    let mut pack = equipment_test_pack();
    pack.actions.push(ActionDefinition {
        id: "equip-steel".to_string(),
        command: "equip-steel".to_string(),
        target_mode: CommandTargetMode::None,
        effects: vec![CommandEffect::EquipItem],
        item_id: "steel-chisel".to_string(),
        event_text: "{actor_name} readies the steel chisel.".to_string(),
        ..ActionDefinition::default()
    });
    rebuild_test_pack_indexes(&mut pack);
    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    state.add_item("iron-chisel");
    state.add_item("steel-chisel");
    let base_attack = state.actor_stat(ACTOR_A_ID, "confidence");

    super::command_effects::handle_actor_command_used(
        &mut state,
        &pack,
        ACTOR_A_ID,
        ACTOR_A_NAME,
        LOUNGE_ID,
        "equip-chisel",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        &mut Vec::new(),
    )
    .expect("equip iron chisel");
    super::command_effects::handle_actor_command_used(
        &mut state,
        &pack,
        ACTOR_A_ID,
        ACTOR_A_NAME,
        LOUNGE_ID,
        "equip-steel",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        &mut Vec::new(),
    )
    .expect("equip steel chisel");

    assert_eq!(state.equipped_item("weapon"), Some("steel-chisel"));
    assert!(
        state.has_item("iron-chisel"),
        "old weapon returns to inventory"
    );
    assert!(!state.has_item("steel-chisel"));
    assert_eq!(
        state.effective_actor_stat(&pack, ACTOR_A_ID, "confidence"),
        base_attack + 4
    );
}

#[test]
fn cannot_equip_an_item_already_in_its_slot_even_with_spares() {
    let pack = equipment_test_pack();
    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    // Two copies: one equipped, one spare in the inventory.
    state.add_item("iron-chisel");
    state.add_item("iron-chisel");

    super::command_effects::handle_actor_command_used(
        &mut state,
        &pack,
        ACTOR_A_ID,
        ACTOR_A_NAME,
        LOUNGE_ID,
        "equip-chisel",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        &mut Vec::new(),
    )
    .expect("equip first chisel");
    assert_eq!(state.equipped_item("weapon"), Some("iron-chisel"));
    assert_eq!(state.item_count("iron-chisel"), 1);

    // Equipping the same item again is rejected: it already fills the slot.
    let second = super::command_effects::handle_actor_command_used(
        &mut state,
        &pack,
        ACTOR_A_ID,
        ACTOR_A_NAME,
        LOUNGE_ID,
        "equip-chisel",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        &mut Vec::new(),
    );
    assert!(
        second.is_none(),
        "equipping an already-equipped item must be rejected"
    );
    assert_eq!(state.equipped_item("weapon"), Some("iron-chisel"));
    assert_eq!(state.item_count("iron-chisel"), 1);
}

#[test]
fn using_a_potion_consumes_it_and_fires_its_use_hook() {
    let pack = equipment_test_pack();
    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    state.add_item("herb-salve");
    state.add_item("herb-salve");
    let stamina_before = state.actor_stat(ACTOR_A_ID, "stamina");

    super::command_effects::handle_actor_command_used(
        &mut state,
        &pack,
        ACTOR_A_ID,
        ACTOR_A_NAME,
        LOUNGE_ID,
        "use-salve",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        &mut Vec::new(),
    )
    .expect("use salve");

    assert_eq!(state.actor_stat(ACTOR_A_ID, "stamina"), stamina_before + 2);
    assert_eq!(state.item_count("herb-salve"), 1);
}

#[test]
fn defeating_an_actor_scatters_its_drops_into_the_room() {
    let mut pack = equipment_test_pack();
    pack.actions.push(ActionDefinition {
        id: "attack".to_string(),
        command: "attack".to_string(),
        target_mode: CommandTargetMode::Actor,
        effects: vec![CommandEffect::AttackTarget],
        event_text: "{actor_name} strikes {target_actor_name}.".to_string(),
        ..ActionDefinition::default()
    });
    let mut golem = test_actor("golem", "dark golem", LOUNGE_ID);
    golem.attackable = true;
    golem.initial_stats = BTreeMap::from([("stamina".to_string(), 1)]);
    golem.drops = BTreeMap::from([("herb-salve".to_string(), 2)]);
    pack.actors.push(golem);
    pack.items.push(crate::content::types::ItemDefinition {
        id: "stone-marker".to_string(),
        label: "stone marker".to_string(),
        description: "A flat stone.".to_string(),
        ..ItemDefinition::default()
    });
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();

    let lines = super::command_effects::handle_actor_command_used(
        &mut state,
        &pack,
        ACTOR_A_ID,
        ACTOR_A_NAME,
        LOUNGE_ID,
        "attack",
        None,
        Some("golem"),
        Some("dark golem"),
        None,
        None,
        None,
        None,
        &mut Vec::new(),
    )
    .unwrap_or_default();

    assert!(state.actor_is_defeated("golem", "stamina"));
    assert_eq!(
        state.loose_room_items(LOUNGE_ID),
        vec![("herb-salve".to_string(), 2)]
    );
    let transcript = lines
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        transcript.contains("Left behind") && transcript.contains("herb salve"),
        "got: {transcript}"
    );
}

#[test]
fn defeating_an_actor_awards_full_xp_to_every_party_member_with_own_curve() {
    let mut pack = reducer_test_pack();
    pack.settings.combat = CombatSettingsDefinition {
        player_actor_id: ACTOR_A_ID.to_string(),
        health_stat_id: "stamina".to_string(),
        attack_stat_id: "confidence".to_string(),
        defense_stat_id: "hunger".to_string(),
        ..CombatSettingsDefinition::default()
    };
    // The default table levels anyone at 10 XP (L1 -> L2, +5 stamina). Blair
    // gets a per-actor override needing 20 XP, so she does NOT level here.
    pack.levels.default = vec![crate::content::types::LevelDefinition {
        exp_required: 10,
        stat_changes: BTreeMap::from([("stamina".to_string(), 5)]),
        unlocks: vec!["power_slice".to_string()],
    }];
    pack.levels.actors = BTreeMap::from([(
        ACTOR_B_ID.to_string(),
        vec![crate::content::types::LevelDefinition {
            exp_required: 20,
            stat_changes: BTreeMap::from([("stamina".to_string(), 2)]),
            unlocks: vec![],
        }],
    )]);
    pack.actions.push(ActionDefinition {
        id: "attack".to_string(),
        command: "attack".to_string(),
        target_mode: CommandTargetMode::Actor,
        effects: vec![CommandEffect::AttackTarget],
        event_text: "{actor_name} strikes {target_actor_name}.".to_string(),
        ..ActionDefinition::default()
    });
    // A one-hp, 10-xp mob in the lounge.
    let mut goblin = test_actor("goblin", "goblin", LOUNGE_ID);
    goblin.attackable = true;
    goblin.initial_stats = BTreeMap::from([("stamina".to_string(), 1)]);
    goblin.xp_drop = 10;
    pack.actors.push(goblin);
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    state.current_room_id = LOUNGE_ID.to_string();
    state.set_relationship(
        ACTOR_B_ID,
        crate::engine::state::ActorRelationship {
            stance: ActorStance::Allied,
            follows_player: true,
        },
    );

    let lines = super::command_effects::handle_actor_command_used(
        &mut state,
        &pack,
        ACTOR_A_ID,
        ACTOR_A_NAME,
        LOUNGE_ID,
        "attack",
        None,
        Some("goblin"),
        Some("goblin"),
        None,
        None,
        None,
        None,
        &mut Vec::new(),
    )
    .unwrap_or_default();

    // Full 10 XP goes to the player (leveled 1->2 on the default curve, +5
    // stamina) and to follower Blair — who keeps it on her own 20-XP curve,
    // so she stays level 1 with 10/20 and no stat bonus.
    assert!(state.actor_is_defeated("goblin", "stamina"));
    assert_eq!(state.actor_xp(ACTOR_A_ID), 0);
    assert_eq!(state.actor_level(ACTOR_A_ID), 2);
    assert_eq!(state.actor_xp(ACTOR_B_ID), 10);
    assert_eq!(state.actor_level(ACTOR_B_ID), 1);
    // Narration is per actor: only the leveling actor is named with its own
    // new level.
    let transcripts = lines
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>();
    assert!(
        transcripts
            .iter()
            .any(|t| t.contains(ACTOR_A_NAME) && t.contains("Level 2")),
        "expected per-actor narration naming Alex at Level 2, got: {transcripts:?}"
    );
    assert!(
        !transcripts
            .iter()
            .any(|t| t.contains(ACTOR_B_NAME) && t.contains("Level 2")),
        "Blair did not level; she should not be narrated, got: {transcripts:?}"
    );
    let alex_stamina = state.actor_stat(ACTOR_A_ID, "stamina");
    let blair_stamina = state.actor_stat(ACTOR_B_ID, "stamina");
    assert_eq!(
        alex_stamina,
        blair_stamina + 5,
        "Alex should be +5 stamina; Alex={alex_stamina}, Blair={blair_stamina}"
    );
}

#[test]
fn level_reveal_follows_the_declared_room_prefix() {
    let mut pack = minimal_test_pack();
    assert!(
        pack.levels_revealed_for_room("alpha"),
        "empty prefix: levels always visible"
    );
    pack.settings.level_reveal_room_prefix = "down".to_string();
    assert!(
        !pack.levels_revealed_for_room("topside"),
        "upper stays hidden"
    );
    assert!(
        pack.levels_revealed_for_room("down-barrow"),
        "descended board reveals levels"
    );
}

#[test]
fn room_observation_switches_to_defeat_override_description_once_actor_falls() {
    let mut pack = reducer_test_pack();
    pack.presentation.presentation_text.room_observation = "{room_title} {body}".to_string();
    pack.settings.combat = CombatSettingsDefinition {
        player_actor_id: ACTOR_A_ID.to_string(),
        health_stat_id: "stamina".to_string(),
        ..CombatSettingsDefinition::default()
    };
    let kitchen = pack
        .rooms
        .iter_mut()
        .find(|room| room.id == KITCHEN_ID)
        .expect("kitchen room");
    kitchen.descriptions.push(RoomDescriptionOverride {
        actor_defeated: ACTOR_C_ID.to_string(),
        summary: "The quiet kitchen is empty now.".to_string(),
        inspect_text: "No one stands in the quiet kitchen.".to_string(),
    });
    let mut state = WorldState::new(&pack);

    let before = super::observation::render_room_observation(
        &pack,
        &state,
        KITCHEN_ID,
        crate::engine::events::ObservationMode::Summary,
    )
    .expect("room observation")
    .to_text();
    assert!(before.contains("quiet kitchen"), "got: {before}");

    state
        .actor_stats
        .entry(ACTOR_C_ID.to_string())
        .or_default()
        .insert("stamina".to_string(), 0);
    let after = super::observation::render_room_observation(
        &pack,
        &state,
        KITCHEN_ID,
        crate::engine::events::ObservationMode::Summary,
    )
    .expect("room observation")
    .to_text();
    assert!(after.contains("empty now"), "got: {after}");
}
