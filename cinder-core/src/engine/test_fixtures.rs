//! Synthetic content packs for unit tests.
//!
//! Engine tests must never depend on a shipped game pack, so this builds a
//! minimal, self-contained pack from a throwaway temp directory and loads it
//! through the normal loader (which also builds the id indexes tests rely on).

use crate::content::loader::load_pack_from_dir;
use crate::content::types::ContentPack;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// A minimal pack with two rooms (`lounge`, `kitchen`), two actors (`blair`,
/// `casey`), and hunger/stamina/confidence actor stats. Tests override fields
/// they need rather than depending on any shipped pack's content.
pub fn minimal_test_pack() -> ContentPack {
    load_test_pack_with_files(&[])
}

/// Loads the minimal pack after replacing or adding the provided relative
/// files. The on-disk pack is removed once loading finishes.
pub fn load_test_pack_with_files(files: &[(&str, &str)]) -> ContentPack {
    let base = TestDir::new("content-pack");
    for (path, contents) in MINIMAL_PACK_FILES {
        base.write(path, contents);
    }
    for (path, contents) in files {
        base.write(path, contents);
    }
    load_pack_from_dir(base.path()).expect("load synthetic pack")
}

/// Rebuilds lookup indexes after a test mutates pack definitions.
pub fn rebuild_test_pack_indexes(pack: &mut ContentPack) {
    pack.room_index = pack
        .rooms
        .iter()
        .enumerate()
        .map(|(index, room)| (room.id.clone(), index))
        .collect();
    pack.actor_index = pack
        .actors
        .iter()
        .enumerate()
        .map(|(index, actor)| (actor.id.clone(), index))
        .collect();
    pack.action_index = pack
        .actions
        .iter()
        .enumerate()
        .map(|(index, action)| (action.id.clone(), index))
        .collect();
}

/// A unique test directory under this crate's `target/` tree.
pub struct TestDir {
    path: PathBuf,
}

impl TestDir {
    pub fn new(label: &str) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);

        let unique = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-data")
            .join(format!("{label}-{}-{unique}", std::process::id()));
        fs::create_dir_all(&path).expect("create test directory");
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn write(&self, relative_path: &str, contents: &str) {
        let path = self.path.join(relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create test file parent");
        }
        fs::write(path, contents).expect("write test file");
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

const MINIMAL_PACK_FILES: &[(&str, &str)] = &[
    ("settings.json", "{}"),
    ("rule_bundles.json", r#"{ "bundles": [] }"#),
    ("locales/en/ui.json", "{}"),
    ("locales/en/system.json", minimal_system_text_json()),
    ("locales/en/opening.json", OPENING_JSON),
    ("locales/en/rooms.json", ROOMS_JSON),
    ("locales/en/actors.json", ACTORS_JSON),
    ("stats.json", STATS_JSON),
];

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
pub const fn minimal_system_text_json() -> &'static str {
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
