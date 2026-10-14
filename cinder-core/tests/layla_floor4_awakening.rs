//! Integration tests for Follower Awakening mechanic via spirit restoration.

use cinder_core::content::loader::load_named_pack;
use cinder_core::engine::narrative::NarrativeLines;
use cinder_core::engine::reducer::transformations::maybe_apply_transformations;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::WorldState;

#[test]
fn sakhra_awakens_into_jamil_when_awakening_triggered() {
    let pack = load_named_pack("layla", Some("en")).expect("pack loads");
    let mut state = WorldState::new(&pack);

    // Place player and sakhra in village_square
    state.current_room_id = "village_square".to_string();

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");

    // Before awakening: Sakhra is unawakened
    assert_eq!(
        runtime.actor_display_name("sakhra").unwrap().as_deref(),
        Some("Sakhra")
    );

    // Trigger awakening via story var (e.g. from spirit shard binding)
    let mut state = runtime.export_state().unwrap();
    state.story_vars.set_unchecked("awakened:sakhra", "true");
    let mut lines = NarrativeLines::default();
    let applied = maybe_apply_transformations(&mut state, &pack, "sakhra", &mut lines);
    assert!(applied);

    let text = lines
        .0
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        text.contains("Zayd. My boy. Run your lamp over me"),
        "awakening fragment present: {text}"
    );
    assert!(
        text.contains("Sakhra is now Jamil"),
        "awakening name change narrated: {text}"
    );

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");

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
fn awakening_persists_once_triggered() {
    let pack = load_named_pack("layla", Some("en")).expect("pack loads");
    let mut state = WorldState::new(&pack);

    state.current_room_id = "village_square".to_string();

    // 1. Awaken Sakhra -> Jamil
    state.story_vars.set_unchecked("awakened:sakhra", "true");
    let mut lines = NarrativeLines::default();
    maybe_apply_transformations(&mut state, &pack, "sakhra", &mut lines);

    assert_eq!(
        state.actor_display_name(&pack, "sakhra").as_deref(),
        Some("Jamil")
    );
    assert!(state.is_actor_awakened("sakhra"));

    // 2. Awaken golem-dark-nw into Orin
    state
        .story_vars
        .set_unchecked("awakened:golem-dark-nw", "true");
    let mut lines = NarrativeLines::default();
    let applied = maybe_apply_transformations(&mut state, &pack, "golem-dark-nw", &mut lines);
    assert!(applied);

    let orin_text = lines
        .0
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        orin_text.contains("Orin"),
        "Orin awakening narrated: {orin_text}"
    );
    assert!(
        orin_text.contains("ringing the stone"),
        "Orin fragment narrated: {orin_text}"
    );
    assert_eq!(
        state.actor_display_name(&pack, "golem-dark-nw").as_deref(),
        Some("Orin")
    );
}

#[test]
fn hard_limit_shaman_and_king_cannot_awaken() {
    let pack = load_named_pack("layla", Some("en")).expect("pack loads");
    let mut state = WorldState::new(&pack);
    let mut lines = NarrativeLines::default();

    // Even if story vars are set, shaman and king have no awakening transformations
    state
        .story_vars
        .set_unchecked("awakened:goblin-shaman", "true");
    state
        .story_vars
        .set_unchecked("awakened:elf-king-5", "true");

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
            trigger: cinder_core::content::types::TransformationTrigger::StoryVar {
                key: "test_wake".to_string(),
                value: "true".to_string(),
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
    state.story_vars.set_unchecked("test_wake", "true");
    let mut lines = NarrativeLines::default();
    let transformed = maybe_apply_transformations(&mut state, &pack, "sakhra", &mut lines);
    assert!(transformed);
    assert!(!lines.0.is_empty());
    for line in &lines.0 {
        assert_ne!(line.text.trim(), "\"\"");
        assert!(!line.text.trim().is_empty());
    }
}
