//! Dismissing conjured allies: the fire sprites a spawn sigil calls up can be sent
//! away to free their place, and nobody else can be dismissed.

use cinder_core::content::loader::load_named_pack;
use cinder_core::engine::dialogue::ScriptedDialogueGenerator;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::{ActorStance, WorldState};
use std::sync::Arc;

fn runtime() -> CinderRuntime {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "sanctuary_sanctum".to_string();
    state.clock.turn_number = 1;
    state.story_vars.set_unchecked("knows_spawn", "true");
    state
        .story_vars
        .set_unchecked("courtyard_cages_opened", "true");
    state
        .combat
        .actor_stats
        .entry("player".to_string())
        .or_default()
        .insert("mp".to_string(), 60);
    state.set_actor_stance(&pack, "commander_astrid", ActorStance::Allied, true);
    state.actors.actor_room_overrides.insert(
        "commander_astrid".to_string(),
        "sanctuary_sanctum".to_string(),
    );
    CinderRuntime::with_dialogue_generator(pack, state, Arc::new(ScriptedDialogueGenerator::new()))
        .expect("runtime creates")
}

fn sprites(runtime: &CinderRuntime) -> Vec<String> {
    let mut ids: Vec<String> = runtime
        .export_state()
        .unwrap()
        .actors
        .spawned_actors
        .keys()
        .filter(|id| id.starts_with("summoned-fire-sprite-"))
        .cloned()
        .collect();
    ids.sort();
    ids
}

#[test]
fn a_dismissed_sprite_is_gone_and_its_place_is_free() {
    let runtime = runtime();
    runtime.run_turn("trace spawn sigil").expect("first");
    runtime.run_turn("trace spawn sigil").expect("second");
    let both = sprites(&runtime);
    assert_eq!(both.len(), 2, "{both:?}");

    let full = runtime.run_turn("trace spawn sigil").expect("third");
    assert_eq!(sprites(&runtime).len(), 2, "{}", full.text());

    let out = runtime
        .run_turn(&format!("order {} dismiss", both[0]))
        .expect("dismiss");
    assert!(out.text().contains("gutters out"), "{}", out.text());
    assert_eq!(sprites(&runtime), vec![both[1].clone()]);

    runtime.run_turn("trace spawn sigil").expect("again");
    assert_eq!(sprites(&runtime).len(), 2, "the freed place is usable");
}

#[test]
fn nobody_else_can_be_dismissed() {
    let runtime = runtime();
    let out = runtime
        .run_turn("order commander_astrid dismiss")
        .expect("order");
    assert!(
        out.text().contains("not yours to send away"),
        "{}",
        out.text()
    );
    let state = runtime.export_state().unwrap();
    assert!(state.follows_player("commander_astrid"));
    assert_eq!(state.stance("commander_astrid"), ActorStance::Allied);
}
