use super::beat_advance::{advance_conditions_met, evaluate_advance_condition};
use super::*;
use crate::content::loader::load_default_pack;
use crate::content::types::{
    ActorDefinition, ActorPromptContext, CommandDefinition, CommandEffect, CommandInputMode,
    CommandTargetMode, CommandsDefinition, ContentPack, ContentSettingsDefinition,
    OpeningDefinition, PresentationDefinition, RoomDefinition, RoomExitDefinition,
    RoomFeatureDefinition, StatDefinition, StatsDefinition,
};
use crate::engine::state::ConversationMemoryKind;
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
    let mut pack = load_default_pack().expect("load base pack");
    pack.settings = ContentSettingsDefinition {
        tick_minutes_per_turn: 1,
        speech_stamina_cost_floor: 0,
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
    pack.affordances = Default::default();
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
            }],
        },
    ];
    pack.actors = vec![
        test_actor(ACTOR_A_ID, ACTOR_A_NAME, LOUNGE_ID),
        test_actor(ACTOR_B_ID, ACTOR_B_NAME, LOUNGE_ID),
        test_actor(ACTOR_C_ID, ACTOR_C_NAME, KITCHEN_ID),
    ];
    pack.stats = reducer_test_stats();
    pack.commands = CommandsDefinition {
        actions: vec![
            CommandDefinition {
                id: "act".to_string(),
                command: "act".to_string(),
                input_mode: CommandInputMode::FreeformText,
                effects: vec![CommandEffect::RememberInRoom],
                ..test_command()
            },
            CommandDefinition {
                id: "hug".to_string(),
                command: "hug".to_string(),
                target_mode: CommandTargetMode::Actor,
                hook_id: "actor.hugged".to_string(),
                event_text: "{actor_name} hugs {target_actor_name}.".to_string(),
                ..test_command()
            },
            CommandDefinition {
                id: "rest".to_string(),
                command: "rest".to_string(),
                target_mode: CommandTargetMode::ContextLabel,
                hook_id: "actor.rested".to_string(),
                event_text: "{actor_name} takes a quiet moment to rest on the {context_label}."
                    .to_string(),
                ..test_command()
            },
            CommandDefinition {
                id: "move".to_string(),
                command: "move".to_string(),
                target_mode: CommandTargetMode::Room,
                effects: vec![CommandEffect::MoveActor],
                event_text: "{actor_name} heads to the {target_room_title}.".to_string(),
                ..test_command()
            },
        ],
    };
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
    presentation.presentation_text.session_ended = "Session ended.".to_string();
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
        prompt_context: ActorPromptContext {
            character_notes: vec![],
            subtext_notes: vec![],
            response_notes: vec![],
            behavior_examples: vec![],
        },
        movement_rules: None,
    }
}

fn test_command() -> CommandDefinition {
    CommandDefinition {
        id: String::new(),
        command: String::new(),
        group: String::new(),
        player_enabled: false,
        player_phrases: vec![],
        outcome_mode: Default::default(),
        input_mode: Default::default(),
        target_mode: Default::default(),
        consumable_kind: None,
        effects: vec![],
        hook_id: String::new(),
        event_text: String::new(),
        content_event: None,
        player_command: None,
        allowed_rooms: vec![],
        creates_item: None,
        consumes_item: None,
        requires_any: vec![],
        consumes_any: vec![],
        available_during: vec![],
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
    pack.command_index = pack
        .commands
        .actions
        .iter()
        .enumerate()
        .map(|(index, command)| (command.id.clone(), index))
        .collect::<HashMap<_, _>>();
    pack.affordance_index = HashMap::new();
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

    assert_eq!(state.actor_stat_u32(ACTOR_A_ID, "hunger"), starting_hunger + 1);
    assert_eq!(state.actor_stat_u32(ACTOR_A_ID, "stamina"), starting_stamina);
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
