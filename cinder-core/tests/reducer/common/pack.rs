//! Shared reducer integration-test fixtures.

use cinder_core::content::loader::load_pack_from_dir;
use cinder_core::content::types::{
    ActionDefinition, ActorDefinition, ActorPromptContext, CommandEffect, CommandInputMode,
    CommandTargetMode, ContentPack, ContentSettingsDefinition, OpeningDefinition,
    PresentationDefinition, RoomDefinition, RoomExitDefinition, RoomFeatureDefinition,
    StatDefinition, StatsDefinition,
};
use serde_json::json;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub const ACTOR_A_ID: &str = "alex";
pub const ACTOR_A_NAME: &str = "Alex";
pub const ACTOR_B_ID: &str = "blair";
pub const ACTOR_B_NAME: &str = "Blair";
pub const ACTOR_C_ID: &str = "casey";
pub const ACTOR_C_NAME: &str = "Casey";
pub const LOUNGE_ID: &str = "lounge";
pub const LOUNGE_TITLE: &str = "Lounge";
pub const KITCHEN_ID: &str = "kitchen";
pub const KITCHEN_TITLE: &str = "Kitchen";
pub const SOFA_ID: &str = "sofa";
pub const SOFA_LABEL: &str = "long sofa";

pub fn reducer_test_pack() -> ContentPack {
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

pub fn reducer_test_presentation() -> PresentationDefinition {
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

pub fn reducer_test_stats() -> StatsDefinition {
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

pub fn test_actor(id: &str, name: &str, room_id: &str) -> ActorDefinition {
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
        attack_kind: String::new(),
        resistances: BTreeMap::new(),
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

pub fn effect_hook(effects: Vec<serde_json::Value>) -> serde_json::Value {
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

pub fn rebuild_test_pack_indexes(pack: &mut ContentPack) {
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

pub fn equipment_test_pack() -> ContentPack {
    let mut pack = reducer_test_pack();
    pack.settings.combat = cinder_core::content::types::CombatSettingsDefinition {
        player_actor_id: ACTOR_A_ID.to_string(),
        health_stat_id: "stamina".to_string(),
        attack_stat_id: "confidence".to_string(),
        defense_stat_id: "hunger".to_string(),
        ..cinder_core::content::types::CombatSettingsDefinition::default()
    };
    pack.settings.equipment_slots = ["weapon".to_string()].into_iter().collect();
    pack.items
        .push(cinder_core::content::types::ItemDefinition {
            id: "iron-chisel".to_string(),
            label: "iron chisel".to_string(),
            description: "A chisel with a worn grip.".to_string(),
            kind: cinder_core::content::types::ItemKind::Weapon,
            equip_slot: "weapon".to_string(),
            stat_bonuses: BTreeMap::from([("confidence".to_string(), 2)]),
            use_hook: String::new(),
            equip_hook: String::new(),
            look_description: String::new(),
            trace_mark: false,
        });
    pack.items
        .push(cinder_core::content::types::ItemDefinition {
            id: "steel-chisel".to_string(),
            label: "steel chisel".to_string(),
            description: "A finer chisel.".to_string(),
            kind: cinder_core::content::types::ItemKind::Weapon,
            equip_slot: "weapon".to_string(),
            stat_bonuses: BTreeMap::from([("confidence".to_string(), 4)]),
            use_hook: String::new(),
            equip_hook: String::new(),
            look_description: String::new(),
            trace_mark: false,
        });
    pack.items
        .push(cinder_core::content::types::ItemDefinition {
            id: "herb-salve".to_string(),
            label: "herb salve".to_string(),
            description: "A fragrant paste.".to_string(),
            kind: cinder_core::content::types::ItemKind::Potion,
            equip_slot: String::new(),
            stat_bonuses: BTreeMap::new(),
            use_hook: "item.salve_used".to_string(),
            equip_hook: String::new(),
            look_description: String::new(),
            trace_mark: false,
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

pub fn minimal_test_pack() -> ContentPack {
    let base = TestPackDir::new();
    let locale_dir = base.path().join("locales").join("en");
    fs::create_dir_all(&locale_dir).expect("create locale dir");
    fs::write(base.path().join("settings.json"), "{}").expect("write settings");
    fs::write(
        base.path().join("beat_objectives.json"),
        r#"{ "objectives": [] }"#,
    )
    .expect("write rule objectives");
    fs::write(locale_dir.join("ui.json"), "{}").expect("write ui");
    fs::write(locale_dir.join("system.json"), minimal_system_text_json()).expect("write system");
    fs::write(locale_dir.join("opening.json"), OPENING_JSON).expect("write opening");
    fs::write(locale_dir.join("rooms.json"), ROOMS_JSON).expect("write rooms");
    fs::write(locale_dir.join("actors.json"), ACTORS_JSON).expect("write actors");
    fs::write(base.path().join("stats.json"), STATS_JSON).expect("write stats");
    load_pack_from_dir(base.path()).expect("load synthetic pack")
}

struct TestPackDir {
    path: PathBuf,
}

impl TestPackDir {
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);

        let unique = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-data")
            .join(format!("reducer-pack-{}-{unique}", std::process::id()));
        fs::create_dir_all(&path).expect("create reducer test pack directory");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestPackDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

const OPENING_JSON: &str = r#"{
  "id": "opening", "title": "Test Opening", "start_room_id": "lounge",
  "start_time_minutes": 1080, "intro_text": "Intro", "help_text": "Help"
}"#;
const ROOMS_JSON: &str = r#"[
 {"id":"lounge","title":"Lounge","summary":"A shared lounge.","inspect_text":"A shared lounge.","features":[],"exits":[{"room_id":"kitchen","label":"Kitchen","aliases":["kitchen"]}]},
 {"id":"kitchen","title":"Kitchen","summary":"A warm kitchen.","inspect_text":"A warm kitchen.","features":[],"exits":[{"room_id":"lounge","label":"Lounge","aliases":["lounge"]}]}
]"#;
const ACTORS_JSON: &str = r#"[
 {"id":"blair","name":"Blair","room_id":"lounge","initial_stats":{"confidence":3,"stamina":6,"hunger":5},"prompt_context":{}},
 {"id":"casey","name":"Casey","room_id":"lounge","initial_stats":{"confidence":7,"stamina":7,"hunger":4},"prompt_context":{}}
]"#;
const STATS_JSON: &str = r#"{
 "actor":{"hunger":{"default":0},"stamina":{"default":0},"confidence":{"default":0}},
 "pair":{"safety":{"default":0},"attraction":{"default":0},"connection":{"default":0}}
}"#;
fn minimal_system_text_json() -> &'static str {
    r#"{
 "dialogue_system_prompt":"","dialogue_section_character":"","dialogue_section_setting":"","dialogue_section_current_beat":"","dialogue_section_subtext":"","dialogue_section_recent_memory":"","dialogue_latest_line_label":"","dialogue_section_response":"","dialogue_no_direct_question":"","dialogue_no_character_facts":"","dialogue_no_setting_facts":"","dialogue_no_current_beat_facts":"","dialogue_no_subtext_facts":"","dialogue_no_recent_memory":"","dialogue_response_fallback":"","menu_intent_system_prompt":"","menu_section_title":"","menu_id_label":"","menu_offered_by_label":"","menu_intent_guidance_label":"","menu_available_options_label":"","menu_section_setting":"","menu_section_current_beat":"","menu_section_recent_memory":"","menu_latest_line_label":"","menu_decision_label":"","menu_no_direct_request":"","menu_no_authored_options":"","menu_decision_instruction":"","prompt_time_note":"","prompt_current_room_note":"","prompt_visible_features_note":"","prompt_people_here_note":"","prompt_exits_note":"","prompt_current_speaker_note":"","prompt_shared_room_note":"","prompt_latest_words_note":"","prompt_address_other_person_note":""
}"#
}
