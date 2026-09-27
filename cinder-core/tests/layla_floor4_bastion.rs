//! Integration tests for Floor 4 Command Bastion:
//! 1. Bastion entry narrative scene and door slam lock-in.
//! 2. Boss combat, Malik defeat drops commander-safe-key, doors unlock.
//! 3. Unlocking the safe consumes the key, awards teleport-scroll, and advances mq_find_scroll.
//! 4. Reading the teleport-scroll teaches teleport-sigil.

use cinder_core::content::loader::load_named_pack;
use cinder_core::engine::dialogue::ScriptedDialogueGenerator;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::WorldState;
use std::sync::Arc;

#[test]
fn floor4_bastion_entry_narrative_and_doors_lock_in() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "south_steam_gantry".to_string();
    state.story_vars.set_unchecked("fortress_gate_open", "true");

    let dialogue = Arc::new(ScriptedDialogueGenerator::new());
    let runtime = CinderRuntime::with_dialogue_generator(pack, state, dialogue)
        .expect("runtime creates");

    // 1. Move east into command_bastion
    let entry_outcome = runtime.run_turn("east").expect("enter command_bastion");
    assert_eq!(runtime.current_room_id().unwrap(), "command_bastion");

    let entry_text = entry_outcome.text();

    // Verify narrative scene and dialogue exchange
    assert!(
        entry_text.contains("Captain Malik and Priest Harun are locked in a venomous exchange"),
        "Bastion entry scene text missing in: {entry_text}"
    );
    assert!(
        entry_text.contains("Priest Harun (to Captain Malik): The transport cart sits idle on the gantry, Malik"),
        "Priest Harun speech missing or misformatted in: {entry_text}"
    );
    assert!(
        entry_text.contains("Captain Malik (to Priest Harun): Hold your tongue, priest!"),
        "Captain Malik reply speech missing or misformatted in: {entry_text}"
    );
    assert!(
        entry_text.contains("massive iron security blast doors drop from the vaulted ceiling with a deafening SLAM"),
        "Doors slam narration missing in: {entry_text}"
    );
    assert!(
        entry_text.contains("Captain Malik (to Layla): The infiltrator! Draw steel, Harun—nobody leaves this bastion alive!"),
        "Malik alert speech missing or misformatted in: {entry_text}"
    );

    // Verify story var set
    let state_after_entry = runtime.export_state().unwrap();
    assert_eq!(
        state_after_entry.story_vars.get("bastion_scene_seen"),
        Some("true"),
        "bastion_scene_seen must be set"
    );

    // 2. Try to flee west back to south_steam_gantry -> blocked!
    let _flee_west = runtime.run_turn("west").expect("try moving west");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "command_bastion",
        "Player must remain locked in command_bastion while Malik is undefeated"
    );

    // 3. Try to flee northwest to east_sentry_walk -> blocked!
    let _flee_nw = runtime.run_turn("northwest").expect("try moving northwest");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "command_bastion",
        "Player must remain locked in command_bastion while Malik is undefeated"
    );
}

#[test]
fn floor4_bastion_malik_defeat_safe_unlock_and_teleport_scroll() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "command_bastion".to_string();
    state.story_vars.set_unchecked("fortress_gate_open", "true");
    state.story_vars.set_unchecked("bastion_scene_seen", "true");

    let dialogue = Arc::new(ScriptedDialogueGenerator::new());
    let runtime = CinderRuntime::with_dialogue_generator(pack.clone(), state, dialogue)
        .expect("runtime creates");

    // 1. Initial state: exits are locked
    let pre_state = runtime.export_state().unwrap();
    assert!(pre_state.story_vars.get("malik_defeated").is_none());
    assert!(!pre_state.actor_has_item("player", "commander-safe-key"));
    assert!(!pre_state.actor_has_item("player", "teleport-scroll"));

    // 2. Defeat Malik: loot key and malik_defeated set
    let mut defeated_state = runtime.export_state().unwrap();
    defeated_state.story_vars.set_unchecked("malik_defeated", "true");
    defeated_state.add_item("commander-safe-key");
    defeated_state.actor_add_item("player", "commander-safe-key");
    defeated_state
        .active_objective_stage_ids
        .push("mq_find_scroll".to_string());

    let dialogue2 = Arc::new(ScriptedDialogueGenerator::new());
    let runtime2 = CinderRuntime::with_dialogue_generator(pack.clone(), defeated_state, dialogue2)
        .expect("runtime creates");

    // Exits must now be open
    let _move_out = runtime2.run_turn("west").expect("turn runs");
    assert_eq!(
        runtime2.current_room_id().unwrap(),
        "south_steam_gantry",
        "Exits must unlock once Malik is defeated"
    );

    // Step back into bastion
    let _ = runtime2.run_turn("east").expect("turn runs");
    assert_eq!(runtime2.current_room_id().unwrap(), "command_bastion");

    // 3. Unlock safe
    let unlock_outcome = runtime2.run_turn("unlock safe").expect("unlock safe command");
    let unlock_text = unlock_outcome.text();

    assert!(
        unlock_text.contains("You insert Commander Malik's heavy brass key into the safe's dual tumblers"),
        "Unlock safe event text missing in: {unlock_text}"
    );

    let state_after_safe = runtime2.export_state().unwrap();
    assert_eq!(
        state_after_safe.story_vars.get("safe_unlocked"),
        Some("true"),
        "safe_unlocked must be set"
    );
    assert!(
        !state_after_safe.actor_has_item("player", "commander-safe-key"),
        "commander-safe-key must be consumed upon opening the safe"
    );
    assert!(
        state_after_safe.actor_has_item("player", "teleport-scroll"),
        "player must now possess teleport-scroll"
    );

    // Verify quest advance
    let objectives = runtime2.current_objective_summaries().unwrap();
    assert!(
        objectives
            .iter()
            .any(|o| o.quest_id.as_deref() == Some("teleport_scroll")
                && o.summary.contains("Activate the teleport platform")),
        "Quest must advance to mq_use_teleport_platform once scroll is acquired: {:?}",
        objectives
    );

    // 4. Read teleport scroll
    let read_outcome = runtime2.run_turn("read teleport scroll").expect("read scroll");
    let read_text = read_outcome.text();
    assert!(
        read_text.contains("learned the Teleportation Sigil")
            || read_text.contains("Teleportation Sigil"),
        "Read scroll outcome missing in: {read_text}"
    );

    let final_state = runtime2.export_state().unwrap();
    assert_eq!(
        final_state.story_vars.get("knows_teleport"),
        Some("true"),
        "knows_teleport must be true after reading the scroll"
    );
}
