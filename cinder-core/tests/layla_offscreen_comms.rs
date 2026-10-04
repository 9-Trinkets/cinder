use cinder_core::engine::narrative::NarrativeLineKind;
use cinder_core::engine::reducer::combat::offscreen_comms::actor_can_use_comms;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::{ActorStance, WorldState};
use cinder_core::loader::load_named_pack;

#[test]
fn astrid_and_einar_start_with_comms_skill() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let state = WorldState::new(&pack);

    assert!(
        state.actor_has_skill("commander_astrid", "comms"),
        "Astrid should possess comms skill from actors.json"
    );
    assert!(
        actor_can_use_comms(&state, "commander_astrid"),
        "Astrid can use comms"
    );

    assert!(
        state.actor_has_skill("einar", "comms"),
        "Einar should possess comms skill from actors.json"
    );
    assert!(actor_can_use_comms(&state, "einar"), "Einar can use comms");
}

#[test]
fn awakening_any_actor_grants_comms() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);

    // Dark golem initially does not have comms
    assert!(
        !state.actor_has_skill("golem-dark-nw", "comms"),
        "Dark golem should not start with comms"
    );
    assert!(
        !actor_can_use_comms(&state, "golem-dark-nw"),
        "Unawakened dark golem cannot use comms"
    );

    // Apply transformation / awakening
    let mut lines = cinder_core::engine::narrative::NarrativeLines::default();
    state
        .adjust_actor_stat(&pack, "golem-dark-nw", "wisdom", 10)
        .unwrap();
    cinder_core::engine::reducer::transformations::maybe_apply_transformations(
        &mut state,
        &pack,
        "golem-dark-nw",
        &mut lines,
    );

    assert!(
        state.is_actor_awakened("golem-dark-nw"),
        "Dark golem should be awakened"
    );
    assert!(
        state.actor_has_skill("golem-dark-nw", "comms"),
        "Awakened dark golem must acquire comms skill"
    );
    assert!(
        actor_can_use_comms(&state, "golem-dark-nw"),
        "Awakened dark golem can use comms"
    );
}

#[test]
fn astrid_reports_contact_when_engaged_offscreen_and_suppresses_raw_narration() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);

    // Layla is in a safe room (r1c1)
    state.current_room_id = "r1c1".to_string();

    // Astrid is stationed at courtyard_south on guard
    state.actor_room_overrides.insert(
        "commander_astrid".to_string(),
        "courtyard_south".to_string(),
    );
    state.set_actor_stance(&pack, "commander_astrid", ActorStance::Allied, false);
    state
        .assign_party_order(&pack, "commander_astrid", "guard".to_string())
        .expect("assign guard order");

    // Position citadel_sentry in courtyard_south ready to strike
    state
        .actor_room_overrides
        .insert("citadel_sentry".to_string(), "courtyard_south".to_string());
    state.set_actor_stance(&pack, "citadel_sentry", ActorStance::Hostile, false);
    state
        .next_hostile_strike_at
        .insert("citadel_sentry".to_string(), 0);

    state.turn_number = 1;

    let dialogue =
        std::sync::Arc::new(cinder_core::engine::dialogue::ScriptedDialogueGenerator::new());
    let runtime =
        CinderRuntime::with_dialogue_generator(pack, state, dialogue).expect("runtime creates");

    let outcome = runtime.run_tick().expect("tick runs");
    let text = outcome.text();

    // 1. Raw combat lines must be suppressed
    assert!(
        !text.contains("strikes Commander Astrid"),
        "Raw strike should be suppressed offscreen: {text}"
    );

    // 2. Dispatch line from Astrid is delivered as Channel line
    let dispatch = outcome
        .lines
        .iter()
        .find(|l| l.kind == NarrativeLineKind::Channel && l.text.starts_with("Commander Astrid:"));
    assert!(
        dispatch.is_some(),
        "Astrid should send a comms dispatch over Channel: {:?}",
        outcome.lines
    );
    let dispatch_line = dispatch.unwrap();
    assert!(
        dispatch_line.text.contains("Holding the line"),
        "Astrid should use her authored contact fallback: {}",
        dispatch_line.text
    );
}

#[test]
fn single_reporter_priority_selects_astrid_over_einar() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);

    // Layla in r1c1
    state.current_room_id = "r1c1".to_string();

    // Both Astrid and Einar are in courtyard_south
    state.actor_room_overrides.insert(
        "commander_astrid".to_string(),
        "courtyard_south".to_string(),
    );
    state.set_actor_stance(&pack, "commander_astrid", ActorStance::Allied, false);
    state
        .assign_party_order(&pack, "commander_astrid", "guard".to_string())
        .expect("assign guard order");

    state
        .actor_room_overrides
        .insert("einar".to_string(), "courtyard_south".to_string());
    state.set_actor_stance(&pack, "einar", ActorStance::Allied, false);
    state
        .assign_party_order(&pack, "einar", "guard".to_string())
        .expect("assign guard order");

    // Enemy in courtyard_south
    state
        .actor_room_overrides
        .insert("citadel_sentry".to_string(), "courtyard_south".to_string());
    state.set_actor_stance(&pack, "citadel_sentry", ActorStance::Hostile, false);
    state
        .next_hostile_strike_at
        .insert("citadel_sentry".to_string(), 0);

    state.turn_number = 1;

    let dialogue =
        std::sync::Arc::new(cinder_core::engine::dialogue::ScriptedDialogueGenerator::new());
    let runtime =
        CinderRuntime::with_dialogue_generator(pack, state, dialogue).expect("runtime creates");

    let outcome = runtime.run_tick().expect("tick runs");

    // Only ONE dispatch from courtyard_south should be emitted, and it should be Astrid!
    let dispatches: Vec<_> = outcome
        .lines
        .iter()
        .filter(|l| {
            l.kind == NarrativeLineKind::Channel
                && (l.text.starts_with("Commander Astrid:") || l.text.starts_with("Einar:"))
        })
        .collect();

    assert_eq!(
        dispatches.len(),
        1,
        "Exactly one party member in the room should report: {:?}",
        dispatches
    );
    assert!(
        dispatches[0].text.starts_with("Commander Astrid:"),
        "Astrid should take priority over Einar as commander: {}",
        dispatches[0].text
    );
}

#[test]
fn area_cleared_milestone_dispatches_when_all_enemies_defeated() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);

    state.current_room_id = "r1c1".to_string();

    state.actor_room_overrides.insert(
        "commander_astrid".to_string(),
        "courtyard_south".to_string(),
    );
    state.set_actor_stance(&pack, "commander_astrid", ActorStance::Allied, false);
    state
        .assign_party_order(&pack, "commander_astrid", "guard".to_string())
        .expect("assign guard order");

    // Mark that combat was active in courtyard_south previously
    state.offscreen_combat_states.insert(
        "courtyard_south".to_string(),
        cinder_core::engine::state::OffscreenCombatState {
            engaged_at_turn: 1,
            last_dispatch_turn: 1,
            contact_dispatched: true,
            low_health_dispatched: false,
            reported_fallen_allies: Default::default(),
            newly_fallen_allies: Default::default(),
        },
    );

    // Make sure no living hostiles exist in courtyard_south
    state
        .actor_room_overrides
        .insert("citadel_sentry".to_string(), "offstage".to_string());
    state
        .actor_room_overrides
        .insert("citadel_guard_captain".to_string(), "offstage".to_string());

    state.turn_number = 2;

    let mut lines = cinder_core::engine::narrative::NarrativeLines::default();
    cinder_core::engine::reducer::combat::evaluate_offscreen_combat_dispatches(
        &mut state, &pack, &mut lines,
    );

    let cleared_dispatch = lines
        .0
        .iter()
        .find(|l| l.kind == NarrativeLineKind::Channel && l.text.contains("is secure"));

    assert!(
        cleared_dispatch.is_some(),
        "Expected area cleared dispatch, found: {:?}",
        lines.0
    );
    assert!(
        cleared_dispatch
            .unwrap()
            .text
            .contains("All hostiles neutralized"),
        "Should announce hostiles neutralized: {}",
        cleared_dispatch.unwrap().text
    );

    // Combat state for the room should be removed
    assert!(
        !state
            .offscreen_combat_states
            .contains_key("courtyard_south"),
        "Offscreen combat state must be cleaned up after area is cleared"
    );
}

#[test]
fn comms_upgrade_pass_uses_dialogue_generator_voice() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);

    state.current_room_id = "r1c1".to_string();

    state.actor_room_overrides.insert(
        "commander_astrid".to_string(),
        "courtyard_south".to_string(),
    );
    state.set_actor_stance(&pack, "commander_astrid", ActorStance::Allied, false);
    state
        .assign_party_order(&pack, "commander_astrid", "guard".to_string())
        .expect("assign guard order");

    state
        .actor_room_overrides
        .insert("citadel_sentry".to_string(), "courtyard_south".to_string());
    state.set_actor_stance(&pack, "citadel_sentry", ActorStance::Hostile, false);
    state
        .next_hostile_strike_at
        .insert("citadel_sentry".to_string(), 0);

    state.turn_number = 1;

    let dialogue = std::sync::Arc::new(
        cinder_core::engine::dialogue::ScriptedDialogueGenerator::new().with_comms_dispatch(
            "commander_astrid",
            "Hold tight, Layla! Commander Astrid here at South Fortress Gate — we have contact!",
        ),
    );
    let runtime =
        CinderRuntime::with_dialogue_generator(pack, state, dialogue).expect("runtime creates");

    let outcome = runtime.run_tick().expect("tick runs");

    let dispatch = outcome
        .lines
        .iter()
        .find(|l| l.kind == NarrativeLineKind::Channel && l.text.starts_with("Commander Astrid:"));
    assert!(
        dispatch.is_some(),
        "Astrid should send a comms dispatch: {:?}",
        outcome.lines
    );
    let line = dispatch.unwrap();
    assert!(
        line.text.contains("Hold tight, Layla!"),
        "DialogueGenerator custom voice should upgrade the fallback text: {}",
        line.text
    );
}
