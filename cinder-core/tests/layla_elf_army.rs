use cinder_core::content::types::PartyController;
use cinder_core::engine::state::{ActorStance, WorldState};
use cinder_core::loader::load_named_pack;

#[test]
fn elf_army_party_authors_role_orders() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");

    for (party_id, leader, member_count) in [
        ("elf-vanguard-west", "elf-knight-2", 4),
        ("elf-vanguard-east", "elf-knight-7", 4),
        ("elf-queen-patrol", "elf-queen-4", 3),
        ("elf-royal-court", "elf-king-5", 5),
    ] {
        let squad = pack
            .settings
            .parties
            .iter()
            .find(|party| party.id == party_id)
            .unwrap_or_else(|| panic!("{party_id} party authored"));
        assert_eq!(squad.controller, PartyController::Ai);
        assert_eq!(squad.leader.as_deref(), Some(leader));
        assert_eq!(squad.members.len(), member_count);
        assert!(
            squad
                .combat_rules
                .iter()
                .all(|rule| !rule.skill_id.is_empty()),
            "AI-party rules must bind a skill so gating stays skill-driven"
        );
    }

    let state = WorldState::new(&pack);
    for (actor_id, order, expected_party) in [
        ("elf-pawn-1", "follow", "elf-vanguard-west"),
        ("elf-pawn-2", "follow", "elf-vanguard-west"),
        ("elf-knight-2", "advance", "elf-vanguard-west"),
        ("elf-bishop-3", "follow", "elf-vanguard-west"),
        ("elf-pawn-3", "follow", "elf-queen-patrol"),
        ("elf-pawn-4", "follow", "elf-queen-patrol"),
        ("elf-queen-4", "patrol", "elf-queen-patrol"),
        ("elf-pawn-5", "guard", "elf-royal-court"),
        ("elf-pawn-6", "guard", "elf-royal-court"),
        ("elf-rook-1", "follow", "elf-royal-court"),
        ("elf-king-5", "guard", "elf-royal-court"),
        ("elf-rook-8", "follow", "elf-royal-court"),
        ("elf-bishop-6", "follow", "elf-vanguard-east"),
        ("elf-knight-7", "advance", "elf-vanguard-east"),
        ("elf-pawn-7", "follow", "elf-vanguard-east"),
        ("elf-pawn-8", "follow", "elf-vanguard-east"),
    ] {
        assert_eq!(
            state.party_order(&pack, actor_id).as_deref(),
            Some(order),
            "unexpected standing order for {actor_id}"
        );
        assert_eq!(
            state
                .party_of_actor(actor_id)
                .map(|party| party.id.as_str()),
            Some(expected_party),
            "{actor_id} should belong to the {expected_party} party"
        );
    }
}

#[test]
fn elf_actors_carry_role_tags() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    for (actor_id, role_tag) in [
        ("elf-pawn-1", "pawn"),
        ("elf-rook-1", "rook"),
        ("elf-knight-2", "knight"),
        ("elf-bishop-3", "bishop"),
        ("elf-queen-4", "queen"),
        ("elf-king-5", "king"),
    ] {
        let actor = pack.actor(actor_id).expect("elf actor exists");
        assert!(
            actor.tags.iter().any(|tag| tag == role_tag),
            "{actor_id} is missing its '{role_tag}' role tag"
        );
        assert!(
            actor.tags.iter().any(|tag| tag == "elf"),
            "{actor_id} lost the shared 'elf' tag"
        );
    }
    let bishop = pack.actor("elf-bishop-3").unwrap();
    assert!(bishop.tags.iter().any(|tag| tag == "healer"));
}

#[test]
fn allied_elf_resolves_to_the_player_party() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = WorldState::new(&pack);
    assert_eq!(
        state
            .party_of_actor("elf-bishop-3")
            .map(|party| party.id.as_str()),
        Some("elf-vanguard-west")
    );

    state.set_actor_stance(&pack, "elf-bishop-3", ActorStance::Allied, false);
    let party = state
        .party_of_actor("elf-bishop-3")
        .expect("allied elf still has a party");
    assert_eq!(
        party.id, "player",
        "an awakened elf must be treated as a player-party ally"
    );
}

#[test]
fn board_entry_mobilizes_the_elf_roles() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let board_rules = pack
        .hooks
        .get("board.entered")
        .and_then(|hook| hook.get("input_overlay"))
        .and_then(|overlay| overlay.get("rules"))
        .and_then(|rules| rules.as_array())
        .expect("board.entered rules authored");

    let mut ordered_tags = std::collections::BTreeSet::new();
    for rule in board_rules {
        let Some(tag) = rule
            .get("payload_template")
            .and_then(|payload| payload.get("tag"))
            .and_then(|tag| tag.as_str())
        else {
            continue;
        };
        if rule
            .get("payload_template")
            .and_then(|payload| payload.get("kind"))
            .and_then(|kind| kind.as_str())
            == Some("set_party_order_by_tag")
        {
            ordered_tags.insert(tag.to_string());
        }
    }
    for role in ["pawn", "knight", "rook", "queen"] {
        assert!(
            ordered_tags.contains(role),
            "board entry should retask the '{role}' role group"
        );
    }
}

#[test]
fn guarding_rook_shields_the_elf_king_from_the_player() {
    use cinder_core::engine::dialogue::ScriptedDialogueGenerator;
    use cinder_core::engine::runtime::CinderRuntime;

    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = WorldState::new(&pack);

    // Fight the king in its own room, with a rook holding the guard.
    state.current_room_id = "d8c5".to_string();
    state
        .actors
        .actor_room_overrides
        .insert("elf-rook-1".to_string(), "d8c5".to_string());
    state.set_actor_stance(&pack, "elf-rook-1", ActorStance::Hostile, false);
    assert!(
        matches!(
            state.party_order(&pack, "elf-rook-1").as_deref(),
            Some("guard" | "follow")
        ),
        "rook order must be guard or follow"
    );

    let king_initial_hp = state.actor_stat("elf-king-5", "hp");
    let rook_initial_hp = state.actor_stat("elf-rook-1", "hp");

    state.clock.turn_number = 1;
    let dialogue = std::sync::Arc::new(ScriptedDialogueGenerator::new());
    let runtime =
        CinderRuntime::with_dialogue_generator(pack, state, dialogue).expect("runtime creates");

    let outcome = runtime.run_turn("attack elf king").expect("turn runs");
    let text = outcome.text();
    let end_state = runtime.export_state().unwrap();

    assert_eq!(
        end_state.actor_stat("elf-king-5", "hp"),
        king_initial_hp,
        "the king should be shielded: {text}"
    );
    assert!(
        end_state.actor_stat("elf-rook-1", "hp") < rook_initial_hp,
        "the rook should absorb the blow: {text}"
    );
    assert!(
        text.contains("steps across"),
        "expected intercept narration: {text}"
    );
}

#[test]
fn board_entry_mobilizes_squad_orders_and_king_defeat_clears_hostility() {
    use cinder_core::engine::dialogue::ScriptedDialogueGenerator;
    use cinder_core::engine::runtime::CinderRuntime;

    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = WorldState::new(&pack);

    // Start in r5c5 (Floor 1 end room) with shaman defeated so the trapdoor is open
    state.current_room_id = "r5c5".to_string();
    state.clock.turn_number = 1;
    state.story_vars.set_unchecked("shaman_defeated", "true");
    let dialogue = std::sync::Arc::new(ScriptedDialogueGenerator::new());
    let runtime = CinderRuntime::with_dialogue_generator(pack.clone(), state, dialogue)
        .expect("runtime creates");

    // Move Down into Floor 2 (d1c1), triggering board.entered for deep-forest
    let outcome = runtime.run_turn("down").expect("descends to floor 2");
    assert!(outcome.text().contains("Mushroom Grove") || outcome.text().contains("d1c1"));

    let end_state = runtime.export_state().unwrap();

    // Verify squad mobilized orders
    for (actor_id, expected_order) in [
        ("elf-knight-2", "advance"),
        ("elf-bishop-3", "follow"),
        ("elf-pawn-1", "follow"),
        ("elf-pawn-2", "follow"),
        ("elf-knight-7", "advance"),
        ("elf-bishop-6", "follow"),
        ("elf-pawn-7", "follow"),
        ("elf-pawn-8", "follow"),
        ("elf-queen-4", "patrol"),
        ("elf-pawn-3", "follow"),
        ("elf-pawn-4", "follow"),
        ("elf-king-5", "guard"),
        ("elf-rook-1", "follow"),
        ("elf-rook-8", "follow"),
        ("elf-pawn-5", "guard"),
        ("elf-pawn-6", "guard"),
    ] {
        assert_eq!(
            end_state.party_order(&pack, actor_id).as_deref(),
            Some(expected_order),
            "expected {actor_id} to have order {expected_order} after board entry"
        );
    }

    // Advance turns until the vanguard reaches the player, asserting squad cohesion:
    // Once assembled, all followers march into rooms together with their knight leader.
    let mut reached_player = false;
    for _ in 1..=20 {
        let _ = runtime.run_turn("look").expect("look turn");
        let _ = runtime.run_tick().expect("run tick");
        let cur = runtime.export_state().unwrap();
        let k_room = cur.actor_current_room_id(&pack, "elf-knight-2");
        if k_room == cur.current_room_id {
            reached_player = true;
            // The entire squad must arrive in the player's room together
            assert_eq!(
                cur.actor_current_room_id(&pack, "elf-bishop-3"),
                cur.current_room_id
            );
            assert_eq!(
                cur.actor_current_room_id(&pack, "elf-pawn-1"),
                cur.current_room_id
            );
            assert_eq!(
                cur.actor_current_room_id(&pack, "elf-pawn-2"),
                cur.current_room_id
            );
            break;
        }
    }
    assert!(reached_player, "knight should have advanced to player room");
}

#[test]
fn vanguard_followers_march_together_with_their_knight_and_succeed_on_defeat() {
    use cinder_core::engine::dialogue::ScriptedDialogueGenerator;
    use cinder_core::engine::runtime::CinderRuntime;

    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = WorldState::new(&pack);

    // Assemble West Vanguard in d7c2 (facing player at d1c1)
    for id in ["elf-knight-2", "elf-bishop-3", "elf-pawn-1", "elf-pawn-2"] {
        state
            .actors
            .actor_room_overrides
            .insert(id.to_string(), "d7c2".to_string());
        state.set_actor_stance(&pack, id, ActorStance::Hostile, false);
    }
    state.current_room_id = "d1c1".to_string();
    state.clock.turn_number = 1;
    // Set clock to 1 so after tick start (+1 minute) it becomes 2 (even, cadence 2 fires)
    state.clock.current_time_minutes = 1;

    let dialogue = std::sync::Arc::new(ScriptedDialogueGenerator::new());
    let runtime = CinderRuntime::with_dialogue_generator(pack.clone(), state, dialogue)
        .expect("runtime creates");

    // Run tick: leader advances toward player (d1c1)
    runtime.run_tick().expect("tick runs");
    let s1 = runtime.export_state().unwrap();
    let k_room = s1.actor_current_room_id(&pack, "elf-knight-2").to_string();
    assert_ne!(k_room, "d7c2", "knight should have advanced to a new room");

    // All squad followers must have moved with the knight into the new room!
    for follower_id in ["elf-bishop-3", "elf-pawn-1", "elf-pawn-2"] {
        assert_eq!(
            s1.actor_current_room_id(&pack, follower_id),
            k_room,
            "follower {follower_id} must have moved into {k_room} with knight leader"
        );
    }

    // Now defeat the knight leader: squad leadership must succeed to one of the followers
    let mut s2 = s1;
    s2.combat
        .actor_stats
        .entry("elf-knight-2".to_string())
        .or_default()
        .insert("hp".to_string(), 1);
    // Put player in the same room as the vanguard to strike
    s2.current_room_id = k_room.to_string();

    let runtime2 = CinderRuntime::with_dialogue_generator(
        pack.clone(),
        s2,
        std::sync::Arc::new(ScriptedDialogueGenerator::new()),
    )
    .expect("runtime creates");
    let _ = runtime2
        .run_turn("attack elf knight")
        .expect("attack defeats knight");
    let end_state = runtime2.export_state().unwrap();

    assert!(end_state.actor_is_defeated("elf-knight-2", "hp"));
    let party = end_state.party_of_actor("elf-bishop-3").unwrap();
    let new_leader = party.leader.as_deref().expect("new leader was chosen");
    assert_ne!(
        new_leader, "elf-knight-2",
        "defeated knight should no longer be leader"
    );
    assert!(
        ["elf-bishop-3", "elf-pawn-1", "elf-pawn-2"].contains(&new_leader),
        "successor must be a living squad member"
    );
    assert_eq!(
        end_state.party_order(&pack, new_leader).as_deref(),
        Some("advance"),
        "new leader must inherit the advance order"
    );
}
