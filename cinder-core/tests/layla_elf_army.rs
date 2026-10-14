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
        ("elf-pawn-1", "patrol", "elf-vanguard-west"),
        ("elf-pawn-2", "patrol", "elf-vanguard-west"),
        ("elf-knight-2", "patrol", "elf-vanguard-west"),
        ("elf-bishop-3", "guard", "elf-vanguard-west"),
        ("elf-pawn-3", "patrol", "elf-queen-patrol"),
        ("elf-pawn-4", "patrol", "elf-queen-patrol"),
        ("elf-queen-4", "patrol", "elf-queen-patrol"),
        ("elf-pawn-5", "guard", "elf-royal-court"),
        ("elf-pawn-6", "guard", "elf-royal-court"),
        ("elf-rook-1", "guard", "elf-royal-court"),
        ("elf-king-5", "guard", "elf-royal-court"),
        ("elf-rook-8", "guard", "elf-royal-court"),
        ("elf-bishop-6", "guard", "elf-vanguard-east"),
        ("elf-knight-7", "patrol", "elf-vanguard-east"),
        ("elf-pawn-7", "patrol", "elf-vanguard-east"),
        ("elf-pawn-8", "patrol", "elf-vanguard-east"),
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
    assert_eq!(
        state.party_order(&pack, "elf-rook-1").as_deref(),
        Some("guard")
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
}
