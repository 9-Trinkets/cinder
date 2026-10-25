//! Ending an act keeps the world. Only variables declared with act scope are
//! cleared, so what the player carries, where they stand and what they have
//! changed all carry into the next act.

use cinder_core::content::loader::load_named_pack;
use cinder_core::content::types::{
    ActorRelocationDefinition, AdvanceEffect, AdvanceSignal, BeatDefinition, Interaction,
    RoomFeatureDefinition,
};
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::{GamePhase, VariableDeclaration, WorldState};
use serde_json::json;

fn runtime() -> CinderRuntime {
    let mut pack = load_named_pack("isla", Some("en")).expect("isla loads");
    pack.variables.insert(
        "this_act".to_string(),
        serde_json::from_value::<VariableDeclaration>(json!({ "scope": "act" }))
            .expect("declaration"),
    );
    pack.variables.insert(
        "whole_game".to_string(),
        serde_json::from_value::<VariableDeclaration>(json!({ "scope": "game" }))
            .expect("declaration"),
    );
    pack.rooms
        .iter_mut()
        .find(|room| room.id == "playroom")
        .expect("playroom")
        .features
        .push(RoomFeatureDefinition {
            id: "test-lamp".to_string(),
            label: "test lamp".to_string(),
            aliases: vec!["test lamp".to_string()],
            inspect_text: "A lamp.".to_string(),
            states: vec!["bright".to_string(), "off".to_string()],
            interactions: vec![Interaction {
                id: "finish".to_string(),
                label: "Finish the act".to_string(),
                effects: vec![
                    AdvanceEffect::SetFeatureState {
                        feature: String::new(),
                        state: "off".to_string(),
                    },
                    AdvanceEffect::EmitSignal {
                        signal: "finish-act".to_string(),
                    },
                ],
                ..Default::default()
            }],
            ..Default::default()
        });
    let player_id = pack.settings.combat.player_actor_id.clone();
    pack.beats.initial_stage_ids = vec!["first".to_string()];
    pack.beats.stages = vec![
        BeatDefinition {
            id: "first".to_string(),
            advance_signals: vec![AdvanceSignal::Simple("finish-act".to_string())],
            next_stage_ids: vec!["last".to_string()],
            ..Default::default()
        },
        BeatDefinition {
            id: "last".to_string(),
            end_act: true,
            actor_relocations: vec![ActorRelocationDefinition {
                actor_id: player_id,
                to_room_id: "staff-room".to_string(),
            }],
            ..Default::default()
        },
    ];
    pack.menus.clear();
    let mut state = WorldState::new(&pack);
    state.current_room_id = "playroom".to_string();
    state.add_item("test-keepsake");
    state.story_vars.set_unchecked("this_act", "yes");
    state.story_vars.set_unchecked("whole_game", "yes");
    CinderRuntime::from_state(pack, state, false).expect("runtime")
}

#[test]
fn the_world_carries_across_an_act_and_only_act_variables_clear() {
    let runtime = runtime();
    let outcome = runtime.run_turn("do test-lamp:finish").expect("turn");
    assert_eq!(outcome.phase, GamePhase::ActEnded, "{}", outcome.text());

    runtime.continue_after_act().expect("continues");
    let state = runtime.export_state().expect("state");
    assert_eq!(state.phase, GamePhase::Active);
    assert_eq!(
        state.current_room_id, "staff-room",
        "a stage can put the player in another room"
    );
    assert!(state.has_item("test-keepsake"));
    assert_eq!(state.story_vars.get("whole_game"), Some("yes"));
    assert_eq!(state.story_vars.get("this_act"), None);
}
