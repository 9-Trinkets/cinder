//! Integration tests for the Handler's reward beats.
//!
//! Each beat is a one-shot interjection the Handler grudgingly attaches to a
//! thing Layla did well, framed as a console upgrade he hands over rather than
//! praise. Kills read as competence, not virtue.

use cinder_core::content::loader::load_named_pack;
use cinder_core::content::types::ContentPack;
use cinder_core::engine::narrative::{NarrativeLine, NarrativeLineKind};
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::{ActorStance, WorldState};

fn handler_spoke(lines: &[NarrativeLine], needle: &str) -> bool {
    lines
        .iter()
        .any(|line| line.kind == NarrativeLineKind::Channel && line.text.contains(needle))
}

fn section_shown(pack: &ContentPack, state: &WorldState, id: &str) -> bool {
    pack.visible_sidebar_sections(state)
        .iter()
        .any(|section| section == id)
}

fn strong_player(state: &mut WorldState) {
    let stats = state
        .combat
        .actor_stats
        .entry("player".to_string())
        .or_default();
    stats.insert("strength".to_string(), 100);
    // `surround_rule` is intelligence-based resistance.
    stats.insert("intelligence".to_string(), 100);
}

#[test]
fn first_mob_kill_grants_vitals_readout_and_handler_line() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "r2c2".to_string(); // goblin-1
    strong_player(&mut state);

    // Vitals stay hidden until Layla proves she can handle herself.
    assert!(
        !section_shown(&pack, &state, "vitals"),
        "vitals must stay hidden until the first kill"
    );

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");
    let outcome = runtime
        .run_turn("attack goblin")
        .expect("attack goblin succeeds");

    let after = runtime.export_state().expect("state exported");
    assert_eq!(after.story_vars.get("first_mob_defeated"), Some("true"));
    assert!(
        section_shown(&pack, &after, "vitals"),
        "the first kill must reveal the vitals readout"
    );
    assert!(
        handler_spoke(&outcome.lines, "First one down"),
        "Handler must comment on the first kill"
    );
}

#[test]
fn first_mob_kill_beat_is_one_shot() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "r2c2".to_string();
    strong_player(&mut state);

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");
    let first = runtime
        .run_turn("attack goblin")
        .expect("attack goblin succeeds");
    assert!(handler_spoke(&first.lines, "First one down"));

    // Second kill, same room, must not repeat the beat.
    let second = runtime
        .run_turn("attack goblin")
        .expect("second attack succeeds");
    assert!(
        !handler_spoke(&second.lines, "First one down"),
        "the first-kill beat must not repeat"
    );
}

#[test]
fn clean_shaman_run_earns_handler_acknowledgement() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "r5c5".to_string();
    strong_player(&mut state);

    // No floor-one mob was attacked, so the clean-run condition holds.
    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");
    let outcome = runtime
        .run_turn("attack goblin shaman")
        .expect("attack shaman succeeds");

    let after = runtime.export_state().expect("state exported");
    assert!(after.has_item("shaman-ring"), "clean run grants the ring");
    assert!(
        handler_spoke(&outcome.lines, "Clean"),
        "Handler must acknowledge the clean shaman run"
    );
}

#[test]
fn messy_shaman_run_earns_no_acknowledgement() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "r5c5".to_string();
    strong_player(&mut state);
    state
        .story_vars
        .set_unchecked("layla_attacked_floor_one_mob", "true");

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");
    let outcome = runtime
        .run_turn("attack goblin shaman")
        .expect("attack shaman succeeds");

    let after = runtime.export_state().expect("state exported");
    assert!(!after.has_item("shaman-ring"));
    assert!(
        !handler_spoke(&outcome.lines, "Clean"),
        "a messy run must not earn the Handler's approval"
    );
}

/// Ring the room containing `target` so the final sigil placement converts it.
/// Every neighbour of `room_id` needs a charm sigil; placing the trigger item
/// in `room_id` itself is what closes the circle and fires the hook.
fn ring_room(
    pack: &cinder_core::content::types::ContentPack,
    state: &mut WorldState,
    room_id: &str,
) -> Vec<NarrativeLine> {
    use cinder_core::content::types::ItemStorageTarget as Storage;
    use cinder_core::engine::events::{TimestampedWorldEvent, WorldEvent};
    use cinder_core::engine::reducer::apply_events;

    let player_id = pack.settings.combat.player_actor_id.clone();
    state.combat.actor_level.insert(player_id, 12);

    for neighbour in pack.adjacent_room_ids(room_id) {
        state.add_item_to_storage("charm-sigil", Storage::CurrentRoom, &neighbour);
    }

    // Placing the final sigil in the target's own room closes the circle.
    let output = apply_events(
        state,
        pack,
        &[TimestampedWorldEvent::now(WorldEvent::ItemAcquired {
            item_id: "charm-sigil".to_string(),
            storage: Storage::CurrentRoom,
        })],
    );
    let _ = room_id;
    output.lines.0
}

#[test]
fn first_mob_charm_earns_handler_line() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "r2c2".to_string(); // goblin-1
    strong_player(&mut state);

    let lines = ring_room(&pack, &mut state, "r2c2");

    assert_eq!(
        state.relationship("goblin-1").stance,
        ActorStance::Allied,
        "the ring must convert goblin-1"
    );
    assert_eq!(state.story_vars.get("first_mob_charmed"), Some("true"));
    assert!(
        handler_spoke(&lines, "closed a ring around it"),
        "Handler must comment on the first charm"
    );
}

#[test]
fn charm_beat_ignores_the_shaman() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "r5c5".to_string();
    strong_player(&mut state);

    let lines = ring_room(&pack, &mut state, "r5c5");

    assert_eq!(
        state.story_vars.get("first_mob_charmed"),
        None,
        "the shaman is a boss and must not spend the first-charm beat"
    );
    assert!(
        !handler_spoke(&lines, "closed a ring around it"),
        "the shaman must not spend the first-charm beat"
    );
}

#[test]
fn sensory_enhancer_narrates_handler_upgrade() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "r5c5".to_string();
    state
        .inventory
        .player_inventory
        .insert("sensory-enhancer".to_string(), 1);

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");
    let outcome = runtime
        .run_turn("use sensory-enhancer")
        .expect("using the enhancer succeeds");

    let after = runtime.export_state().expect("state exported");
    assert_eq!(after.story_vars.get("has_sensory_enhancer"), Some("true"));
    assert!(
        handler_spoke(&outcome.lines, "perception capsule"),
        "Handler must frame the enhancer as a console upgrade"
    );
}

#[test]
fn killing_a_golem_first_also_unlocks_vitals() {
    // Regression: the four floor-one golems sit beside the goblins, and the
    // original rules only enumerated goblin-1..4. Killing a golem first left
    // `first_mob_defeated` unset, so vitals stayed locked for the whole run.
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "r3c3".to_string(); // golem-dark-nw
    strong_player(&mut state);

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");
    let outcome = runtime
        .run_turn("attack golem")
        .expect("attack golem succeeds");

    let after = runtime.export_state().expect("state exported");
    assert_eq!(
        after.story_vars.get("first_mob_defeated"),
        Some("true"),
        "a golem kill must count as the first mob kill"
    );
    assert!(
        section_shown(&pack, &after, "vitals"),
        "killing a golem first must still reveal vitals"
    );
    assert!(
        handler_spoke(&outcome.lines, "First one down"),
        "Handler must comment on a golem kill too"
    );
}

#[test]
fn killing_the_shaman_first_still_unlocks_vitals() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "r5c5".to_string();
    strong_player(&mut state);

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");
    runtime
        .run_turn("attack goblin shaman")
        .expect("attack shaman succeeds");

    let after = runtime.export_state().expect("state exported");
    assert_eq!(after.story_vars.get("first_mob_defeated"), Some("true"));
    assert!(section_shown(&pack, &after, "vitals"));
}

#[test]
fn handler_reward_lines_are_defined() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    for key in [
        "handler.first_skill_used",
        "handler.first_mob_killed",
        "handler.first_mob_charmed",
        "handler.clean_shaman_reward",
        "handler.sensory_enhancer_unlocked",
    ] {
        assert!(pack.messages.contains_key(key), "{key} must exist");
    }
}

#[test]
fn first_skill_use_grants_skills_readout_and_handler_line() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "r2c2".to_string(); // goblin-1
    // Give goblin enough health to survive so vitals are not unlocked yet
    state
        .combat
        .actor_stats
        .entry("goblin-1".to_string())
        .or_default()
        .insert("hp".to_string(), 100);

    assert!(
        !section_shown(&pack, &state, "skills"),
        "skills readout must stay hidden before first skill use"
    );
    assert!(
        !section_shown(&pack, &state, "vitals"),
        "vitals must stay hidden before first mob kill"
    );

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");
    let outcome = runtime
        .run_turn("attack goblin")
        .expect("attack goblin succeeds");

    let after = runtime.export_state().expect("state exported");
    assert_eq!(after.story_vars.get("first_skill_used"), Some("true"));
    assert!(
        section_shown(&pack, &after, "skills"),
        "first skill use must reveal the skills readout"
    );
    assert!(
        !section_shown(&pack, &after, "vitals"),
        "vitals must remain hidden if the mob was not killed"
    );
    assert!(
        handler_spoke(&outcome.lines, "Technique, chalk, blade"),
        "Handler must comment on the first skill use"
    );
}

#[test]
fn first_skill_use_beat_is_one_shot() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "r2c2".to_string();
    state
        .combat
        .actor_stats
        .entry("goblin-1".to_string())
        .or_default()
        .insert("hp".to_string(), 100);

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");
    let first = runtime
        .run_turn("attack goblin")
        .expect("attack goblin succeeds");
    assert!(handler_spoke(&first.lines, "Technique, chalk, blade"));

    let second = runtime
        .run_turn("attack goblin")
        .expect("second attack succeeds");
    assert!(
        !handler_spoke(&second.lines, "Technique, chalk, blade"),
        "the first-skill-use beat must not repeat"
    );
}

#[test]
fn tracing_sigil_first_reveals_skills_readout_before_vitals() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let state = WorldState::new(&pack);

    assert!(
        !section_shown(&pack, &state, "skills"),
        "skills readout must stay hidden before first skill use"
    );
    assert!(
        !section_shown(&pack, &state, "vitals"),
        "vitals must stay hidden before first kill"
    );

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");
    let outcome = runtime
        .run_turn("trace charm")
        .expect("trace charm succeeds");

    let after = runtime.export_state().expect("state exported");
    assert_eq!(after.story_vars.get("first_skill_used"), Some("true"));
    assert!(
        section_shown(&pack, &after, "skills"),
        "tracing a sigil must reveal the skills readout"
    );
    assert!(
        !section_shown(&pack, &after, "vitals"),
        "vitals must remain hidden when tracing a sigil without killing a mob"
    );
    assert!(
        handler_spoke(&outcome.lines, "Technique, chalk, blade"),
        "Handler must comment on the first skill use when tracing"
    );
}

#[test]
fn handler_boss_and_sigil_lines_are_defined() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    for key in [
        "handler.descend.frost_citadel",
        "handler.drain_tutorial",
        "handler.spawn_tutorial",
        "handler.floor2_court_cleared",
        "handler.floor3_elemental_resolved",
        "handler.quest.citadel_siege_complete",
    ] {
        let msg = pack
            .messages
            .get(key)
            .unwrap_or_else(|| panic!("{key} must exist"));
        assert_eq!(
            msg.voice(),
            cinder_core::content::types::PackMessageVoice::Handler,
            "{key} must have voice: handler"
        );
    }
}

#[test]
fn scroll_reading_triggers_handler_sigil_tutorials() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");

    // 1. Drain scroll
    let mut s1 = WorldState::new(&pack);
    s1.acquire_player_item(&pack, "drain-scroll");
    let rt1 = CinderRuntime::from_state(pack.clone(), s1, false).expect("runtime");
    let out1 = rt1.run_turn("read scroll").expect("read drain scroll");
    assert!(
        handler_spoke(&out1.lines, "Drain sigil"),
        "Handler must comment on learning Drain Sigil: {:#?}",
        out1.lines
    );

    // 2. Spawn scroll
    let mut s2 = WorldState::new(&pack);
    s2.acquire_player_item(&pack, "spawn-scroll");
    let rt2 = CinderRuntime::from_state(pack.clone(), s2, false).expect("runtime");
    let out2 = rt2.run_turn("use ember scroll").expect("read spawn scroll");
    assert!(
        handler_spoke(&out2.lines, "Spawn sigil"),
        "Handler must comment on learning Spawn Sigil: {:#?}",
        out2.lines
    );
}

#[test]
fn floor5_courtyard_first_visit_triggers_handler_descend() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "teleport_platform".to_string();
    state.story_vars.set_unchecked("knows_teleport", "true");
    state
        .story_vars
        .set_unchecked("anchor_floor4_platform", "true");
    state.story_vars.set_unchecked("anchor_floor5_gate", "true");

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime");
    let out = runtime
        .run_turn("teleport floor5_start")
        .expect("teleport to floor 5");

    assert_eq!(runtime.current_room_id().unwrap(), "courtyard_center");
    assert!(
        handler_spoke(&out.lines, "Frost Citadel") || handler_spoke(&out.lines, "Floor five."),
        "Handler must comment on first visit to Floor 5 courtyard: {:#?}",
        out.lines
    );
}

#[test]
fn floor2_elf_king_defeat_triggers_court_cleared_commentary() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "d8c5".to_string(); // elf-king room
    strong_player(&mut state);

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime");
    let out = runtime
        .run_turn("attack elf king")
        .expect("attack elf king succeeds");

    assert!(
        handler_spoke(&out.lines, "The King yields his square"),
        "Handler must comment on Elf King defeat: {:#?}",
        out.lines
    );
}

#[test]
fn floor3_fire_elemental_resolution_triggers_commentary() {
    use cinder_core::content::types::ItemStorageTarget as Storage;
    use cinder_core::engine::events::{TimestampedWorldEvent, WorldEvent};
    use cinder_core::engine::reducer::apply_events;

    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);

    state.current_room_id = "oas".to_string();
    let player_id = pack.settings.combat.player_actor_id.clone();
    state.combat.actor_level.insert(player_id.clone(), 12);

    for room_id in ["o6", "o7", "oh"] {
        state.add_item_to_storage("charm-sigil", Storage::CurrentRoom, room_id);
    }

    let out = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::ItemAcquired {
            item_id: "charm-sigil".to_string(),
            storage: Storage::CurrentRoom,
        })],
    );

    assert!(
        handler_spoke(&out.lines.0, "Core temperature dropping to baseline"),
        "Handler must comment on Fire Elemental charm: {:#?}",
        out.lines.0
    );
}
