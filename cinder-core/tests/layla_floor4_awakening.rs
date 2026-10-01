//! Integration tests for Floor 4 Follower Awakening mechanic.

use cinder_core::content::loader::load_named_pack;
use cinder_core::engine::narrative::NarrativeLines;
use cinder_core::engine::reducer::transformations::maybe_apply_transformations;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::WorldState;

#[test]
fn sakhra_awakens_into_jamil_via_zayd_lantern() {
    let pack = load_named_pack("layla", Some("en")).expect("pack loads");
    let mut state = WorldState::new(&pack);

    // Place player and sakhra in village_square
    state.current_room_id = "village_square".to_string();
    state.add_item("zayd-lantern");

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");

    // Before awakening: Sakhra is unawakened
    assert_eq!(
        runtime.actor_display_name("sakhra").unwrap().as_deref(),
        Some("Sakhra")
    );

    // Give lantern to Sakhra -> Sakhra auto-equips (+6 WIS, base 5 WIS -> 11 WIS >= 10)
    let turn_out = runtime
        .run_turn("give battered miner's lantern to sakhra")
        .expect("give lantern");

    let text = turn_out.text();
    assert!(
        text.contains("Zayd. My boy. Run your lamp over me"),
        "awakening fragment present: {text}"
    );
    assert!(
        text.contains("Sakhra is now Jamil"),
        "awakening name change narrated: {text}"
    );

    // After awakening: display name is now Jamil
    assert_eq!(
        runtime.actor_display_name("sakhra").unwrap().as_deref(),
        Some("Jamil")
    );

    let s = runtime.export_state().unwrap();
    assert!(s.is_actor_awakened("sakhra"));
    assert_eq!(
        s.story_vars.get("transformed:sakhra:awakening"),
        Some("true")
    );
    assert!(s.follows_player("sakhra"), "awakened Jamil follows player");

    // Inspecting Jamil shows awakened inspect text
    let inspect_out = runtime.run_turn("look at jamil").expect("look at jamil");
    assert!(
        inspect_out.text().contains("Jamil, once known as Sakhra")
            || inspect_out.text().contains("father of Zayd"),
        "awakened inspect text: {}",
        inspect_out.text()
    );
}

#[test]
fn awakening_persists_when_lantern_transferred_to_another_follower() {
    let pack = load_named_pack("layla", Some("en")).expect("pack loads");
    let mut state = WorldState::new(&pack);

    state.current_room_id = "village_square".to_string();
    state.add_item("zayd-lantern");

    // Also place golem-dark-nw in village_square as ally
    state
        .actor_room_overrides
        .insert("golem-dark-nw".to_string(), "village_square".to_string());
    state.set_stance(
        "golem-dark-nw",
        cinder_core::engine::state::ActorStance::Allied,
    );

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");

    // 1. Awaken Sakhra -> Jamil
    runtime
        .run_turn("give battered miner's lantern to sakhra")
        .expect("give lantern to sakhra");
    assert_eq!(
        runtime.actor_display_name("sakhra").unwrap().as_deref(),
        Some("Jamil")
    );

    // 2. Take lantern back from Jamil
    let take_out = runtime
        .run_turn("take battered miner's lantern from jamil")
        .expect("take lantern back");
    assert!(
        take_out.text().contains("take") || take_out.text().contains("battered miner's lantern")
    );

    // Jamil remains awakened!
    assert_eq!(
        runtime.actor_display_name("sakhra").unwrap().as_deref(),
        Some("Jamil")
    );
    let s = runtime.export_state().unwrap();
    assert!(s.is_actor_awakened("sakhra"));

    // 3. Give lantern to golem-dark-nw (base 4 WIS + 6 = 10 WIS) -> awakens into Orin!
    let orin_turn = runtime
        .run_turn("give battered miner's lantern to dark golem")
        .expect("give lantern to golem");
    let orin_text = orin_turn.text();
    assert!(
        orin_text.contains("Orin"),
        "Orin awakening narrated: {orin_text}"
    );
    assert!(
        orin_text.contains("ringing the stone"),
        "Orin fragment narrated: {orin_text}"
    );
    assert_eq!(
        runtime
            .actor_display_name("golem-dark-nw")
            .unwrap()
            .as_deref(),
        Some("Orin")
    );
}

#[test]
fn hard_limit_shaman_and_king_cannot_awaken() {
    let pack = load_named_pack("layla", Some("en")).expect("pack loads");
    let mut state = WorldState::new(&pack);
    let mut lines = NarrativeLines::default();

    // Set high wisdom on shaman and elf king
    state
        .actor_stats
        .entry("goblin-shaman".to_string())
        .or_default()
        .insert("wisdom".to_string(), 20);
    state
        .actor_stats
        .entry("elf-king-5".to_string())
        .or_default()
        .insert("wisdom".to_string(), 20);

    let shaman_awakened =
        maybe_apply_transformations(&mut state, &pack, "goblin-shaman", &mut lines);
    assert!(!shaman_awakened, "goblin-shaman cannot awaken");
    assert!(!state.is_actor_awakened("goblin-shaman"));

    let king_awakened = maybe_apply_transformations(&mut state, &pack, "elf-king-5", &mut lines);
    assert!(!king_awakened, "elf-king cannot awaken");
    assert!(!state.is_actor_awakened("elf-king-5"));
}

#[test]
fn sakhra_is_not_in_party_at_game_start() {
    let pack = load_named_pack("layla", Some("en")).expect("pack loads");
    let state = WorldState::new(&pack);

    assert!(
        !state.is_party_member(&pack, "sakhra"),
        "sakhra should not be a party member at game start"
    );
    assert!(
        !state.follows_player("sakhra"),
        "sakhra should not follow the player at game start"
    );
    assert_eq!(
        state.stance("sakhra"),
        cinder_core::engine::state::ActorStance::Neutral,
        "sakhra should be neutral before awakening"
    );
}

#[test]
fn transformation_with_empty_fragment_omits_empty_quotes() {
    let mut pack = load_named_pack("layla", Some("en")).expect("pack loads");
    if let Some(actor) = pack.actors.iter_mut().find(|a| a.id == "sakhra") {
        actor.transformations = vec![cinder_core::content::types::ActorTransformation {
            id: "test_empty_fragment".to_string(),
            trigger: cinder_core::content::types::TransformationTrigger::Stat {
                stat: "wisdom".to_string(),
                gte: 1,
            },
            rename: Some(cinder_core::content::types::TransformationRename {
                name: "Jamil".to_string(),
                fragment: String::new(),
                ..Default::default()
            }),
            ..Default::default()
        }];
    }
    let mut state = WorldState::new(&pack);
    let mut lines = NarrativeLines::default();
    let transformed = maybe_apply_transformations(&mut state, &pack, "sakhra", &mut lines);
    assert!(transformed);
    assert!(!lines.0.is_empty());
    for line in &lines.0 {
        assert_ne!(line.text.trim(), "\"\"");
        assert!(!line.text.trim().is_empty());
    }
}
