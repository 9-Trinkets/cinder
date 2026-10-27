//! Floor 6: the Control Hall platform is armed by tracing the teleport sigil,
//! and Layla's followers cross when she leaves from a permanent platform.

use cinder_core::content::loader::load_named_pack;
use cinder_core::engine::dialogue::ScriptedDialogueGenerator;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::{ActorStance, WorldState};
use std::sync::Arc;

fn runtime_with(
    setup: impl FnOnce(&cinder_core::content::types::ContentPack, &mut WorldState),
) -> CinderRuntime {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.clock.turn_number = 1;
    state.story_vars.set_unchecked("knows_teleport", "true");
    state.story_vars.set_unchecked("anchor_floor5_gate", "true");
    state
        .combat
        .actor_stats
        .entry("player".to_string())
        .or_default()
        .insert("mp".to_string(), 20);
    setup(&pack, &mut state);
    CinderRuntime::with_dialogue_generator(pack, state, Arc::new(ScriptedDialogueGenerator::new()))
        .expect("runtime creates")
}

#[test]
fn tracing_the_teleport_sigil_wakes_the_platform() {
    let runtime = runtime_with(|pack, s| {
        s.current_room_id = "sanctuary_control_hall".to_string();
        s.story_vars.set_unchecked("seer_defeated", "true");
        s.acquire_player_item(pack, "teleport-sigil");
    });
    assert_ne!(
        runtime
            .export_state()
            .unwrap()
            .story_vars
            .get("anchor_floor6_platform"),
        Some("true")
    );

    let out = runtime.run_turn("trace teleport sigil").expect("trace");
    assert!(
        out.text().contains("pillar of pale light"),
        "{}",
        out.text()
    );
    assert!(
        !out.text().contains("anchoring this room"),
        "the generic chalk line must not also fire: {}",
        out.text()
    );
    let state = runtime.export_state().unwrap();
    assert_eq!(state.story_vars.get("anchor_floor6_platform"), Some("true"));
    assert!(!state.has_chalk_anchor("sanctuary_control_hall"));

    let look = runtime.run_turn("look at platform").expect("look");
    assert!(look.text().contains("rings turn slowly"), "{}", look.text());
}

#[test]
fn the_sanctuary_platform_cannot_be_reached_until_it_is_armed() {
    let runtime = runtime_with(|_, s| s.current_room_id = "courtyard_center".to_string());
    runtime.run_turn("teleport floor 6").expect("turn");
    assert_eq!(runtime.current_room_id().unwrap(), "courtyard_center");
}

#[test]
fn a_following_golem_crosses_with_layla_and_a_fallen_one_does_not() {
    let runtime = runtime_with(|pack, s| {
        s.current_room_id = "courtyard_center".to_string();
        s.story_vars.set_unchecked("anchor_floor6_platform", "true");
        for golem in ["golem-dark-nw", "golem-dark-sw"] {
            s.set_actor_stance(pack, golem, ActorStance::Allied, true);
            s.actors
                .actor_room_overrides
                .insert(golem.to_string(), "courtyard_center".to_string());
        }
        s.combat
            .actor_stats
            .get_mut("golem-dark-sw")
            .expect("stats")
            .insert("hp".to_string(), 0);
    });

    runtime.run_turn("teleport floor 6").expect("teleport");
    assert_eq!(runtime.current_room_id().unwrap(), "sanctuary_control_hall");
    let state = runtime.export_state().unwrap();
    assert_eq!(
        state.actor_room_id("golem-dark-nw", ""),
        "sanctuary_control_hall"
    );
    assert_eq!(state.actor_room_id("golem-dark-sw", ""), "courtyard_center");
}

// --- the Seer must fall first; then the whole party crosses -------------------

fn hall_with_party(seer_dead: bool, einar_alive: bool, astrid_alive: bool) -> CinderRuntime {
    runtime_with(|pack, s| {
        s.current_room_id = "sanctuary_control_hall".to_string();
        s.acquire_player_item(pack, "teleport-sigil");
        s.story_vars.set_unchecked("courtyard_cages_opened", "true");
        if seer_dead {
            s.story_vars.set_unchecked("seer_defeated", "true");
        }
        // Orva came down the stair with Layla.
        s.set_actor_stance(pack, "orva", ActorStance::Allied, true);
        s.actors
            .actor_room_overrides
            .insert("orva".to_string(), "sanctuary_control_hall".to_string());
        // A golem follows her; Astrid and Einar hold the courtyard.
        s.set_actor_stance(pack, "golem-dark-nw", ActorStance::Allied, true);
        for actor in ["golem-dark-nw", "commander_astrid", "einar"] {
            s.actors
                .actor_room_overrides
                .insert(actor.to_string(), "courtyard_center".to_string());
        }
        for (actor, alive) in [("einar", einar_alive), ("commander_astrid", astrid_alive)] {
            if !alive {
                s.combat
                    .actor_stats
                    .get_mut(actor)
                    .expect("stats")
                    .insert("hp".to_string(), 0);
            }
        }
    })
}

#[test]
fn the_platform_stays_dead_while_the_seer_lives() {
    let runtime = hall_with_party(false, true, true);
    let out = runtime.run_turn("trace teleport sigil").expect("trace");
    assert!(
        out.text().contains("holds the platform shut"),
        "{}",
        out.text()
    );
    let state = runtime.export_state().unwrap();
    assert_ne!(state.story_vars.get("anchor_floor6_platform"), Some("true"));
    assert_eq!(
        state.actor_room_id("commander_astrid", ""),
        "courtyard_center"
    );
}

#[test]
fn arming_the_platform_brings_the_whole_party_over_following_her() {
    let runtime = hall_with_party(true, true, true);
    let out = runtime.run_turn("trace teleport sigil").expect("trace");
    let text = out.text();
    assert!(text.contains("Your people have crossed"), "{text}");
    assert!(text.contains("The perimeter is secure"), "{text}");
    assert!(text.contains("We are with you"), "{text}");

    let state = runtime.export_state().unwrap();
    for actor in ["commander_astrid", "einar", "golem-dark-nw", "orva"] {
        assert_eq!(
            state.actor_room_id(actor, ""),
            "sanctuary_control_hall",
            "{actor} should have crossed"
        );
        assert!(state.follows_player(actor), "{actor} should follow her");
    }
}

#[test]
fn einar_and_orva_are_reunited_when_both_are_there() {
    let runtime = hall_with_party(true, true, true);
    let text = runtime
        .run_turn("trace teleport sigil")
        .expect("trace")
        .text();
    assert!(text.contains("one half of a pair of wings"), "{text}");
    assert!(text.contains("I thought they had burned you"), "{text}");
    assert!(text.contains("You are really warm"), "{text}");
}

#[test]
fn there_is_no_reunion_when_einar_has_fallen_and_he_does_not_cross() {
    let runtime = hall_with_party(true, false, true);
    let text = runtime
        .run_turn("trace teleport sigil")
        .expect("trace")
        .text();
    assert!(text.contains("The perimeter is secure"), "{text}");
    assert!(!text.contains("one half of a pair of wings"), "{text}");
    let state = runtime.export_state().unwrap();
    assert_eq!(state.actor_room_id("einar", ""), "courtyard_center");
}

#[test]
fn astrid_stays_silent_if_she_has_fallen() {
    let runtime = hall_with_party(true, true, false);
    let text = runtime
        .run_turn("trace teleport sigil")
        .expect("trace")
        .text();
    assert!(!text.contains("The perimeter is secure"), "{text}");
    assert!(text.contains("one half of a pair of wings"), "{text}");
}
