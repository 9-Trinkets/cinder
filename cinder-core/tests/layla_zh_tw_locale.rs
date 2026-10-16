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
