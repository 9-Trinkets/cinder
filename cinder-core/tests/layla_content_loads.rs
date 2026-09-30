//! Loads the in-repo `layla` pack end-to-end through the content loader and
//! validator. Guards real content regressions: drop tables, equipment slots,
//! and action hooks must keep resolving against the engine.

use cinder_core::content::loader::load_named_pack;
use cinder_core::content::types::{CommandEffect, DropSpec};

#[test]
fn layla_pack_loads_and_validates() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    assert!(!pack.actors.is_empty());
    assert!(!pack.actions.is_empty());
    assert!(pack.actions.iter().all(|action| {
        !action.effects.iter().any(|effect| {
            matches!(
                effect,
                CommandEffect::EquipItem | CommandEffect::UnequipItem
            )
        })
    }));
}

#[test]
fn elf_chess_mobs_declarations_resolve() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");

    assert!(pack.settings.equipment_slots.contains("off-hand"));
    assert!(pack.settings.equipment_slots.contains("cloak"));
    assert!(pack.item("leaf-crook").unwrap().equip_slots.len() == 2);
    assert!(pack.item("leaf-cloak").is_some());
    assert_eq!(
        pack.item("leaf-buckler").unwrap().equip_slots,
        vec!["off-hand".to_string()]
    );
    assert!(pack.item("leaf-paste").unwrap().equip_slots.is_empty());
    assert_eq!(pack.item("leaf-paste").unwrap().use_hook, "item.salve_used");

    let pawn = pack.actor("elf-pawn-1").unwrap();
    let DropSpec::Weighted(pool) = &pawn.initial_inventory["pawn-kit"] else {
        panic!("pawn kit must parse as a weighted pool");
    };
    assert!(pack.item(&pool.entries[0].item_id).is_some());

    let queen = pack.actor("elf-queen-4").unwrap();
    assert!(matches!(
        queen.initial_inventory["leaf-ring"],
        DropSpec::Chance(_)
    ));

    let king = pack.actor("elf-king-5").unwrap();
    assert!(matches!(
        king.initial_inventory["drain-scroll"],
        DropSpec::Always(1)
    ));
    assert!(matches!(
        king.initial_inventory["leaf-cloak"],
        DropSpec::Chance(_)
    ));
}

#[test]
fn goblin_shaman_is_initially_hostile_and_attacks_on_sight() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let shaman = pack.actor("goblin-shaman").expect("goblin-shaman exists");
    assert!(shaman.initial_hostile);
    assert!(shaman.attackable);
    assert_eq!(shaman.attack_interval_minutes, Some(2));

    let mut state = cinder_core::engine::state::WorldState::new(&pack);
    assert_eq!(
        state.relationship("goblin-shaman").stance,
        cinder_core::engine::state::ActorStance::Hostile
    );

    // Layla in r5c5 sees hostile shaman
    state.current_room_id = "r5c5".to_string();
    assert_eq!(
        state.stance("goblin-shaman"),
        cinder_core::engine::state::ActorStance::Hostile
    );
}

#[test]
fn goblin_shaman_defeat_narrates_world_hint_lines() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    assert!(pack.messages.contains_key("shaman.defeat"));
    assert!(pack.messages.contains_key("handler.minimap_unlocked"));
    let defeat_msg = pack.render_message("shaman.defeat", &[]).unwrap();
    assert!(defeat_msg.contains("relief"));
    // The shaman's words are an actor speech line, not part of the narration
    assert!(!defeat_msg.contains("dead do not stay here"));
    let defeat_line = pack.render_message("shaman.defeat_line", &[]).unwrap();
    assert!(defeat_line.contains("dead do not stay here"));

    let mut state = cinder_core::engine::state::WorldState::new(&pack);
    state.current_room_id = "r5c5".to_string();

    // Reduce shaman hp to 1, then Layla attacks to defeat it
    state
        .actor_stats
        .entry("goblin-shaman".to_string())
        .or_default()
        .insert("hp".to_string(), 1);

    let output = cinder_core::engine::reducer::apply_events(
        &mut state,
        &pack,
        &[cinder_core::engine::events::TimestampedWorldEvent::now(
            cinder_core::engine::events::WorldEvent::ActorCommandUsed {
                actor_id: "player".to_string(),
                actor_name: "Layla".to_string(),
                room_id: "r5c5".to_string(),
                command_id: "attack".to_string(),
                target_room_id: None,
                target_actor_id: Some("goblin-shaman".to_string()),
                target_actor_name: Some("goblin shaman".to_string()),
                context_label: None,
                feature_id: None,
                consumable_id: None,
                freeform_text: None,
            },
        )],
    );

    assert_eq!(state.story_vars.get("shaman_defeated"), Some("true"));
    // Verify shaman-ring acquired by player upon defeat without attacking mobs
    assert!(state.has_item("shaman-ring"));
    // Verify narration lines include shaman.defeat, shaman.reveal, shaman.memory, and handler.minimap_unlocked
    let texts: Vec<&str> = output
        .lines
        .0
        .iter()
        .map(|line| line.text.as_str())
        .collect();
    assert!(texts.iter().any(|t| t.contains("dead do not stay here")));
    assert!(texts.iter().any(|t| t.contains("rough stair descends")));
    assert!(texts.iter().any(|t| t.contains("Go board")));
    assert!(texts.iter().any(|t| t.contains("minimap")));
    assert!(output.lines.0.iter().any(|line| {
        line.kind == cinder_core::engine::narrative::NarrativeLineKind::Channel
            && line.text.contains("Handler:")
            && line.text.contains("minimap")
    }));
}

#[test]
fn goblin_shaman_defeat_via_runtime_turn() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = cinder_core::engine::state::WorldState::new(&pack);
    state.current_room_id = "r5c5".to_string();
    state
        .actor_stats
        .entry("goblin-shaman".to_string())
        .or_default()
        .insert("hp".to_string(), 1);

    let runtime = cinder_core::engine::runtime::CinderRuntime::from_state(pack, state, false)
        .expect("runtime creates");
    let outcome = runtime.run_turn("attack goblin shaman").expect("turn runs");
    println!("OUTCOME LINES: {:#?}", outcome.lines);
    assert!(outcome.lines.iter().any(|line| {
        line.kind == cinder_core::engine::narrative::NarrativeLineKind::Channel
            && line.text.contains("Handler:")
            && line.text.contains("minimap")
    }));
}

#[test]
fn elf_king_defeat_narrates_dungeon_master_myth() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let king_speech = pack.render_message("king.speech", &[]).unwrap();

    // Must refer to the master by the canonical in-game name, "the dark lord"
    assert!(king_speech.contains("the dark lord"));
    assert!(king_speech.contains("the night"));
    // Never refer to the master by names like "dungeon master"
    assert!(!king_speech.to_lowercase().contains("dungeon master"));

    let mut state = cinder_core::engine::state::WorldState::new(&pack);
    state.current_room_id = "d8c5".to_string();
    state
        .actor_stats
        .entry("elf-king-5".to_string())
        .or_default()
        .insert("hp".to_string(), 1);

    let output = cinder_core::engine::reducer::apply_events(
        &mut state,
        &pack,
        &[cinder_core::engine::events::TimestampedWorldEvent::now(
            cinder_core::engine::events::WorldEvent::ActorCommandUsed {
                actor_id: "player".to_string(),
                actor_name: "Layla".to_string(),
                room_id: "d8c5".to_string(),
                command_id: "attack".to_string(),
                target_room_id: None,
                target_actor_id: Some("elf-king-5".to_string()),
                target_actor_name: Some("elf king".to_string()),
                context_label: None,
                feature_id: None,
                consumable_id: None,
                freeform_text: None,
            },
        )],
    );

    assert_eq!(state.story_vars.get("elf_king_defeated"), Some("true"));
    // Drops drain-scroll into room
    assert!(
        state
            .loose_room_items("d8c5")
            .iter()
            .any(|(item, _)| item == "drain-scroll")
    );
    // Other elves stand down to neutral
    assert_eq!(
        state.relationship("elf-pawn-1").stance,
        cinder_core::engine::state::ActorStance::Neutral
    );
    // King defeat narrative present
    let texts: Vec<&str> = output
        .lines
        .0
        .iter()
        .map(|line| line.text.as_str())
        .collect();
    assert!(texts.iter().any(|t| t.contains("the dark lord")));
    // Spoken speech line is attributed with character name
    assert!(
        texts
            .iter()
            .any(|t| t.starts_with("elf king:") && t.contains("the dark lord"))
    );
    // Intro prose and aftermath prose are present
    assert!(
        texts
            .iter()
            .any(|t| t.contains("The elf king sinks to one knee"))
    );
    assert!(
        texts
            .iter()
            .any(|t| t.contains("He bows his head. The forest holds its breath."))
    );
}

#[test]
fn leaf_paste_used_via_generic_item_use() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");

    // Must not be exposed as a standalone action in actions.json
    assert!(pack.action("use_leaf-paste").is_none());

    let mut state = cinder_core::engine::state::WorldState::new(&pack);
    state.add_item("leaf-paste");
    state
        .actor_stats
        .entry("player".to_string())
        .or_default()
        .insert("hp".to_string(), 5);

    let dialogue =
        std::sync::Arc::new(cinder_core::engine::dialogue::ScriptedDialogueGenerator::new());
    let runtime =
        cinder_core::engine::runtime::CinderRuntime::with_dialogue_generator(pack, state, dialogue)
            .expect("runtime creates");

    let outcome = runtime.run_turn("use leaf paste").expect("turn runs");

    let end_state = runtime.export_state().unwrap();
    let player_hp = end_state.actor_stat("player", "hp");
    assert_eq!(player_hp, 9); // healed 4 hp
    assert!(!end_state.has_item("leaf-paste"));

    let text = outcome.text();
    assert!(text.contains(
        "You press the leaf paste into your wounds. It goes warm and green, and the hurt eases."
    ));
}

#[test]
fn bitter_moss_used_via_generic_item_use() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");

    // Must not be exposed as a standalone action in actions.json
    assert!(pack.action("use_salve").is_none());
    assert!(pack.action("use_bitter-moss").is_none());

    let mut state = cinder_core::engine::state::WorldState::new(&pack);
    state.add_item("bitter-moss");
    state
        .actor_stats
        .entry("player".to_string())
        .or_default()
        .insert("hp".to_string(), 5);

    let dialogue =
        std::sync::Arc::new(cinder_core::engine::dialogue::ScriptedDialogueGenerator::new());
    let runtime = cinder_core::engine::runtime::CinderRuntime::with_dialogue_generator(
        pack.clone(),
        state,
        dialogue.clone(),
    )
    .expect("runtime creates");

    let outcome = runtime.run_turn("use bitter moss").expect("turn runs");

    let end_state = runtime.export_state().unwrap();
    let player_hp = end_state.actor_stat("player", "hp");
    assert_eq!(player_hp, 9); // healed 4 hp
    assert!(!end_state.has_item("bitter-moss"));

    let text = outcome.text();
    assert!(
        text.contains(
            "You press the bitter moss into your wounds. The herbs bite, then settle warm."
        )
    );

    // Also verify alias "use moss" resolves cleanly
    let mut state_moss = cinder_core::engine::state::WorldState::new(&pack);
    state_moss.add_item("bitter-moss");
    let runtime_moss = cinder_core::engine::runtime::CinderRuntime::with_dialogue_generator(
        pack, state_moss, dialogue,
    )
    .expect("runtime creates");
    let outcome_moss = runtime_moss.run_turn("use moss").expect("turn runs");
    assert!(
        outcome_moss.text().contains(
            "You press the bitter moss into your wounds. The herbs bite, then settle warm."
        )
    );
}

#[test]
fn transition_commentary_falls_back_when_no_llm() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = cinder_core::engine::state::WorldState::new(&pack);
    state.current_room_id = "r5c5".to_string();
    state.story_vars.set_unchecked("shaman_defeated", "true");

    let dialogue =
        std::sync::Arc::new(cinder_core::engine::dialogue::ScriptedDialogueGenerator::new());
    let runtime =
        cinder_core::engine::runtime::CinderRuntime::with_dialogue_generator(pack, state, dialogue)
            .expect("runtime creates");

    let outcome = runtime.run_turn("go down").expect("turn runs");
    assert!(
        outcome
            .text()
            .contains("Floor two. A glowing wood under a cave")
    );

    // Climbing up and descending again does NOT repeat the fallback commentary
    let _ = runtime.run_turn("go up").expect("turn runs");
    let outcome2 = runtime.run_turn("go down").expect("turn runs");
    assert!(
        !outcome2
            .text()
            .contains("Floor two. A glowing wood under a cave")
    );
}

#[test]
fn transition_commentary_tailored_when_llm_responds() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = cinder_core::engine::state::WorldState::new(&pack);
    state.current_room_id = "r5c5".to_string();
    state.story_vars.set_unchecked("shaman_defeated", "true");

    let dialogue = std::sync::Arc::new(
        cinder_core::engine::dialogue::ScriptedDialogueGenerator::new().with_transition_commentary(
            "d1c1",
            "Well, you survived the mines without getting turned into soup. Welcome to the damp mushroom patch.",
        ),
    );
    let runtime =
        cinder_core::engine::runtime::CinderRuntime::with_dialogue_generator(pack, state, dialogue)
            .expect("runtime creates");
    runtime
        .set_transcript(vec![
            "Layla attacked the goblin shaman.".to_string(),
            "The shaman fell into the dust.".to_string(),
        ])
        .expect("set transcript");

    let outcome = runtime.run_turn("go down").expect("turn runs");
    assert!(
        outcome
            .text()
            .contains("Well, you survived the mines without getting turned into soup")
    );
    assert!(
        !outcome
            .text()
            .contains("Floor two. A glowing wood under a cave")
    );
}

#[test]
fn transition_commentary_floor_3_tailored() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = cinder_core::engine::state::WorldState::new(&pack);
    state.current_room_id = "d8c5".to_string();
    state.story_vars.set_unchecked("elf_king_defeated", "true");

    let dialogue = std::sync::Arc::new(
        cinder_core::engine::dialogue::ScriptedDialogueGenerator::new().with_transition_commentary(
            "oan",
            "You actually toppled the elf king. Try not to break whatever is left down on the board.",
        ),
    );
    let runtime =
        cinder_core::engine::runtime::CinderRuntime::with_dialogue_generator(pack, state, dialogue)
            .expect("runtime creates");

    let outcome = runtime.run_turn("go down").expect("turn runs");
    assert!(outcome.text().contains(
        "You actually toppled the elf king. Try not to break whatever is left down on the board."
    ));
    assert!(!outcome.text().contains("Floor three. The actual board"));
}

#[test]
fn transition_commentary_two_messages_summary_and_introduction() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = cinder_core::engine::state::WorldState::new(&pack);
    state.current_room_id = "r5c5".to_string();
    state.story_vars.set_unchecked("shaman_defeated", "true");

    let summary = "Floor one cleared. You dismantled that shaman faster than HR revokes badge access after two missed standups.";
    let intro = "Welcome to the luminescent moss district. Try not to inhale the spores; hazard pay hasn't cleared finance yet.";

    let dialogue = std::sync::Arc::new(
        cinder_core::engine::dialogue::ScriptedDialogueGenerator::new()
            .with_transition_commentary_lines("d1c1", vec![summary.to_string(), intro.to_string()]),
    );
    let runtime =
        cinder_core::engine::runtime::CinderRuntime::with_dialogue_generator(pack, state, dialogue)
            .expect("runtime creates");

    let outcome = runtime.run_turn("go down").expect("turn runs");

    // Both messages appear in the overall text
    assert!(outcome.text().contains(summary));
    assert!(outcome.text().contains(intro));
    assert!(
        !outcome
            .text()
            .contains("Floor two. A glowing wood under a cave")
    );

    // Both messages are emitted as distinct Channel lines
    let channel_lines: Vec<_> = outcome
        .lines
        .iter()
        .filter(|l| l.kind == cinder_core::engine::narrative::NarrativeLineKind::Channel)
        .collect();

    // At least the first two channel lines are the descent commentary (summary and intro)
    assert!(channel_lines.len() >= 2);
    assert!(channel_lines[0].text.starts_with("Handler:"));
    assert!(channel_lines[0].text.contains(summary));
    assert!(channel_lines[1].text.starts_with("Handler:"));
    assert!(channel_lines[1].text.contains(intro));
}

#[test]
fn transition_commentary_via_switch_room_view() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = cinder_core::engine::state::WorldState::new(&pack);
    state.current_room_id = "r5c5".to_string();
    state.story_vars.set_unchecked("shaman_defeated", "true");

    let summary = "Floor one cleared. Nice work.";
    let intro = "Floor two ahead. Watch the mushrooms.";

    let dialogue = std::sync::Arc::new(
        cinder_core::engine::dialogue::ScriptedDialogueGenerator::new()
            .with_transition_commentary_lines("d1c1", vec![summary.to_string(), intro.to_string()]),
    );
    let runtime =
        cinder_core::engine::runtime::CinderRuntime::with_dialogue_generator(pack, state, dialogue)
            .expect("runtime creates");

    // Running 'go down' via generic command pipeline
    let outcome = runtime.run_turn("go down").expect("turn runs");

    assert!(outcome.text().contains(summary));
    assert!(outcome.text().contains(intro));
    assert!(
        !outcome
            .text()
            .contains("Floor two. A glowing wood under a cave")
    );

    let channel_lines: Vec<_> = outcome
        .lines
        .iter()
        .filter(|l| l.kind == cinder_core::engine::narrative::NarrativeLineKind::Channel)
        .collect();

    assert!(channel_lines.len() >= 2);
    assert!(channel_lines[0].text.starts_with("Handler:"));
    assert!(channel_lines[0].text.contains(summary));
    assert!(channel_lines[1].text.starts_with("Handler:"));
    assert!(channel_lines[1].text.contains(intro));
}

#[test]
fn transition_commentary_only_plays_on_first_descent() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = cinder_core::engine::state::WorldState::new(&pack);
    state.current_room_id = "r5c5".to_string();
    state.story_vars.set_unchecked("shaman_defeated", "true");
    state.scripted_sequences.clear();

    let summary = "Floor one cleared. Nice work.";
    let intro = "Floor two ahead. Watch the mushrooms.";

    let dialogue = std::sync::Arc::new(
        cinder_core::engine::dialogue::ScriptedDialogueGenerator::new()
            .with_transition_commentary_lines("d1c1", vec![summary.to_string(), intro.to_string()]),
    );
    let runtime =
        cinder_core::engine::runtime::CinderRuntime::with_dialogue_generator(pack, state, dialogue)
            .expect("runtime creates");

    // 1. First descent: 'go down'
    let outcome1 = runtime
        .run_turn("go down")
        .expect("first descent turn runs");
    assert!(outcome1.text().contains(summary));
    assert!(outcome1.text().contains(intro));
    let channel_lines1: Vec<_> = outcome1
        .lines
        .iter()
        .filter(|l| l.kind == cinder_core::engine::narrative::NarrativeLineKind::Channel)
        .collect();
    assert!(channel_lines1.iter().any(|l| l.text.contains(summary)));
    assert!(channel_lines1.iter().any(|l| l.text.contains(intro)));

    // 2. Climb back up: 'go up'
    let outcome_up = runtime.run_turn("go up").expect("climb up turn runs");
    assert!(!outcome_up.text().contains(summary));
    assert!(!outcome_up.text().contains(intro));

    // 3. Second descent: 'go down' again
    let outcome2 = runtime
        .run_turn("go down")
        .expect("second descent turn runs");
    // Neither tailored commentary nor fallback line should appear
    assert!(!outcome2.text().contains(summary));
    assert!(!outcome2.text().contains(intro));
    assert!(
        !outcome2
            .text()
            .contains("Floor two. A glowing wood under a cave")
    );
    let channel_lines2: Vec<_> = outcome2
        .lines
        .iter()
        .filter(|l| l.kind == cinder_core::engine::narrative::NarrativeLineKind::Channel)
        .collect();
    assert!(
        channel_lines2.is_empty(),
        "expected no channel lines on second descent, got: {:?}",
        channel_lines2
    );
    // Normal room description should still appear
    assert!(outcome2.text().contains("The Mushroom Grove"));
}

#[test]
fn player_can_take_and_drop_shaman_ring_with_various_phrasings() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = cinder_core::engine::state::WorldState::new(&pack);
    state.current_room_id = "r5c5".to_string();
    state.add_item_to_storage(
        "shaman-ring",
        cinder_core::content::types::ItemStorageTarget::CurrentRoom,
        "r5c5",
    );

    let dialogue =
        std::sync::Arc::new(cinder_core::engine::dialogue::ScriptedDialogueGenerator::new());
    let runtime = cinder_core::engine::runtime::CinderRuntime::with_dialogue_generator(
        pack.clone(),
        state,
        dialogue.clone(),
    )
    .expect("runtime creates");

    // 1. take shaman-ring (exact id)
    let outcome = runtime.run_turn("take shaman-ring").expect("turn runs");
    assert!(outcome.text().contains("Picked up shaman's ring."));
    {
        let s = runtime.export_state().unwrap();
        assert!(s.has_item("shaman-ring"));
        assert!(s.loose_room_items("r5c5").is_empty());
    }

    // 2. drop shaman-ring (exact id)
    let outcome = runtime.run_turn("drop shaman-ring").expect("turn runs");
    assert!(
        outcome
            .text()
            .contains("Placed shaman's ring on the ground.")
    );
    {
        let s = runtime.export_state().unwrap();
        assert!(!s.has_item("shaman-ring"));
        assert_eq!(
            s.loose_room_items("r5c5"),
            vec![("shaman-ring".to_string(), 1)]
        );
    }

    // 3. take shaman ring (without hyphen)
    let outcome = runtime.run_turn("take shaman ring").expect("turn runs");
    assert!(outcome.text().contains("Picked up shaman's ring."));
    {
        let s = runtime.export_state().unwrap();
        assert!(s.has_item("shaman-ring"));
    }

    // 4. drop shaman ring
    let outcome = runtime.run_turn("drop shaman ring").expect("turn runs");
    assert!(
        outcome
            .text()
            .contains("Placed shaman's ring on the ground.")
    );

    // 5. take ring (substring/token match)
    let outcome = runtime.run_turn("take ring").expect("turn runs");
    assert!(outcome.text().contains("Picked up shaman's ring."));
    {
        let s = runtime.export_state().unwrap();
        assert!(s.has_item("shaman-ring"));
    }

    // 6. equip ring
    let outcome = runtime.run_turn("equip ring").expect("turn runs");
    assert!(outcome.text().contains("Equipped shaman's ring."));

    // 7. drop ring while equipped is rejected
    let outcome = runtime.run_turn("drop ring").expect("turn runs");
    assert!(outcome.text().contains("Unequip it before dropping"));

    // 8. take off ring (unequip via take off phrase)
    let outcome = runtime.run_turn("take off ring").expect("turn runs");
    assert!(outcome.text().contains("Unequipped shaman's ring."));

    // 9. drop ring now succeeds
    let outcome = runtime.run_turn("drop ring").expect("turn runs");
    assert!(
        outcome
            .text()
            .contains("Placed shaman's ring on the ground.")
    );

    // 10. take the shaman's ring (with article and apostrophe)
    let outcome = runtime
        .run_turn("take the shaman's ring")
        .expect("turn runs");
    assert!(outcome.text().contains("Picked up shaman's ring."));
    {
        let s = runtime.export_state().unwrap();
        assert!(s.has_item("shaman-ring"));
        assert!(s.loose_room_items("r5c5").is_empty());
    }

    // 11. taking an anchored trace mark is denied
    {
        let mut s = runtime.export_state().unwrap();
        s.add_item_to_storage(
            "drain-sigil",
            cinder_core::content::types::ItemStorageTarget::CurrentRoom,
            "r5c5",
        );
        // Note: CinderRuntime doesn't have an import_state, so we test take on a fresh runtime with the sigil in room
        let mut sigil_state = cinder_core::engine::state::WorldState::new(&pack);
        sigil_state.current_room_id = "r5c5".to_string();
        sigil_state.add_item_to_storage(
            "drain-sigil",
            cinder_core::content::types::ItemStorageTarget::CurrentRoom,
            "r5c5",
        );
        let sigil_runtime = cinder_core::engine::runtime::CinderRuntime::with_dialogue_generator(
            pack.clone(),
            sigil_state,
            dialogue.clone(),
        )
        .expect("runtime creates");
        let sigil_outcome = sigil_runtime
            .run_turn("take drain-sigil")
            .expect("turn runs");
        assert!(sigil_outcome.text().contains("anchored to the floor"));
    }
}

#[test]
#[ignore]
fn live_test_synapse_transition_commentary() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = cinder_core::engine::state::WorldState::new(&pack);
    state.current_room_id = "r5c5".to_string();
    state.story_vars.set_unchecked("shaman_defeated", "true");
    state.transcript = vec![
        "Layla struck the goblin shaman with iron chisel.".to_string(),
        "The goblin shaman fell into dust.".to_string(),
        "Layla picked up shaman's ring.".to_string(),
    ];
    let runtime = cinder_core::engine::runtime::CinderRuntime::from_state(pack, state, false)
        .expect("runtime from state");
    let outcome = runtime.run_turn("go down").expect("turn runs");
    println!("DESCENT OUTCOME TEXT:\n{}", outcome.text());
    for (i, line) in outcome.lines.iter().enumerate() {
        println!("LINE {i} [{:?}]: {}", line.kind, line.text);
    }
}

#[test]
fn follow_and_unfollow_commands_and_panel_options() {
    let pack = load_named_pack("aera", Some("en")).expect("aera loads and validates");
    let state = cinder_core::engine::state::WorldState::new(&pack);
    let dialogue =
        std::sync::Arc::new(cinder_core::engine::dialogue::ScriptedDialogueGenerator::new());
    let runtime =
        cinder_core::engine::runtime::CinderRuntime::with_dialogue_generator(pack, state, dialogue)
            .expect("runtime creates");

    // Exit options generate executable 'go <label>' commands
    let exit_options = runtime
        .panel_options(&cinder_core::content::types::PanelDataSource::Exits)
        .expect("exit options build");
    assert!(!exit_options.is_empty());
    for opt in &exit_options {
        assert!(
            opt.command.starts_with("go "),
            "command should be 'go <label>', got: {}",
            opt.command
        );
    }

    // Follow options generate executable 'unfollow' and 'follow <actor>' commands
    let follow_options = runtime
        .panel_options(&cinder_core::content::types::PanelDataSource::FollowActors)
        .expect("follow options build");
    assert!(!follow_options.is_empty());
    assert_eq!(follow_options[0].command, "unfollow");
    assert!(
        follow_options
            .iter()
            .skip(1)
            .all(|opt| opt.command.starts_with("follow "))
    );

    // Test follow command execution
    let outcome = runtime.run_turn("follow ren").expect("follow ren runs");
    assert!(outcome.text().contains("following Ren") || outcome.text().contains("following ren"));
    assert_eq!(
        runtime.followed_actor_id().unwrap(),
        Some("ren".to_string())
    );

    // Test unfollow command execution
    let outcome = runtime.run_turn("unfollow").expect("unfollow runs");
    assert!(outcome.text().contains("stopped following"));
    assert_eq!(runtime.followed_actor_id().unwrap(), None);

    // Test follow none
    let outcome = runtime.run_turn("follow none").expect("follow none runs");
    assert!(outcome.text().contains("stopped following"));
    assert_eq!(runtime.followed_actor_id().unwrap(), None);
}

#[test]
fn player_starting_inventory_seeds_from_actor_definition() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let player = pack.actor("player").expect("player actor exists");
    assert_eq!(
        player.initial_inventory.get("magic-chalk"),
        Some(&DropSpec::Always(1))
    );

    let state = cinder_core::engine::state::WorldState::new(&pack);
    assert_eq!(state.player_inventory.get("magic-chalk"), Some(&1));
}

#[test]
fn layla_trace_requires_magic_chalk_in_inventory() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let trace = pack.action("trace").expect("trace action exists");
    assert_eq!(
        trace.available.requires_item.as_deref(),
        Some("magic-chalk")
    );

    let mut state = cinder_core::engine::state::WorldState::new(&pack);
    assert_eq!(state.player_inventory.get("magic-chalk"), Some(&1));
    assert!(cinder_core::engine::turn_policies::action_is_available(
        &pack,
        &state,
        trace,
        &state.current_room_id
    ));

    // Remove magic-chalk from Layla's inventory (simulating giving it to an ally)
    state.remove_item("magic-chalk");
    assert_eq!(state.player_inventory.get("magic-chalk"), None);
    assert!(!cinder_core::engine::turn_policies::action_is_available(
        &pack,
        &state,
        trace,
        &state.current_room_id
    ));

    // Running trace turn without chalk returns ActionRejected with missing item message
    let runtime =
        cinder_core::engine::runtime::CinderRuntime::from_state(pack.clone(), state.clone(), false)
            .expect("runtime from state");
    let outcome = runtime.run_turn("trace charm-sigil").expect("turn runs");
    assert!(outcome.text().contains("magic chalk"));

    // Giving chalk back restores trace availability
    state.add_item("magic-chalk");
    assert!(cinder_core::engine::turn_policies::action_is_available(
        &pack,
        &state,
        trace,
        &state.current_room_id
    ));
}

#[test]
fn trace_deducts_mp_and_recovers_over_time() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let state = cinder_core::engine::state::WorldState::new(&pack);

    let initial_mp = state.actor_stat("player", "mp");
    assert_eq!(initial_mp, 10);

    let runtime =
        cinder_core::engine::runtime::CinderRuntime::from_state(pack.clone(), state, false)
            .expect("runtime from state");

    // Tracing charm-sigil costs 1 MP (10 -> 9 MP)
    let outcome = runtime.run_turn("trace charm-sigil").expect("turn runs");
    assert!(
        !outcome.text().contains("not enough magic"),
        "outcome: {}",
        outcome.text()
    );

    let state_after = runtime.export_state().unwrap();
    let mp_after = state_after.actor_stat("player", "mp");
    assert_eq!(mp_after, 9, "Charm sigil costs 1 MP");

    // Tracing drain-sigil (after unlocking) costs 2 MP (9 -> 7 MP)
    let mut state_with_drain = state_after;
    state_with_drain
        .story_vars
        .set_unchecked("knows_drain", "true");
    let runtime2 = cinder_core::engine::runtime::CinderRuntime::from_state(
        pack.clone(),
        state_with_drain,
        false,
    )
    .expect("runtime2");

    let outcome2 = runtime2.run_turn("trace drain-sigil").expect("turn runs");
    assert!(
        !outcome2.text().contains("not enough magic"),
        "outcome: {}",
        outcome2.text()
    );

    let state2 = runtime2.export_state().unwrap();
    // Turn 2: drain sigil costs 2 MP (9 - 2 = 7 MP). Same-turn regen is suppressed on spellcast turns.
    assert_eq!(
        state2.actor_stat("player", "mp"),
        7,
        "Drain sigil costs 2 MP (9 -> 7 MP)"
    );

    // MP regenerates over time on 5-minute tick intervals (+1 MP every 5 minutes)
    // Moving around advances time by 1 minute each step
    let _ = runtime2.run_turn("south").expect("move south"); // minute 2 -> 3 (no interval crossed)
    let s_t1 = runtime2.export_state().unwrap();
    assert_eq!(s_t1.actor_stat("player", "mp"), 7);

    let _ = runtime2.run_turn("north").expect("move north"); // minute 3 -> 4 (no interval crossed)
    let s_t2 = runtime2.export_state().unwrap();
    assert_eq!(s_t2.actor_stat("player", "mp"), 7);

    let _ = runtime2.run_turn("south").expect("move south"); // minute 4 -> 5 (5-min interval crossed -> +1 MP)
    let s_t3 = runtime2.export_state().unwrap();
    assert_eq!(
        s_t3.actor_stat("player", "mp"),
        8,
        "MP recovered 1 point at minute 5"
    );

    // Insufficient MP check (fresh room without existing marks)
    let mut state_low_mp = cinder_core::engine::state::WorldState::new(&pack);
    state_low_mp
        .actor_stats
        .entry("player".to_string())
        .or_default()
        .insert("mp".to_string(), 0);
    let runtime3 =
        cinder_core::engine::runtime::CinderRuntime::from_state(pack.clone(), state_low_mp, false)
            .expect("runtime3");
    let outcome3 = runtime3.run_turn("trace charm-sigil").expect("turn runs");
    assert!(
        outcome3.text().contains("You do not have enough magic"),
        "got: {}",
        outcome3.text()
    );
    let s_t3 = runtime3.export_state().unwrap();
    assert_eq!(
        s_t3.actor_stat("player", "mp"),
        0,
        "MP remains 0 on rejection"
    );
}

#[test]
fn layla_shipped_pack_invariants_and_wiring() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    assert!(pack.maps.len() >= 4);
    assert_eq!(
        pack.map_for_room("r1c1")
            .map(|map| (map.id.as_str(), map.rooms.len())),
        Some(("upper-works", 81))
    );
    assert_eq!(
        pack.map_for_room("d8c5").map(|map| map.id.as_str()),
        Some("deep-forest")
    );
    assert_eq!(
        pack.map_for_room("oh").map(|map| map.id.as_str()),
        Some("outer-ring")
    );
    assert_eq!(
        pack.map_for_room("village_square")
            .map(|map| map.id.as_str()),
        Some("the-commoners")
    );
    assert_eq!(pack.settings.periodic_actor_effects.len(), 1);
    assert_eq!(pack.settings.periodic_actor_effects[0].id, "drain_sigil");
    assert_eq!(
        pack.actor("fire-elemental").unwrap().initial_inventory,
        std::collections::BTreeMap::from([("spawn-scroll".to_string(), DropSpec::Always(1))])
    );
    assert_eq!(
        pack.item("spawn-scroll").map(|item| item.use_hook.as_str()),
        Some("item.spawn_scroll_read")
    );
    assert_eq!(
        pack.action("trace")
            .and_then(|action| action.item_creation.as_ref())
            .and_then(|creation| creation.craftable_item_gates.get("spawn-sigil"))
            .map(String::as_str),
        Some("knows_spawn")
    );
    let handler = pack.actor("handler").unwrap();
    assert!(handler.room_id.is_empty(), "handler must be offstage");
    assert_eq!(
        handler
            .initial_relationship
            .as_ref()
            .map(|rel| (rel.stance, rel.follows_player)),
        Some((cinder_core::engine::state::ActorStance::Allied, false))
    );
    assert_eq!(pack.settings.feedback_channel_id.as_str(), "handler-comms");
    assert_eq!(
        pack.message("item.acquired_inventory"),
        Some("Picked up {label}.")
    );
    assert_eq!(
        pack.message_voice("item.acquired_inventory"),
        cinder_core::content::types::PackMessageVoice::System
    );
    assert_eq!(
        pack.message_voice("item.consumed_use"),
        cinder_core::content::types::PackMessageVoice::System
    );
    assert_eq!(
        pack.message_voice("item.takedenied"),
        cinder_core::content::types::PackMessageVoice::Handler
    );
    assert_eq!(
        pack.message_voice("combat.attack_hit"),
        cinder_core::content::types::PackMessageVoice::Narration
    );
    assert_eq!(
        pack.settings
            .party
            .initial_orders
            .get("golem-dark-nw")
            .cloned(),
        Some("follow".to_string())
    );
    assert_eq!(
        pack.settings
            .party
            .initial_orders
            .get("golem-pale-ne")
            .cloned(),
        Some("follow".to_string())
    );
    assert_eq!(pack.settings.party.combat_rules.len(), 4);
    assert_eq!(
        pack.settings.party.combat_rules[0].tier,
        cinder_core::content::types::PartyDecisionTier::Survival
    );
    assert_eq!(
        pack.channel("handler-comms")
            .map(|channel| channel.participants.as_slice()),
        Some(&["player".to_string(), "handler".to_string()][..])
    );
    assert!(pack.opening.system_lines.is_empty());
    assert_eq!(
        pack.opening.opening_sequence_id.as_deref(),
        Some("handler-introduction")
    );
    let opening_sequence = pack.sequence("handler-introduction").unwrap();
    assert_eq!(opening_sequence.steps.len(), 5);
    assert!(matches!(
        &opening_sequence.steps[0],
        cinder_core::content::types::ScriptedLine::Channel {
            speaker_id,
            recipient_id: Some(recipient_id),
            ..
        } if speaker_id == "handler" && recipient_id == "player"
    ));
    assert!(matches!(
        &opening_sequence.steps[1],
        cinder_core::content::types::ScriptedLine::Channel {
            speaker_id,
            recipient_id: Some(recipient_id),
            line,
            ..
        } if speaker_id == "player"
            && recipient_id == "handler"
            && line == "Who's there? Where am I?"
    ));
    assert!(matches!(
        opening_sequence.completion_effects.as_slice(),
        [cinder_core::content::types::AdvanceEffect::SetStoryVar { key, value }]
            if key == "handler_introduced" && value == "true"
    ));
}
