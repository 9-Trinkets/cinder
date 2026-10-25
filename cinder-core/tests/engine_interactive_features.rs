//! Features with a state and interactions, checked on a pack with a lamp and a
//! door added in the test, so they do not depend on any pack's story.

use cinder_core::content::loader::load_named_pack;
use cinder_core::content::types::{
    AdvanceEffect, Condition, ContentPack, FeatureDescription, Interaction, PanelDataSource,
    RoomFeatureDefinition,
};
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::WorldState;

fn feature_is(feature: &str, state: &str) -> Condition {
    Condition::FeatureIs {
        feature: feature.to_string(),
        state: state.to_string(),
    }
}

fn set_state(state: &str) -> AdvanceEffect {
    AdvanceEffect::SetFeatureState {
        feature: String::new(),
        state: state.to_string(),
    }
}

fn lamp() -> RoomFeatureDefinition {
    RoomFeatureDefinition {
        id: "test-lamp".to_string(),
        label: "test lamp".to_string(),
        aliases: vec!["test lamp".to_string()],
        inspect_text: "A lamp.".to_string(),
        states: vec!["bright".to_string(), "dim".to_string(), "off".to_string()],
        descriptions: vec![
            FeatureDescription {
                when: vec![feature_is("test-lamp", "dim")],
                inspect_text: "The lamp glows low and amber.".to_string(),
            },
            FeatureDescription {
                when: vec![feature_is("test-lamp", "off")],
                inspect_text: "The lamp is dark.".to_string(),
            },
        ],
        interactions: vec![
            Interaction {
                id: "dim".to_string(),
                label: "Dim the lamp".to_string(),
                when: vec![feature_is("test-lamp", "bright")],
                lines: vec!["You turn the dial down.".to_string()],
                effects: vec![set_state("dim")],
                repeatable: true,
            },
            Interaction {
                id: "brighten".to_string(),
                label: "Brighten the lamp".to_string(),
                when: vec![feature_is("test-lamp", "dim")],
                lines: vec!["You turn the dial up.".to_string()],
                effects: vec![set_state("bright")],
                repeatable: true,
            },
            Interaction {
                id: "switch-off".to_string(),
                label: "Switch the lamp off".to_string(),
                when: Vec::new(),
                lines: vec!["Click.".to_string()],
                effects: vec![set_state("off")],
                repeatable: false,
            },
        ],
        ..Default::default()
    }
}

fn pack() -> ContentPack {
    let mut pack = load_named_pack("isla", Some("en")).expect("isla loads");
    pack.rooms
        .iter_mut()
        .find(|room| room.id == "playroom")
        .expect("playroom")
        .features
        .push(lamp());
    pack
}

fn runtime_in_the_playroom() -> CinderRuntime {
    let pack = pack();
    let mut state = WorldState::new(&pack);
    state.current_room_id = "playroom".to_string();
    CinderRuntime::from_state(pack, state, false).expect("runtime")
}

fn do_titles(runtime: &CinderRuntime) -> Vec<String> {
    runtime
        .panel_options(&PanelDataSource::FeatureInteractions)
        .expect("options")
        .into_iter()
        .map(|option| option.title)
        .collect()
}

fn lamp_state(runtime: &CinderRuntime) -> Option<String> {
    let pack = pack();
    runtime
        .export_state()
        .expect("state")
        .feature_state(&pack, "playroom", "test-lamp")
}

#[test]
fn a_feature_starts_in_its_first_state_and_offers_what_fits_it() {
    let runtime = runtime_in_the_playroom();
    assert_eq!(lamp_state(&runtime).as_deref(), Some("bright"));
    assert_eq!(
        do_titles(&runtime),
        vec!["Dim the lamp", "Switch the lamp off"]
    );
}

#[test]
fn choosing_an_interaction_plays_its_lines_and_changes_the_state() {
    let runtime = runtime_in_the_playroom();
    let text = runtime.run_turn("do test-lamp:dim").expect("do").text();
    assert!(text.contains("You turn the dial down."), "{text}");
    assert_eq!(lamp_state(&runtime).as_deref(), Some("dim"));
    assert_eq!(
        do_titles(&runtime),
        vec!["Brighten the lamp", "Switch the lamp off"],
        "the offers follow the state"
    );
}

#[test]
fn a_feature_reads_differently_in_each_state() {
    let runtime = runtime_in_the_playroom();
    let bright = runtime.run_turn("look at test lamp").expect("look").text();
    assert!(bright.contains("A lamp."), "{bright}");
    runtime.run_turn("do test-lamp:dim").expect("dim");
    let dim = runtime.run_turn("look at test lamp").expect("look").text();
    assert!(dim.contains("glows low and amber"), "{dim}");
}

#[test]
fn an_interaction_can_be_typed_by_its_label() {
    let runtime = runtime_in_the_playroom();
    let text = runtime
        .run_turn("do Dim the lamp")
        .expect("do by label")
        .text();
    assert!(text.contains("You turn the dial down."), "{text}");
}

#[test]
fn a_one_time_interaction_is_not_offered_again() {
    let runtime = runtime_in_the_playroom();
    runtime.run_turn("do test-lamp:switch-off").expect("off");
    assert_eq!(lamp_state(&runtime).as_deref(), Some("off"));
    assert!(
        !do_titles(&runtime).contains(&"Switch the lamp off".to_string()),
        "{:?}",
        do_titles(&runtime)
    );
}

#[test]
fn an_interaction_that_is_not_on_offer_cannot_be_used() {
    let runtime = runtime_in_the_playroom();
    let text = runtime
        .run_turn("do test-lamp:brighten")
        .expect("do")
        .text();
    assert!(!text.contains("You turn the dial up."), "{text}");
    assert_eq!(lamp_state(&runtime).as_deref(), Some("bright"));
}

#[test]
fn a_feature_state_gates_an_exit() {
    let mut pack = pack();
    let playroom = pack
        .rooms
        .iter_mut()
        .find(|room| room.id == "playroom")
        .expect("playroom");
    playroom.exits[0].when = vec![feature_is("test-lamp", "dim")];
    let mut state = WorldState::new(&pack);
    state.current_room_id = "playroom".to_string();
    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime");
    let blocked = runtime.run_turn("go hall").expect("go").text();
    assert!(!blocked.contains("Ground-Floor Hall"), "{blocked}");
    runtime.run_turn("do test-lamp:dim").expect("dim");
    let open = runtime.run_turn("go hall").expect("go").text();
    assert!(open.contains("Ground-Floor Hall"), "{open}");
}

#[test]
fn feature_state_is_saved_with_the_game() {
    let runtime = runtime_in_the_playroom();
    runtime.run_turn("do test-lamp:dim").expect("dim");
    let state = runtime.export_state().expect("state");
    let json = serde_json::to_string(&state).expect("serializes");
    let loaded: WorldState = serde_json::from_str(&json).expect("loads");
    assert_eq!(
        loaded
            .feature_state(&pack(), "playroom", "test-lamp")
            .as_deref(),
        Some("dim")
    );
}

#[test]
fn an_older_save_has_every_feature_in_its_starting_state() {
    let pack = pack();
    let state = WorldState::new(&pack);
    let mut json: serde_json::Value = serde_json::to_value(&state).expect("serializes");
    json.as_object_mut()
        .expect("object")
        .remove("feature_states");
    let loaded: WorldState = serde_json::from_value(json).expect("an old save still loads");
    assert_eq!(
        loaded
            .feature_state(&pack, "playroom", "test-lamp")
            .as_deref(),
        Some("bright")
    );
}

#[test]
fn the_do_action_is_hidden_where_nothing_can_be_done() {
    let pack = pack();
    let state = WorldState::new(&pack);
    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime");
    assert!(
        runtime
            .panel_options(&PanelDataSource::FeatureInteractions)
            .expect("options")
            .is_empty(),
        "the porch has no operable features"
    );
}
