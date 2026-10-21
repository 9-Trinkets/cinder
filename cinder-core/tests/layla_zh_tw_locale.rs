//! Verifies the `zh-TW` locale of the `layla` pack loads end-to-end and
//! localizes player-facing text, guarding the localization against English
//! leaks. The English-facing behavior is covered by `layla_content_loads.rs`.

use cinder_core::content::loader::load_named_pack;

/// Removes `{placeholder}` tokens so only literal text remains.
fn strip_placeholders(s: &str) -> String {
    let mut out = String::new();
    let mut depth = 0u32;
    for c in s.chars() {
        match c {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out
}

fn has_ascii_letters(s: &str) -> bool {
    s.chars().any(|c| c.is_ascii_alphabetic())
}

/// Whether the text contains at least one CJK ideograph. Used to assert
/// prompt-facing prose was actually translated: an untranslated English string
/// carries no CJK at all.
fn has_cjk(s: &str) -> bool {
    s.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c))
}

#[test]
fn layla_zh_tw_locale_loads_and_names_itself() {
    let pack = load_named_pack("layla", Some("zh-TW")).expect("layla zh-TW loads and validates");
    assert_eq!(pack.ui_text.language_name, "繁體中文");
    assert!(!pack.actors.is_empty());
}

#[test]
fn layla_zh_tw_messages_have_no_english_leaks() {
    let pack = load_named_pack("layla", Some("zh-TW")).expect("layla zh-TW loads and validates");
    assert!(!pack.messages.is_empty());

    let mut leaks: Vec<String> = Vec::new();
    for (key, message) in &pack.messages {
        let literal = strip_placeholders(message.text());
        if has_ascii_letters(&literal) {
            leaks.push(format!("{key}: {}", message.text()));
        }
    }
    assert!(
        leaks.is_empty(),
        "untranslated zh-TW message text ({} keys):\n{leaks:#?}",
        leaks.len()
    );
}

#[test]
fn layla_zh_tw_specific_lines_render_in_chinese() {
    let pack = load_named_pack("layla", Some("zh-TW")).expect("layla zh-TW loads and validates");

    let picked_up = pack
        .render_message("item.acquired_inventory", &[("label", "葉膏")])
        .expect("item.acquired_inventory renders");
    assert_eq!(picked_up, "你撿起了葉膏。");

    // A bundled-default key (not authored in the pack) must also localize.
    let default = pack
        .render_message("error.use_empty", &[])
        .expect("error.use_empty renders");
    assert!(
        !has_ascii_letters(&strip_placeholders(&default)),
        "{default}"
    );

    // The Handler speaks Chinese.
    let handler = pack
        .render_message("handler.descend.deep_wood", &[])
        .expect("handler line renders");
    assert!(handler.contains("森林"), "{handler}");
}

#[test]
fn layla_zh_tw_presentation_and_ui_are_localized() {
    let pack = load_named_pack("layla", Some("zh-TW")).expect("layla zh-TW loads and validates");

    assert_eq!(
        pack.presentation.presentation_text.objective,
        "\n\n操作員：\n{objective}"
    );
    assert!(
        pack.presentation
            .presentation_text
            .actor_speech
            .starts_with("{actor_name}：")
    );
    assert!(!has_ascii_letters(&pack.ui_text.look_panel_title));
    assert!(
        pack.ui_text
            .shell_menu
            .items
            .iter()
            .all(|item| !item.label.is_empty() && !has_ascii_letters(&item.label))
    );

    // Structural ids inside the UI shell menu must remain untouched.
    assert!(
        pack.ui_text
            .shell_menu
            .items
            .iter()
            .any(|item| item.id == "quests")
    );
}

#[test]
fn layla_zh_tw_prompts_localized_with_machine_tokens_intact() {
    let pack = load_named_pack("layla", Some("zh-TW")).expect("layla zh-TW loads and validates");
    let sys = &pack.system_text;

    // Output-generating prompts instruct Chinese output.
    assert!(sys.dialogue_system_prompt.contains("繁體中文"));
    assert!(sys.transition_commentary_system_prompt.contains("繁體中文"));
    assert!(
        sys.chapter_script_summary_prompt_template
            .contains("繁體中文")
    );

    // Machine-parsed tokens must survive translation.
    assert!(sys.menu_intent_system_prompt.contains("OPEN"));
    assert!(sys.menu_intent_system_prompt.contains("PASS"));
    assert!(
        sys.transition_commentary_system_prompt
            .contains("'summary'")
    );
    assert!(
        sys.transition_commentary_system_prompt
            .contains("'introduction'")
    );
    assert!(
        sys.transition_commentary_prompt_template
            .contains("\"summary\"")
    );
    assert!(sys.actor_turn_act_decision_template.contains("{command}"));

    // Substitution placeholders must survive translation.
    assert_eq!(sys.prompt_current_room_note, "你在{room_title}。");
    assert!(
        sys.actor_turn_move_option_template
            .contains("{prompt_verb}")
    );
    assert!(sys.actor_turn_move_option_template.contains("{room_title}"));
}

#[test]
fn layla_zh_tw_prompt_context_and_narrative_lines_are_localized() {
    let pack = load_named_pack("layla", Some("zh-TW")).expect("layla zh-TW loads and validates");

    let mut untranslated: Vec<String> = Vec::new();
    let mut checked = 0usize;

    fn check(untranslated: &mut Vec<String>, checked: &mut usize, origin: &str, text: &str) {
        if text.trim().is_empty() {
            return;
        }
        *checked += 1;
        if !has_cjk(text) {
            untranslated.push(format!("{origin}: {text}"));
        }
    }

    fn check_notes(
        untranslated: &mut Vec<String>,
        checked: &mut usize,
        origin: &str,
        notes: &[String],
    ) {
        for note in notes {
            check(untranslated, checked, origin, note);
        }
    }

    for actor in &pack.actors {
        let ctx = &actor.prompt_context;
        let origin = || format!("actor {}", actor.id);
        check_notes(
            &mut untranslated,
            &mut checked,
            &format!("{} character_notes", origin()),
            &ctx.character_notes,
        );
        check_notes(
            &mut untranslated,
            &mut checked,
            &format!("{} subtext_notes", origin()),
            &ctx.subtext_notes,
        );
        check_notes(
            &mut untranslated,
            &mut checked,
            &format!("{} response_notes", origin()),
            &ctx.response_notes,
        );
        check_notes(
            &mut untranslated,
            &mut checked,
            &format!("{} behavior_examples", origin()),
            &ctx.behavior_examples,
        );

        for stage in &actor.transformations {
            let Some(rename) = &stage.rename else {
                continue;
            };
            check(
                &mut untranslated,
                &mut checked,
                &format!("{} rename.who", origin()),
                &rename.who,
            );
            check(
                &mut untranslated,
                &mut checked,
                &format!("{} rename.fragment", origin()),
                &rename.fragment,
            );
            if let Some(rn_ctx) = &rename.prompt_context {
                check_notes(
                    &mut untranslated,
                    &mut checked,
                    &format!("{} rename.character_notes", origin()),
                    &rn_ctx.character_notes,
                );
                check_notes(
                    &mut untranslated,
                    &mut checked,
                    &format!("{} rename.subtext_notes", origin()),
                    &rn_ctx.subtext_notes,
                );
                check_notes(
                    &mut untranslated,
                    &mut checked,
                    &format!("{} rename.response_notes", origin()),
                    &rn_ctx.response_notes,
                );
                check_notes(
                    &mut untranslated,
                    &mut checked,
                    &format!("{} rename.behavior_examples", origin()),
                    &rn_ctx.behavior_examples,
                );
            }
        }
    }

    let opening_ctx = &pack.opening.prompt_context;
    check_notes(
        &mut untranslated,
        &mut checked,
        "opening setting_notes",
        &opening_ctx.setting_notes,
    );
    check_notes(
        &mut untranslated,
        &mut checked,
        "opening subtext_notes",
        &opening_ctx.subtext_notes,
    );

    for stage in &pack.beats.stages {
        for line in &stage.narrative_lines {
            check(
                &mut untranslated,
                &mut checked,
                &format!("beat {} narrative_lines", stage.id),
                line,
            );
        }
    }

    assert!(checked > 0, "no prompt-context or narrative text found");
    assert!(
        untranslated.is_empty(),
        "untranslated zh-TW prompt/narrative text ({}):\n{untranslated:#?}",
        untranslated.len()
    );
}

/// Whether the text carries an English-looking word (three or more consecutive
/// ASCII letters) outside placeholders. Single letters (`J`) and key names
/// handled by the allowlist are not leaks.
fn has_english_word(s: &str) -> bool {
    const ALLOWED: &[&str] = &[
        "Enter",
        "Esc",
        "PageUp",
        "PageDown",
        "JSON",
        "rating",
        "review_text",
        "Trinkets",
        "www",
        "com",
        "MP",
        "HP",
        "markdown",
    ];
    let literal = strip_placeholders(s);
    let mut word = String::new();
    let flush = |word: &mut String| -> bool {
        let leak = word.len() >= 3 && !ALLOWED.contains(&word.as_str());
        word.clear();
        leak
    };
    for c in literal.chars() {
        if c.is_ascii_alphabetic() || c == '_' {
            word.push(c);
        } else if flush(&mut word) {
            return true;
        }
    }
    flush(&mut word)
}

#[test]
fn layla_zh_tw_ui_text_has_no_english_fallbacks() {
    let pack = load_named_pack("layla", Some("zh-TW")).expect("layla zh-TW loads and validates");
    let value = serde_json::to_value(&pack.ui_text).expect("ui text serializes");
    let mut leaks = Vec::new();
    collect_leaks("ui_text", &value, &mut leaks);
    assert!(
        leaks.is_empty(),
        "UI text keys still falling back to English ({}):\n{leaks:#?}",
        leaks.len()
    );
}

fn collect_leaks(path: &str, value: &serde_json::Value, leaks: &mut Vec<String>) {
    match value {
        serde_json::Value::String(text) if has_english_word(text) => {
            leaks.push(format!("{path}: {text}"));
        }
        serde_json::Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                collect_leaks(&format!("{path}/{index}"), item, leaks);
            }
        }
        serde_json::Value::Object(map) => {
            for (key, item) in map {
                if key == "id" {
                    continue;
                }
                collect_leaks(&format!("{path}/{key}"), item, leaks);
            }
        }
        _ => {}
    }
}

#[test]
fn layla_zh_tw_overlay_files_localize_pack_root_text() {
    let pack = load_named_pack("layla", Some("zh-TW")).expect("layla zh-TW loads and validates");
    let mut leaks = Vec::new();

    for item in &pack.items {
        for (field, text) in [
            ("label", item.label.as_str()),
            ("description", item.description.as_str()),
            ("look_description", item.look_description.as_str()),
        ] {
            if has_english_word(text) {
                leaks.push(format!("item {} {field}: {text}", item.id));
            }
        }
    }
    for skill in &pack.skills.skills {
        for (field, text) in [
            ("label", skill.label.as_str()),
            ("description", skill.description.as_str()),
        ] {
            if has_english_word(text) {
                leaks.push(format!("skill {} {field}: {text}", skill.id));
            }
        }
    }
    for action in &pack.actions {
        if has_english_word(&action.label) {
            leaks.push(format!("action {} label: {}", action.id, action.label));
        }
        if let Some(panel) = &action.ui.panel_config {
            for (field, text) in [
                ("title", panel.title.as_str()),
                ("prompt", panel.prompt.as_str()),
            ] {
                if has_english_word(text) {
                    leaks.push(format!("action {} panel {field}: {text}", action.id));
                }
            }
        }
    }
    for (field, text) in [
        ("title", pack.settings.title.as_str()),
        ("tagline", pack.settings.tagline.as_str()),
        ("description", pack.settings.description.as_str()),
    ] {
        if has_english_word(text) {
            leaks.push(format!("settings {field}: {text}"));
        }
    }
    if has_english_word(&pack.settings.combat.player_defeat_text) {
        leaks.push("settings combat player_defeat_text".to_string());
    }
    for tag in &pack.settings.tags {
        if has_english_word(tag) {
            leaks.push(format!("settings tag: {tag}"));
        }
    }

    assert!(
        leaks.is_empty(),
        "English left in zh-TW pack-root text ({}):\n{leaks:#?}",
        leaks.len()
    );
}

#[test]
fn layla_zh_tw_accepts_chinese_command_phrases_alongside_english() {
    let pack = load_named_pack("layla", Some("zh-TW")).expect("layla zh-TW loads and validates");
    let look = pack
        .actions
        .iter()
        .find(|action| action.id == "look")
        .expect("look action");
    assert!(look.phrases.iter().any(|phrase| phrase == "查看"));
    assert!(
        look.phrases.iter().any(|phrase| phrase == "look"),
        "English phrases must survive the overlay"
    );
    let english = load_named_pack("layla", Some("en")).expect("layla en loads");
    assert!(
        english
            .actions
            .iter()
            .all(|a| a.phrases.iter().all(|p| p.is_ascii()))
    );
}
