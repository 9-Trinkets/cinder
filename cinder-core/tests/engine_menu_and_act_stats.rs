//! Generic engine behavior, checked against a base pack with content injected
//! in the test so the tests do not depend on any pack's story.

use cinder_core::content::loader::load_named_pack;
use cinder_core::content::types::{
    ActCastMember, BeatDefinition, OpeningMenuDefinition, OpeningMenuOptionDefinition,
};
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::WorldState;
use serde_json::json;

#[test]
fn a_menu_attached_to_an_initial_stage_is_open_at_the_start_of_play() {
    let mut pack = load_named_pack("layla", Some("en")).expect("layla loads");
    pack.beats.initial_stage_ids = vec!["test-stage".to_string()];
    pack.beats.stages.push(BeatDefinition {
        id: "test-stage".to_string(),
        open_menu: "test-menu".to_string(),
        ..Default::default()
    });
    pack.menus.push(OpeningMenuDefinition {
        id: "test-menu".to_string(),
        stage_id: "test-stage".to_string(),
        options: vec![OpeningMenuOptionDefinition {
            id: "a".to_string(),
            title: "A".to_string(),
            menu_text: String::new(),
            ..Default::default()
        }],
        ..Default::default()
    });
    let state = WorldState::new(&pack);
    assert_eq!(state.active_menu_id.as_deref(), Some("test-menu"));
}

/// The server rebuilds the runtime from saved state on every turn. Stats that
/// change during an act must survive that, and not snap back to the snapshot
/// taken when the act began.
#[test]
fn stats_earned_during_an_act_survive_a_runtime_reload() {
    let mut pack = load_named_pack("isla", Some("en")).expect("isla loads");
    pack.stats.actor =
        serde_json::from_value(json!({ "points": { "default": 0, "min": 0, "max": 16 } }))
            .expect("stat definitions parse");
    pack.act_cast = vec![ActCastMember {
        id: "gloria".to_string(),
        name: "Gloria".to_string(),
        actor_id: "gloria".to_string(),
        ..Default::default()
    }];
    let state = WorldState::new(&pack);
    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime");

    let mut state = runtime.export_state().expect("state exports");
    state
        .combat
        .actor_stats
        .entry("gloria".to_string())
        .or_default()
        .insert("points".to_string(), 3);

    let reloaded = CinderRuntime::from_state(pack, state, false).expect("runtime reloads");
    let after = reloaded.export_state().expect("state exports");
    assert_eq!(after.actor_stat("gloria", "points"), 3);
}

#[test]
fn rooms_can_start_with_loose_items_lying_in_them() {
    let mut pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let room_id = pack.rooms[0].id.clone();
    pack.rooms[0].items.insert("magic-chalk".to_string(), 2);
    let state = WorldState::new(&pack);
    assert_eq!(
        state.loose_room_items(&room_id),
        vec![("magic-chalk".to_string(), 2)]
    );
}
