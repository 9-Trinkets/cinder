use std::fs;
use std::path::Path;

pub fn scaffold_pack(content_dir: &Path, pack_name: &str) -> std::io::Result<()> {
    let pack_path = content_dir.join(pack_name);
    let en_path = pack_path.join("locales").join("en");

    fs::create_dir_all(&en_path)?;

    // Root JSON files
    fs::write(
        pack_path.join("settings.json"),
        r#"{
  "combat": {
    "player_actor_id": "player",
    "turn_duration_seconds": 6
  },
  "movement": {
    "defaults": {
      "strike": {
        "kind": "attack"
      },
      "hold": {
        "kind": "rest"
      }
    }
  }
}
"#,
    )?;

    fs::write(pack_path.join("actions.json"), "{\n  \"actions\": []\n}\n")?;
    fs::write(pack_path.join("beats.json"), "{\n  \"stages\": []\n}\n")?;
    fs::write(pack_path.join("beat_objectives.json"), "{\n  \"objectives\": []\n}\n")?;
    fs::write(pack_path.join("hooks.json"), "{\n  \"rules\": {}\n}\n")?;
    fs::write(pack_path.join("items.json"), "[]\n")?;

    // Locale JSON files
    fs::write(
        en_path.join("rooms.json"),
        r#"[
  {
    "id": "start",
    "title": "Starting Chamber",
    "summary": "A quiet stone room where the journey begins.",
    "inspect_text": "The room is calm and quiet. A single doorway leads into the unknown.",
    "features": [
      {
        "id": "start-hearth",
        "label": "the stone hearth",
        "aliases": ["hearth", "fire", "stone"],
        "inspect_text": "A low stone hearth where gentle embers glow warm."
      }
    ],
    "exits": []
  }
]
"#,
    )?;

    fs::write(
        en_path.join("actors.json"),
        r#"[
  {
    "id": "player",
    "name": "Traveler",
    "room_id": "start",
    "initial_stats": {
      "hp": 10,
      "strength": 3,
      "defense": 1,
      "intelligence": 3
    },
    "aliases": ["me", "myself"],
    "inspect_text": "Yourself.",
    "attackable": false,
    "prompt_context": {}
  }
]
"#,
    )?;

    fs::write(
        en_path.join("maps.json"),
        r#"[
  {
    "id": "overworld",
    "label": "Overworld",
    "rooms": [
      { "room_id": "start", "x": 0.0, "y": 0.0 }
    ]
  }
]
"#,
    )?;

    fs::write(en_path.join("messages.json"), "{}\n")?;
    fs::write(
        en_path.join("opening.json"),
        format!(
            r#"{{
  "id": "{pack_name}",
  "title": "{pack_name}",
  "start_room_id": "start",
  "intro_text": "A quiet journey begins.",
  "help_text": "Type commands or 'help' for instructions."
}}
"#
        ),
    )?;
    fs::write(en_path.join("sequences.json"), "[]\n")?;
    fs::write(
        en_path.join("system.json"),
        r#"{
  "dialogue_system_prompt": "You write one short line of in-character dialogue for a grounded narrative game. Return only the spoken line. Do not include speaker names or formatting.",
  "dialogue_section_character": "Character",
  "dialogue_section_setting": "Setting",
  "dialogue_section_current_beat": "Current Beat",
  "dialogue_section_subtext": "Subtext / Emotional Context",
  "dialogue_section_recent_memory": "Recent Memory",
  "dialogue_latest_line_label": "{other_person_name}'s Latest Line",
  "dialogue_section_response": "Response",
  "dialogue_no_direct_question": "No direct question. Offer one grounded line.",
  "dialogue_no_character_facts": "No character facts.",
  "dialogue_no_setting_facts": "No setting facts.",
  "dialogue_no_current_beat_facts": "No current beat facts.",
  "dialogue_no_subtext_facts": "No subtext facts.",
  "dialogue_no_recent_memory": "No recent conversation memory.",
  "dialogue_response_fallback": "Reply naturally and stay grounded.",
  "menu_intent_system_prompt": "Decide whether the player's latest line clearly expresses the specific intent needed to open a menu. Return only OPEN or PASS.",
  "menu_section_title": "Menu",
  "menu_id_label": "Id",
  "menu_offered_by_label": "Offered by",
  "menu_intent_guidance_label": "Intent guidance",
  "menu_available_options_label": "Available options",
  "menu_section_setting": "Setting",
  "menu_section_current_beat": "Current Beat",
  "menu_section_recent_memory": "Recent Memory",
  "menu_latest_line_label": "{other_person_name}'s Latest Line",
  "menu_decision_label": "Decision",
  "menu_no_direct_request": "No direct request was given.",
  "menu_no_authored_options": "No authored options.",
  "menu_decision_instruction": "Return OPEN only if the latest line clearly expresses the intent. Return PASS otherwise.",
  "prompt_time_note": "It is {current_time}.",
  "prompt_current_room_note": "You are in the {room_title}.",
  "prompt_visible_features_note": "Visible here: {features}.",
  "prompt_people_here_note": "Also here: {people}.",
  "prompt_exits_note": "From here you can go to {exits}.",
  "prompt_current_speaker_note": "{other_person_name} is speaking with you in the {room_title}.",
  "prompt_shared_room_note": "{other_person_name} is here with you in the {room_title}.",
  "prompt_latest_words_note": "Their latest words mention {features}.",
  "prompt_address_other_person_note": "Reply directly to {other_person_name}."
}
"#,
    )?;
    fs::write(en_path.join("ui.json"), "{}\n")?;

    Ok(())
}
