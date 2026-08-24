use cinder_core::content::loader::load_named_pack;
use std::collections::HashSet;

const GRID: usize = 9;

fn room_ids() -> Vec<String> {
    (1..=GRID)
        .flat_map(|r| (1..=GRID).map(move |c| format!("r{}c{}", r, c)))
        .collect()
}

fn exit_count(r: usize, c: usize) -> usize {
    (r > 1) as usize + (r < GRID) as usize + (c > 1) as usize + (c < GRID) as usize
}

#[test]
fn layla_pack_boots() {
    let content = load_named_pack("layla", None).expect("layla pack must load");
    assert_eq!(content.locale, "en");
    assert_eq!(content.settings.title, "Layla");
    assert_eq!(content.opening.start_room_id, "r1c1");
    assert_eq!(content.opening.start_room_ids.len(), 81);

    let room = content.room("r1c1").expect("start room must exist");
    assert_eq!(room.title, "A Passage of Scored Walls");

    let commands: Vec<String> = content
        .actions
        .iter()
        .filter(|c| c.player_enabled)
        .map(|c| c.id.clone())
        .collect();
    assert_eq!(
        commands,
        vec![
            "look",
            "move",
            "speak",
            "attack",
            "place_flag",
            "pick_up_flag"
        ]
    );

    assert!(content.stats.actor.contains_key("hp"));
    assert!(content.stats.actor.contains_key("strength"));
    assert!(content.stats.actor.contains_key("defense"));

    // Golems at star points
    let golems: Vec<&str> = content.actors.iter().map(|a| a.id.as_str()).collect();
    assert!(golems.contains(&"golem-dark-nw"), "dark golem NW at r3c3");
    assert!(golems.contains(&"golem-pale-ne"), "pale golem NE at r3c7");
    assert!(golems.contains(&"golem-boss"), "boss at r5c5");
    assert!(golems.contains(&"golem-dark-sw"), "dark golem SW at r7c3");
    assert!(golems.contains(&"golem-pale-se"), "pale golem SE at r7c7");
    assert_eq!(golems.len(), 5);

    // Boss has more HP than regular golems
    let boss = content.actor("golem-boss").expect("boss exists");
    let boss_hp = boss.initial_stats.get("hp").copied().unwrap_or(0);
    let nw = content.actor("golem-dark-nw").expect("nw golem");
    let nw_hp = nw.initial_stats.get("hp").copied().unwrap_or(0);
    assert!(
        boss_hp > nw_hp,
        "boss ({}) should have more HP than regular golem ({})",
        boss_hp,
        nw_hp
    );

    let look = content.command("look").expect("look command");
    assert!(look.has_effect(cinder_core::content::types::CommandEffect::ObserveRoom));
    let move_cmd = content.command("move").expect("move command");
    assert!(move_cmd.has_effect(cinder_core::content::types::CommandEffect::MoveActor));
    assert!(
        content
            .command("speak")
            .expect("speak command")
            .player_enabled
    );
}

#[test]
fn layla_grid_has_81_rooms() {
    let content = load_named_pack("layla", None).expect("layla pack must load");
    let ids: HashSet<String> = content.rooms.iter().map(|r| r.id.clone()).collect();
    assert_eq!(ids.len(), 81);
    assert_eq!(ids, room_ids().into_iter().collect());
}

#[test]
fn layla_grid_adjacency() {
    let content = load_named_pack("layla", None).expect("layla pack must load");
    let by_id: std::collections::HashMap<&str, &cinder_core::content::types::RoomDefinition> =
        content.rooms.iter().map(|r| (r.id.as_str(), r)).collect();

    for r in 1..=GRID {
        for c in 1..=GRID {
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
                assert!(
                    by_id.contains_key(exit.room_id.as_str()),
                    "{} -> {} missing target",
                    rid,
                    exit.room_id
                );
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

    // Corner: 2 exits
    let corner = by_id.get("r1c1").expect("corner");
    let dirs: HashSet<String> = corner.exits.iter().map(|e| e.label.clone()).collect();
    assert_eq!(
        dirs,
        HashSet::from(["East".to_string(), "South".to_string()])
    );
    // Interior: 4 exits
    let center = by_id.get("r5c5").expect("center");
    assert_eq!(center.exits.len(), 4);
    // Star point: 4 exits
    let star = by_id.get("r3c3").expect("star");
    assert_eq!(star.exits.len(), 4);
}

#[test]
fn layla_random_spawn() {
    let content = load_named_pack("layla", None).expect("layla pack must load");
    let valid: HashSet<String> = room_ids().into_iter().collect();
    let mut seen = HashSet::new();
    for _ in 0..200 {
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
        seen.len() >= 5,
        "spawns should vary across runs, saw only {}",
        seen.len()
    );
}

#[test]
fn layla_runtime_boots_web_path() {
    let content = load_named_pack("layla", None).expect("layla pack must load");
    let runtime = cinder_core::CinderRuntime::new(content, false).expect("runtime must construct");
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
    let runtime = cinder_core::CinderRuntime::new(content, false).expect("runtime must construct");
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
    let runtime = cinder_core::CinderRuntime::new(content, false).expect("runtime must construct");
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

#[test]
fn layla_place_flag_via_runtime() {
    let content = load_named_pack("layla", None).expect("layla pack must load");
    let runtime = cinder_core::CinderRuntime::new(content, false).expect("runtime must construct");
    let outcome = runtime
        .run_turn("place flag")
        .expect("place flag must dispatch");
    assert!(
        outcome.text.to_lowercase().contains("marker")
            || outcome.text.to_lowercase().contains("flag"),
        "expected flag placement message, got: {}",
        outcome.text
    );
}

#[test]
fn layla_exhausted_flag_supply_rejects_placement() {
    let content = load_named_pack("layla", None).expect("layla pack must load");
    let runtime =
        cinder_core::CinderRuntime::new(content.clone(), false).expect("runtime must construct");
    // Drop into a mid-session state with the marker pool empty.
    let mut state = runtime.export_state().expect("state export");
    state.flags_remaining = 0;
    let runtime = cinder_core::CinderRuntime::from_state(content, state, false)
        .expect("runtime must reconstruct from exported state");

    let outcome = runtime
        .run_turn("place flag")
        .expect("place flag must dispatch");
    assert!(
        outcome.text.contains("no stone markers"),
        "expected supply-exhausted rejection, got: {}",
        outcome.text
    );
}

#[test]
fn layla_attack_dispatches() {
    let content = load_named_pack("layla", None).expect("layla pack must load");
    let runtime = cinder_core::CinderRuntime::new(content, false).expect("runtime must construct");
    let state = runtime.export_state().expect("state export");
    let room = runtime
        .content()
        .room(&state.current_room_id)
        .expect("spawn room exists");
    let has_actor_exit = room.exits.iter().any(|e| {
        runtime
            .content()
            .actors
            .iter()
            .any(|a| a.room_id == e.room_id && a.id.starts_with("golem-"))
    });
    if has_actor_exit {
        let golem_exit = room
            .exits
            .iter()
            .find(|e| {
                runtime
                    .content()
                    .actors
                    .iter()
                    .any(|a| a.room_id == e.room_id && a.id.starts_with("golem-"))
            })
            .unwrap();
        runtime
            .run_turn(&format!("go {}", golem_exit.label.to_lowercase()))
            .expect("move to golem");
        let golem = runtime
            .content()
            .actors
            .iter()
            .find(|a| a.room_id == golem_exit.room_id && a.id.starts_with("golem-"))
            .unwrap();
        let outcome = runtime
            .run_turn(&format!("attack {}", golem.name.to_lowercase()))
            .expect("attack must dispatch");
        assert!(
            outcome.text.to_lowercase().contains("damage")
                || outcome.text.to_lowercase().contains("strike"),
            "expected combat output, got: {}",
            outcome.text
        );
        // Regression: strikes are autonomous tick events, so attacking must not
        // trigger same-turn retaliation and the player stays untouched.
        let after = runtime.export_state().expect("state export");
        assert_eq!(
            after.phase,
            cinder_core::engine::state::GamePhase::Active,
            "player must survive waking the golem"
        );
        let hp = after.actor_stat("player", "hp");
        assert_eq!(
            hp, 10,
            "no same-turn retaliation: woken golem waits for its tick cooldown"
        );
        assert!(
            after.stance(golem.id.as_str()) == cinder_core::engine::state::ActorStance::Hostile,
            "attacked golem should wake hostile"
        );
        assert!(
            after.next_hostile_strike_at.contains_key(golem.id.as_str()),
            "woken golem must have an autonomous strike cooldown seeded"
        );
    }
}

fn path_to_nearest_star_golem(
    content: &cinder_core::ContentPack,
    from_room_id: &str,
) -> Option<(Vec<String>, String)> {
    const STAR_POINTS: [&str; 5] = ["r3c3", "r3c7", "r5c5", "r7c3", "r7c7"];
    let mut queue =
        std::collections::VecDeque::from([(from_room_id.to_string(), Vec::<String>::new())]);
    let mut visited = HashSet::from([from_room_id.to_string()]);
    while let Some((room_id, path)) = queue.pop_front() {
        if STAR_POINTS.contains(&room_id.as_str()) {
            let golem = content
                .actors
                .iter()
                .find(|a| a.id.starts_with("golem-") && a.room_id == room_id)?;
            return Some((path, golem.name.clone()));
        }
        for exit in &content.room(&room_id)?.exits {
            if visited.insert(exit.room_id.clone()) {
                let mut next_path = path.clone();
                next_path.push(exit.label.clone());
                queue.push_back((exit.room_id.clone(), next_path));
            }
        }
    }
    None
}

#[test]
fn layla_woken_golem_waits_for_its_tick_cooldown() {
    let content = load_named_pack("layla", None).expect("layla pack must load");
    let runtime =
        cinder_core::CinderRuntime::new(content.clone(), false).expect("runtime must construct");
    let state = runtime.export_state().expect("state export");
    let (path, golem_name) = path_to_nearest_star_golem(&content, &state.current_room_id)
        .expect("grid always reaches a star-point golem");

    // Walk to the golem's room. Golems are passive until struck, so travel is safe.
    for label in &path {
        runtime
            .run_turn(&format!("go {}", label.to_lowercase()))
            .expect("walk step must dispatch");
    }
    let here = runtime
        .export_state()
        .expect("state export")
        .current_room_id;
    let golem = content
        .actors
        .iter()
        .find(|a| a.id.starts_with("golem-") && a.room_id == here)
        .expect("golem present at the reached star point");

    runtime
        .run_turn(&format!("attack {}", golem_name.to_lowercase()))
        .expect("attack must dispatch once sharing the golem's room");

    // Autonomous model: the woken golem seeds a strike cooldown instead of
    // retaliating inside the player's turn. The grace window must cover the
    // first few game minutes after waking.
    let after_attack = runtime.export_state().expect("state export");
    assert_eq!(
        after_attack.phase,
        cinder_core::engine::state::GamePhase::Active,
        "player must survive waking the golem"
    );
    assert!(
        after_attack.stance(golem.id.as_str()) == cinder_core::engine::state::ActorStance::Hostile,
        "expected the attacked golem to wake hostile"
    );
    let seeded_at = after_attack
        .next_hostile_strike_at
        .get(golem.id.as_str())
        .copied()
        .expect("wake must seed an autonomous strike cooldown");
    assert!(
        seeded_at > after_attack.current_time_minutes,
        "cooldown must start in the future (grace window), got {seeded_at} vs now {}",
        after_attack.current_time_minutes
    );
    let interval = golem.attack_interval_minutes();
    // Later sub-turns of the same run_turn advance game time past the wake
    // moment, so only bound the cooldown relative to the exported clock.
    assert!(
        seeded_at > after_attack.current_time_minutes,
        "cooldown must still be pending (grace window), got {seeded_at} vs now {}",
        after_attack.current_time_minutes
    );
    assert!(
        seeded_at <= after_attack.current_time_minutes + interval,
        "cooldown must have been seeded no earlier than wake time + interval"
    );
}

#[test]
fn layla_attacking_an_ally_is_rejected_at_planning() {
    let content = load_named_pack("layla", None).expect("layla pack must load");
    let runtime =
        cinder_core::CinderRuntime::new(content.clone(), false).expect("runtime must construct");
    let state = runtime.export_state().expect("state export");
    let (path, golem_name) = path_to_nearest_star_golem(&content, &state.current_room_id)
        .expect("grid always reaches a star-point golem");
    for label in &path {
        runtime
            .run_turn(&format!("go {}", label.to_lowercase()))
            .expect("walk step must dispatch");
    }
    let here = runtime
        .export_state()
        .expect("state export")
        .current_room_id;
    let golem = content
        .actors
        .iter()
        .find(|a| a.id.starts_with("golem-") && a.room_id == here)
        .expect("golem present at the reached star point");

    // Simulate a converted ally (the encirclement path is covered elsewhere).
    let mut converted = runtime.export_state().expect("state export");
    converted.set_stance(
        golem.id.as_str(),
        cinder_core::engine::state::ActorStance::Allied,
    );
    let expected_hp = converted.actor_stat(golem.id.as_str(), "hp");
    let ally_runtime = cinder_core::CinderRuntime::from_state(content.clone(), converted, false)
        .expect("runtime from converted state");

    let outcome = ally_runtime
        .run_turn(&format!("attack {}", golem_name.to_lowercase()))
        .expect("attack input must dispatch a turn");
    assert!(
        outcome.text.contains("cannot raise a hand"),
        "expected ally rejection line, got: {}",
        outcome.text
    );
    let after = ally_runtime.export_state().expect("state export");
    assert_eq!(
        after.actor_stat(golem.id.as_str(), "hp"),
        expected_hp,
        "an ally must not take damage"
    );
}
