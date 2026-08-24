//! Synthetic content packs for unit tests.
//!
//! Engine tests must never depend on a shipped game pack, so this builds a
//! minimal, self-contained pack from a throwaway temp directory and loads it
//! through the normal loader (which also builds the id indexes tests rely on).

use crate::content::loader::load_pack_from_dir;
use crate::content::types::ContentPack;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

/// A minimal pack with two rooms (`lounge`, `kitchen`), two actors (`blair`,
/// `casey`), and hunger/stamina/confidence actor stats. Tests override fields
/// they need rather than depending on any shipped pack's content.
pub fn minimal_test_pack() -> ContentPack {
    let base = temp_pack_dir();
    let locale_dir = base.join("locales").join("en");
    fs::create_dir_all(&locale_dir).expect("create locale dir");
    fs::write(base.join("settings.json"), "{}").expect("write settings");
    fs::write(base.join("rule_bundles.json"), r#"{ "bundles": [] }"#).expect("write rule bundles");
    fs::write(locale_dir.join("ui.json"), "{}").expect("write ui");
    fs::write(locale_dir.join("system.json"), minimal_system_text_json()).expect("write system");
    fs::write(locale_dir.join("opening.json"), OPENING_JSON).expect("write opening");
    fs::write(locale_dir.join("rooms.json"), ROOMS_JSON).expect("write rooms");
    fs::write(locale_dir.join("actors.json"), ACTORS_JSON).expect("write actors");
    fs::write(base.join("stats.json"), STATS_JSON).expect("write stats");
    load_pack_from_dir(&base).expect("load synthetic pack")
}

fn temp_pack_dir() -> std::path::PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("cinder-test-pack-{unique}"))
}

const OPENING_JSON: &str = r#"{
  "id": "opening",
  "title": "Test Opening",
  "start_room_id": "lounge",
  "start_time_minutes": 1080,
  "intro_text": "Intro",
  "help_text": "Help"
}"#;

const ROOMS_JSON: &str = r#"[
  {
    "id": "lounge",
    "title": "Lounge",
    "summary": "A shared lounge.",
    "inspect_text": "A shared lounge.",
    "features": [],
    "exits": [
      { "room_id": "kitchen", "label": "Kitchen", "aliases": ["kitchen"] }
    ]
  },
  {
    "id": "kitchen",
    "title": "Kitchen",
    "summary": "A warm kitchen.",
    "inspect_text": "A warm kitchen.",
    "features": [],
    "exits": [
      { "room_id": "lounge", "label": "Lounge", "aliases": ["lounge"] }
    ]
  }
]"#;

const ACTORS_JSON: &str = r#"[
  {
    "id": "blair",
    "name": "Blair",
    "room_id": "lounge",
    "initial_stats": { "confidence": 3, "stamina": 6, "hunger": 5 },
    "prompt_context": {}
  },
  {
    "id": "casey",
    "name": "Casey",
    "room_id": "lounge",
    "initial_stats": { "confidence": 7, "stamina": 7, "hunger": 4 },
    "prompt_context": {}
  }
]"#;

const STATS_JSON: &str = r#"{
  "actor": {
    "hunger": { "default": 0 },
    "stamina": { "default": 0 },
    "confidence": { "default": 0 }
  },
  "pair": {
    "safety": { "default": 0 },
    "attraction": { "default": 0 },
    "connection": { "default": 0 }
  }
}"#;

/// Minimal `system.json` that satisfies the loader's required-field checks.
pub fn minimal_system_text_json() -> &'static str {
    r#"{
  "dialogue_system_prompt": "",
  "dialogue_section_character": "",
  "dialogue_section_setting": "",
  "dialogue_section_current_beat": "",
  "dialogue_section_subtext": "",
  "dialogue_section_recent_memory": "",
  "dialogue_latest_line_label": "",
  "dialogue_section_response": "",
  "dialogue_no_direct_question": "",
  "dialogue_no_character_facts": "",
  "dialogue_no_setting_facts": "",
  "dialogue_no_current_beat_facts": "",
  "dialogue_no_subtext_facts": "",
  "dialogue_no_recent_memory": "",
  "dialogue_response_fallback": "",
  "menu_intent_system_prompt": "",
  "menu_section_title": "",
  "menu_id_label": "",
  "menu_offered_by_label": "",
  "menu_intent_guidance_label": "",
  "menu_available_options_label": "",
  "menu_section_setting": "",
  "menu_section_current_beat": "",
  "menu_section_recent_memory": "",
  "menu_latest_line_label": "",
  "menu_decision_label": "",
  "menu_no_direct_request": "",
  "menu_no_authored_options": "",
  "menu_decision_instruction": "",
  "prompt_time_note": "",
  "prompt_current_room_note": "",
  "prompt_visible_features_note": "",
  "prompt_people_here_note": "",
  "prompt_exits_note": "",
  "prompt_current_speaker_note": "",
  "prompt_shared_room_note": "",
  "prompt_latest_words_note": "",
  "prompt_address_other_person_note": ""
}"#
}
