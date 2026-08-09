use cinder_core::content::loader::load_named_pack;

#[test]
fn layla_pack_boots() {
    let content = load_named_pack("layla", None).expect("layla pack must load");
    assert_eq!(content.locale, "en");
    assert_eq!(content.settings.title, "Layla");
    assert_eq!(content.opening.start_room_id, "lightless-cell");

    let room = content
        .room("lightless-cell")
        .expect("start room must exist");
    assert_eq!(room.title, "A Lightless Cell");

    let commands: Vec<String> = content
        .commands
        .actions
        .iter()
        .filter(|c| c.player_enabled)
        .map(|c| c.id.clone())
        .collect();
    assert_eq!(commands, vec!["look", "move", "speak"]);

    assert!(content.stats.actor.contains_key("hp"));

    let look = content.command("look").expect("look command");
    assert!(look.has_effect(cinder_core::content::types::CommandEffect::ObserveRoom));
    let move_cmd = content.command("move").expect("move command");
    assert!(move_cmd.has_effect(cinder_core::content::types::CommandEffect::MoveActor));
    assert!(content.command("speak").expect("speak command").player_enabled);
}

#[test]
fn layla_runtime_boots_web_path() {
    let content = load_named_pack("layla", None).expect("layla pack must load");
    let runtime =
        cinder_core::CinderRuntime::new(content, false).expect("runtime must construct");
    let intro = runtime.current_intro_text().expect("intro text");
    assert!(intro.contains("Layla") || intro.contains("cold stone"));
    let state = runtime.export_state().expect("state export");
    assert_eq!(state.current_room_id, "lightless-cell");
    let _ = serde_json::to_string(&state).expect("state serializes");
}

#[test]
fn layla_look_dispatch() {
    let content = load_named_pack("layla", None).expect("layla pack must load");
    let runtime =
        cinder_core::CinderRuntime::new(content, false).expect("runtime must construct");
    let outcome = runtime
        .run_turn("look")
        .expect("look must not error at dispatch");
    assert!(
        outcome.text.contains("Lightless") || outcome.text.contains("cold stone"),
        "unexpected look text: {}",
        outcome.text
    );
    let outcome = runtime
        .run_turn("go north")
        .expect("move with no exit must not panic");
    assert!(
        outcome.text.to_lowercase().contains("error")
            || outcome.text.to_lowercase().contains("can't")
            || outcome.text.to_lowercase().contains("cannot")
            || outcome.text.to_lowercase().contains("nowhere"),
        "expected graceful no-exit message, got: {}",
        outcome.text
    );
    let outcome = runtime
        .run_turn("talk to the old man")
        .expect("speak with no actor must not panic");
    assert!(
        !outcome.text.trim().is_empty(),
        "expected a graceful no-actor reply, got empty"
    );
}
