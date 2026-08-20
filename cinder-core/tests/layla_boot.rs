use cinder_core::content::loader::load_named_pack;
use std::collections::HashSet;

fn room_ids() -> Vec<String> {
    (1..=5)
        .flat_map(|r| (1..=5).map(move |c| format!("r{}c{}", r, c)))
        .collect()
}

fn exit_count(r: usize, c: usize) -> usize {
    (r > 1) as usize + (r < 5) as usize + (c > 1) as usize + (c < 5) as usize
}

#[test]
fn layla_pack_boots() {
    let content = load_named_pack("layla", None).expect("layla pack must load");
    assert_eq!(content.locale, "en");
    assert_eq!(content.settings.title, "Layla");
    assert_eq!(content.opening.start_room_id, "r1c1");
    assert_eq!(content.opening.start_room_ids.len(), 25);

    let room = content.room("r1c1").expect("start room must exist");
    assert_eq!(room.title, "A Lightless Cell");

    let commands: Vec<String> = content
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
fn layla_grid_has_25_rooms() {
    let content = load_named_pack("layla", None).expect("layla pack must load");
    let ids: HashSet<String> = content.rooms.iter().map(|r| r.id.clone()).collect();
    assert_eq!(ids.len(), 25);
    assert_eq!(ids, room_ids().into_iter().collect());
}

#[test]
fn layla_grid_adjacency() {
    let content = load_named_pack("layla", None).expect("layla pack must load");
    let by_id: std::collections::HashMap<&str, &cinder_core::content::types::RoomDefinition> =
        content.rooms.iter().map(|r| (r.id.as_str(), r)).collect();

    for r in 1..=5 {
        for c in 1..=5 {
            let rid = format!("r{}c{}", r, c);
            let room = *by_id.get(rid.as_str()).expect("room exists");
            assert_eq!(
                room.exits.len(),
                exit_count(r, c),
                "{} should have {} exits (corner=2, edge=3, interior=4)",
                rid,
                exit_count(r, c)
            );
            for exit in &room.exits {
                assert!(by_id.contains_key(exit.room_id.as_str()), "{} -> {} missing target", rid, exit.room_id);
                let back = by_id
                    .get(exit.room_id.as_str())
                    .expect("target exists")
                    .exits
                    .iter()
                    .any(|e| e.room_id == rid);
                assert!(back, "{} -> {} has no return exit", exit.room_id, rid);
            }
        }
    }

    let corner = by_id.get("r1c1").expect("corner");
    let dirs: HashSet<String> = corner.exits.iter().map(|e| e.label.clone()).collect();
    assert_eq!(dirs, HashSet::from(["East".to_string(), "South".to_string()]));
    let center = by_id.get("r3c3").expect("center");
    assert_eq!(center.exits.len(), 4);
}

#[test]
fn layla_random_spawn() {
    let content = load_named_pack("layla", None).expect("layla pack must load");
    let valid: HashSet<String> = room_ids().into_iter().collect();
    let mut seen = HashSet::new();
    for _ in 0..60 {
        let runtime =
            cinder_core::CinderRuntime::new(content.clone(), false).expect("runtime constructs");
        let state = runtime.export_state().expect("state export");
        assert!(
            valid.contains(&state.current_room_id),
            "spawn {} not a valid room",
            state.current_room_id
        );
        seen.insert(state.current_room_id);
    }
    assert!(
        seen.len() >= 3,
        "spawns should vary across runs, saw only {}",
        seen.len()
    );
}

#[test]
fn layla_runtime_boots_web_path() {
    let content = load_named_pack("layla", None).expect("layla pack must load");
    let runtime =
        cinder_core::CinderRuntime::new(content, false).expect("runtime must construct");
    let intro = runtime.current_intro_text().expect("intro text");
    assert!(intro.contains("Layla") || intro.contains("cold stone"));
    let state = runtime.export_state().expect("state export");
    let valid: HashSet<String> = room_ids().into_iter().collect();
    assert!(valid.contains(&state.current_room_id));
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
        outcome.text.contains("==") && outcome.text.contains("\n"),
        "expected room observation framing, got: {}",
        outcome.text
    );
    let outcome = runtime
        .run_turn("go northeast")
        .expect("move with no exit must not panic");
    assert!(
        outcome.text.to_lowercase().contains("can't")
            || outcome.text.to_lowercase().contains("error")
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

#[test]
fn layla_move_dispatch() {
    let content = load_named_pack("layla", None).expect("layla pack must load");
    let runtime =
        cinder_core::CinderRuntime::new(content, false).expect("runtime must construct");
    let state = runtime.export_state().expect("state export");
    let room = runtime
        .content()
        .room(&state.current_room_id)
        .expect("spawn room exists");
    assert!(!room.exits.is_empty(), "spawn room should have an exit");
    let exit = &room.exits[0];
    let outcome = runtime
        .run_turn(&format!("go {}", exit.label.to_lowercase()))
        .expect("move must dispatch");
    let after = runtime.export_state().expect("state export");
    assert_eq!(after.current_room_id, exit.room_id);
    let _ = outcome;
}
