//! The playroom and hall are instruments: their light, blind and white-noise
//! machine have states that interactions change. They are offered only in an
//! episode that turns them on (`episode_tam`), so Wren's session is unchanged.

use cinder_core::content::loader::load_named_pack;
use cinder_core::content::types::PanelDataSource;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::WorldState;

fn runtime_in(room: &str, episode: bool) -> CinderRuntime {
    let pack = load_named_pack("isla", Some("en")).expect("isla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = room.to_string();
    if episode {
        state.story_vars.set_unchecked("episode_tam", "true");
    }
    CinderRuntime::from_state(pack, state, false).expect("runtime")
}

fn titles(runtime: &CinderRuntime) -> Vec<String> {
    runtime
        .panel_options(&PanelDataSource::FeatureInteractions)
        .expect("options")
        .into_iter()
        .map(|option| option.title)
        .collect()
}

fn state_of(runtime: &CinderRuntime, room: &str, feature: &str) -> String {
    let pack = load_named_pack("isla", Some("en")).expect("isla loads");
    runtime
        .export_state()
        .expect("state")
        .feature_state(&pack, room, feature)
        .expect("a state")
}

#[test]
fn nothing_can_be_operated_outside_the_episode_that_needs_it() {
    assert!(titles(&runtime_in("playroom", false)).is_empty());
    assert!(titles(&runtime_in("hall", false)).is_empty());
}

#[test]
fn the_playroom_offers_light_and_blind_in_their_starting_states() {
    let runtime = runtime_in("playroom", true);
    let offered = titles(&runtime);
    for expected in [
        "Turn the lamp on",
        "Turn the overhead light off",
        "Lower the blind",
    ] {
        assert!(offered.contains(&expected.to_string()), "{offered:?}");
    }
    assert!(!offered.contains(&"Turn the lamp down".to_string()));
}

#[test]
fn the_lamp_goes_through_its_states() {
    let runtime = runtime_in("playroom", true);
    runtime.run_turn("do lamp:on").expect("on");
    assert_eq!(state_of(&runtime, "playroom", "lamp"), "bright");
    runtime.run_turn("do lamp:down").expect("down");
    assert_eq!(state_of(&runtime, "playroom", "lamp"), "dim");
    let text = runtime.run_turn("look at lamp").expect("look").text();
    assert!(text.contains("turned low"), "{text}");
    runtime.run_turn("do Turn the lamp off").expect("off");
    assert_eq!(state_of(&runtime, "playroom", "lamp"), "off");
}

#[test]
fn the_overhead_light_and_blind_change_what_the_room_reads_like() {
    let runtime = runtime_in("playroom", true);
    runtime.run_turn("do overhead-light:off").expect("off");
    runtime.run_turn("do blind:lower").expect("lower");
    assert_eq!(state_of(&runtime, "playroom", "overhead-light"), "off");
    assert_eq!(state_of(&runtime, "playroom", "blind"), "down");
    let light = runtime.run_turn("look at light").expect("look").text();
    assert!(light.contains("dark"), "{light}");
    let blind = runtime.run_turn("look at blind").expect("look").text();
    assert!(blind.contains("The blind is down"), "{blind}");
}

#[test]
fn the_white_noise_machine_can_only_be_turned_down_from_the_hall() {
    let runtime = runtime_in("playroom", true);
    assert!(
        !titles(&runtime)
            .iter()
            .any(|title| title.contains("machine")),
        "the machine is in the hall"
    );
    let hall = runtime_in("hall", true);
    assert!(titles(&hall).contains(&"Turn the machine down".to_string()));
    hall.run_turn("do white-noise:down").expect("down");
    assert_eq!(state_of(&hall, "hall", "white-noise"), "low");
    hall.run_turn("do white-noise:off").expect("off");
    assert_eq!(state_of(&hall, "hall", "white-noise"), "off");
    let text = hall.run_turn("look at machine").expect("look").text();
    assert!(text.contains("is off"), "{text}");
}
