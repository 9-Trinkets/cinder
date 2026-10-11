//! Integration tests for Floor 5 (The Frost Citadel: 7-Room Hexagonal Courtyard).

use cinder_core::content::loader::load_named_pack;
use cinder_core::content::types::ItemStorageTarget;
use cinder_core::engine::events::{TimestampedWorldEvent, WorldEvent};
use cinder_core::engine::reducer::apply_events;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::{ActorStance, SpawnActorConfig, SpawnActorOutcome, WorldState};

const EXPECTED_COURTYARD_ROOMS: &[&str] = &[
    "courtyard_center",
    "courtyard_north",
    "courtyard_northeast",
    "courtyard_southeast",
    "courtyard_south",
    "courtyard_southwest",
    "courtyard_northwest",
];

fn spawned_count(state: &WorldState, template_id: &str) -> usize {
    let prefix = format!("{template_id}-");
    state
        .spawned_actors
        .keys()
        .filter(|actor_id| actor_id.starts_with(&prefix))
        .count()
}

fn narrative_text(lines: &cinder_core::engine::narrative::NarrativeLines) -> String {
    lines
        .0
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn floor5_courtyard_rooms_load_and_validate() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");

    assert_eq!(EXPECTED_COURTYARD_ROOMS.len(), 7);
    for room_id in EXPECTED_COURTYARD_ROOMS {
        let room = pack
            .room(room_id)
            .unwrap_or_else(|| panic!("missing room {room_id}"));
        assert!(!room.title.trim().is_empty(), "room {room_id} title empty");
        assert!(
            !room.summary.trim().is_empty(),
            "room {room_id} summary empty"
        );
        assert!(
            !room.inspect_text.trim().is_empty(),
            "room {room_id} inspect_text empty"
        );
        assert!(
            !room.features.is_empty(),
            "room {room_id} should have features"
        );
        for feature in &room.features {
            assert!(
                !feature.inspect_text.trim().is_empty(),
                "feature {} in room {room_id} has empty inspect text",
                feature.id
            );
        }
    }
}

#[test]
fn floor5_defines_five_named_civilian_offerings() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let civilians = [
        ("nivi_olsen", "salt_reach"),
        ("eliska_novakova", "glassbank"),
        ("amaru_quispe", "woolcross"),
        ("abeni_adeyemi", "greenrest"),
        ("chen_yu_xin", "brass_yard"),
    ];

    for (actor_id, town_tag) in civilians {
        let actor = pack
            .actor(actor_id)
            .unwrap_or_else(|| panic!("missing civilian {actor_id}"));
        assert_eq!(actor.room_id, "courtyard_center");
        assert!(!actor.attackable);
        assert!(actor.tags.iter().any(|tag| tag == "civilian"));
        assert!(actor.tags.iter().any(|tag| tag == town_tag));
    }

    assert!(
        pack.actor("caged_offerings").is_none(),
        "the aggregate offering actor must be replaced by named civilians"
    );
}

#[test]
fn floor5_house_heads_have_role_appropriate_endurance() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let expected = [("lord_vane", 6), ("warmaster_torin", 8), ("lady_sylvan", 4)];

    for (actor_id, endurance) in expected {
        let actor = pack
            .actor(actor_id)
            .unwrap_or_else(|| panic!("missing house head {actor_id}"));
        assert_eq!(
            actor.initial_stats.get("endurance").copied(),
            Some(endurance),
            "{actor_id} endurance should match its combat role"
        );
    }
}

#[test]
fn floor5_courtyard_siege_soldiers_attack_civilians_and_charmed_civilians_can_evacuate() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "courtyard_center".to_string();
    state.active_objective_stage_ids = vec!["sq_five_offerings".to_string()];
    state
        .stage_started_minutes
        .insert("sq_five_offerings".to_string(), state.current_time_minutes);
    let raider_id = match state.spawn_actor(
        &pack,
        SpawnActorConfig {
            template_id: "frost_wolf_raider",
            room_id: Some("courtyard_center"),
            stance: Some(ActorStance::Hostile),
            follows_player: false,
            scale_with_actor_id: None,
            scale_stat: None,
            max_active_instances: None,
        },
    ) {
        SpawnActorOutcome::Success(actor) => actor.instance_id,
        other => panic!("raider should spawn: {other:?}"),
    };

    // First strike: raider attacks Nivi Olsen in courtyard_center
    let strike1 = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::HostileStrike {
            actor_id: raider_id.clone(),
        })],
    );
    let strike1_text = narrative_text(&strike1.lines);
    assert!(
        strike1_text.contains("Nivi Olsen is down"),
        "Handler must announce Nivi's fall: {strike1_text}"
    );
    assert!(state.actor_is_defeated("nivi_olsen", &pack.settings.combat.health_stat_id));
    assert_eq!(state.story_vars.get("five_offerings_deaths"), Some("1"));

    // Player charms Eliška Nováková and evacuates her to courtyard_north
    state.set_stance("eliska_novakova", ActorStance::Allied);
    state.set_follows_player("eliska_novakova", true);
    state
        .actor_room_overrides
        .insert("eliska_novakova".to_string(), "courtyard_north".to_string());

    // Second strike in courtyard_center: raider cannot target evacuated Eliška, strikes Amaru Quispe instead
    state.current_time_minutes += 5;
    let strike2 = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::HostileStrike {
            actor_id: raider_id.clone(),
        })],
    );
    let strike2_text = narrative_text(&strike2.lines);
    assert!(
        strike2_text.contains("Amaru Quispe is down"),
        "Handler must announce Amaru's fall: {strike2_text}"
    );
    assert!(state.actor_is_defeated("amaru_quispe", &pack.settings.combat.health_stat_id));
    assert!(
        !state.actor_is_defeated("eliska_novakova", &pack.settings.combat.health_stat_id),
        "Evacuated Eliška must remain safe and unattacked in courtyard_north"
    );
    assert_eq!(state.story_vars.get("five_offerings_deaths"), Some("2"));

    // Station an allied guard in courtyard_center to protect remaining civilians
    state.set_stance("commander_astrid", ActorStance::Allied);
    state
        .party_orders
        .insert("commander_astrid".to_string(), "guard".to_string());
    state.actor_room_overrides.insert(
        "commander_astrid".to_string(),
        "courtyard_center".to_string(),
    );
    let astrid_initial_hp = state.actor_stat("commander_astrid", "hp");

    state.current_time_minutes += 5;
    let strike3 = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::HostileStrike {
            actor_id: raider_id,
        })],
    );
    let strike3_text = narrative_text(&strike3.lines);
    assert!(
        strike3_text.contains("Astrid"),
        "Guard must intercept strike: {strike3_text}"
    );
    assert!(
        state.actor_stat("commander_astrid", "hp") < astrid_initial_hp,
        "Astrid should absorb the damage"
    );
    assert!(
        !state.actor_is_defeated("abeni_adeyemi", &pack.settings.combat.health_stat_id),
        "Civilians remain safe while guard holds"
    );
    assert_eq!(state.story_vars.get("five_offerings_deaths"), Some("2"));
}

#[test]
fn floor5_protection_quest_fails_on_third_death_without_ending_game() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "courtyard_center".to_string();
    state.active_objective_stage_ids = vec!["sq_five_offerings".to_string()];
    state
        .stage_started_minutes
        .insert("sq_five_offerings".to_string(), state.current_time_minutes);
    let raider_id = match state.spawn_actor(
        &pack,
        SpawnActorConfig {
            template_id: "frost_wolf_raider",
            room_id: Some("courtyard_center"),
            stance: Some(ActorStance::Hostile),
            follows_player: false,
            scale_with_actor_id: None,
            scale_stat: None,
            max_active_instances: None,
        },
    ) {
        SpawnActorOutcome::Success(actor) => actor.instance_id,
        other => panic!("raider should spawn: {other:?}"),
    };

    // Raider strikes 3 times in courtyard_center, defeating 3 civilians sequentially
    for expected_deaths in 1..=3 {
        state.current_time_minutes += 5;
        apply_events(
            &mut state,
            &pack,
            &[TimestampedWorldEvent::now(WorldEvent::HostileStrike {
                actor_id: raider_id.clone(),
            })],
        );
        let expected_deaths = expected_deaths.to_string();
        assert_eq!(
            state.story_vars.get("five_offerings_deaths"),
            Some(expected_deaths.as_str())
        );
    }

    assert_eq!(state.story_vars.get("five_offerings_failed"), Some("true"));
    assert!(
        state
            .active_objective_stage_ids
            .contains(&"sq_five_offerings_failed".to_string())
    );
    assert_eq!(state.phase, cinder_core::engine::state::GamePhase::Active);
    assert!(
        !state.actor_is_defeated("abeni_adeyemi", &pack.settings.combat.health_stat_id),
        "failure must leave remaining civilians alive and protectable"
    );
    assert!(
        !state.actor_is_defeated("chen_yu_xin", &pack.settings.combat.health_stat_id),
        "failure must leave remaining civilians alive and protectable"
    );
}

#[test]
fn floor5_courtyard_hexagonal_navigation_resolves() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");

    // Center has no physical connection to Floor 4; connects only to the 6 perimeter points
    let center = pack.room("courtyard_center").expect("center exists");
    assert!(
        !center
            .exits
            .iter()
            .any(|e| e.room_id == "teleport_platform"),
        "floor 4 and floor 5 must not be physically connected by an exit"
    );
    let platform = pack.room("teleport_platform").expect("platform exists");
    assert!(
        !platform
            .exits
            .iter()
            .any(|e| e.room_id == "courtyard_center"),
        "floor 4 teleport platform must not have a physical exit to floor 5"
    );
    for perimeter_id in &EXPECTED_COURTYARD_ROOMS[1..] {
        assert!(
            center.exits.iter().any(|e| e.room_id == *perimeter_id),
            "center connects directly to perimeter room {perimeter_id}"
        );
    }

    // Each perimeter point connects back to center
    for perimeter_id in &EXPECTED_COURTYARD_ROOMS[1..] {
        let room = pack.room(perimeter_id).unwrap();
        assert!(
            room.exits.iter().any(|e| e.room_id == "courtyard_center"),
            "perimeter room {perimeter_id} connects back to courtyard_center"
        );
    }

    // Perimeter ring connections (clockwise and counter-clockwise)
    let ring_pairs = [
        ("courtyard_north", "courtyard_northeast"),
        ("courtyard_northeast", "courtyard_southeast"),
        ("courtyard_southeast", "courtyard_south"),
        ("courtyard_south", "courtyard_southwest"),
        ("courtyard_southwest", "courtyard_northwest"),
        ("courtyard_northwest", "courtyard_north"),
    ];

    for (room_a_id, room_b_id) in ring_pairs {
        let room_a = pack.room(room_a_id).unwrap();
        let room_b = pack.room(room_b_id).unwrap();
        assert!(
            room_a.exits.iter().any(|e| e.room_id == room_b_id),
            "{room_a_id} connects to {room_b_id}"
        );
        assert!(
            room_b.exits.iter().any(|e| e.room_id == room_a_id),
            "{room_b_id} connects to {room_a_id}"
        );
    }
}

#[test]
fn floor5_sentry_captain_defeat_drops_courtyard_key() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "courtyard_center".to_string();
    state
        .actor_stats
        .entry("player".to_string())
        .or_default()
        .insert("strength".to_string(), 20);

    // Sentry captain carries the cage key
    assert_eq!(
        state.actor_item_count("citadel_sentry_captain", "courtyard-cage-key"),
        1
    );

    // Set captain to 1 HP so a single strike defeats him
    state
        .actor_stats
        .entry("citadel_sentry_captain".to_string())
        .or_default()
        .insert("hp".to_string(), 1);

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");

    // Attack captain to trigger defeat sequence and drop inventory
    let attack_out = runtime
        .run_turn("attack sentry captain")
        .expect("attack captain");
    assert!(
        attack_out.text().contains("strikes")
            || attack_out.text().contains("Sentry Captain")
            || attack_out.text().contains("defeated")
    );

    let state = runtime.export_state().unwrap();
    assert!(
        state.has_item_in_storage(
            "courtyard-cage-key",
            ItemStorageTarget::CurrentRoom,
            "courtyard_center"
        ),
        "courtyard-cage-key should drop into courtyard_center upon captain defeat"
    );

    // Player takes the key
    let take_out = runtime
        .run_turn("take courtyard cage key")
        .expect("take key");
    assert!(
        take_out.text().contains("Picked up") || take_out.text().contains("courtyard cage key")
    );
    let final_state = runtime.export_state().unwrap();
    assert!(final_state.has_item("courtyard-cage-key"));
}

#[test]
fn floor5_unlock_cages_frees_astrid_and_einar() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "courtyard_center".to_string();
    state.acquire_player_item(&pack, "courtyard-cage-key");

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");

    // Astrid and Einar start un-allied
    let s0 = runtime.export_state().unwrap();
    assert_ne!(s0.stance("commander_astrid"), ActorStance::Allied);
    assert_ne!(s0.stance("einar"), ActorStance::Allied);
    assert_ne!(s0.story_vars.get("courtyard_cages_opened"), Some("true"));
    assert_eq!(s0.actor_item_count("einar", "sensory-enhancer"), 1);
    assert!(!s0.has_item("sensory-enhancer"));

    // Unlock cages command
    let unlock_out = runtime.run_turn("unlock cages").expect("unlock cages");
    let text = unlock_out.text();
    assert!(
        text.contains("Commander Astrid")
            || text.contains("iron gates swing open")
            || text.contains("weeping in relief")
    );
    assert!(
        text.contains("Take this sensory enhancer capsule"),
        "Einar must offer sensory enhancer in thanks: {text}"
    );
    assert!(
        !text.contains("warmth of high wisdom"),
        "Must not trigger wisdom awakening on cage unlock: {text}"
    );
    assert!(!text.contains("\"\""), "Must not emit empty quotes: {text}");

    let s1 = runtime.export_state().unwrap();
    assert_eq!(s1.story_vars.get("courtyard_cages_opened"), Some("true"));
    assert!(!s1.has_item("courtyard-cage-key"));
    assert_eq!(s1.stance("commander_astrid"), ActorStance::Allied);
    assert_eq!(s1.stance("einar"), ActorStance::Allied);
    assert!(
        s1.has_item("sensory-enhancer"),
        "Player must receive sensory enhancer from Einar"
    );
    assert_eq!(
        s1.actor_item_count("einar", "sensory-enhancer"),
        0,
        "Einar handed over the sensory enhancer"
    );
}

#[test]
fn floor5_sensory_enhancer_use_sets_story_var_and_consumes_item() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "courtyard_center".to_string();
    state.acquire_player_item(&pack, "sensory-enhancer");

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");

    let s0 = runtime.export_state().unwrap();
    assert!(s0.has_item("sensory-enhancer"));
    assert_ne!(s0.story_vars.get("has_sensory_enhancer"), Some("true"));

    // Swallow the sensory enhancer
    let out = runtime
        .run_turn("use sensory enhancer")
        .expect("use enhancer");
    let text = out.text();
    assert!(
        text.contains("translucent blue capsule")
            || text.contains("senses sharpen")
            || text.contains("electric chill")
    );

    let s1 = runtime.export_state().unwrap();
    assert!(!s1.has_item("sensory-enhancer"));
    assert_eq!(s1.story_vars.get("has_sensory_enhancer"), Some("true"));
}

#[test]
fn floor5_citadel_map_definition_and_reveal_condition() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");

    let map = pack
        .map_for_room("courtyard_center")
        .expect("courtyard_center has map");
    assert_eq!(map.id, "the-frost-citadel");
    assert_eq!(map.label, "The Frost Citadel");
    assert_eq!(map.rooms.len(), 16);

    // Each courtyard room is present in the map
    for room_id in EXPECTED_COURTYARD_ROOMS {
        assert!(
            map.rooms.iter().any(|r| r.room_id == *room_id),
            "map should include room {room_id}"
        );
    }
    assert!(
        map.rooms
            .iter()
            .all(|room| room.room_id != "citadel_sanctum_gate"),
        "citadel_sanctum_gate should not be on the map"
    );

    // Map reveal condition is gated by has_sensory_enhancer
    assert!(
        map.reveal_conditions.iter().any(|c| matches!(
            c,
            cinder_core::content::types::MapRevealCondition::StoryVarTruthy { key } if key == "has_sensory_enhancer"
        )),
        "map should be revealed by has_sensory_enhancer"
    );
}

#[test]
fn floor5_to_floor4_only_via_teleport() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "courtyard_center".to_string();
    state.story_vars.set_unchecked("knows_teleport", "true");
    state
        .story_vars
        .set_unchecked("anchor_floor4_platform", "true");
    state.story_vars.set_unchecked("anchor_floor5_gate", "true");

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");

    // Physical movement up/ascend is rejected because there is no physical connection
    for dir in ["ascend", "up", "go up", "go floor 4"] {
        let outcome = runtime.run_turn(dir).expect("turn runs");
        assert_eq!(
            runtime.current_room_id().unwrap(),
            "courtyard_center",
            "physical command '{dir}' should not move player out of courtyard"
        );
        let text = outcome.text();
        assert!(
            text.contains("see no path")
                || text.contains("cannot go that way")
                || text.contains("can't go that way")
                || text.contains("aren't sure how to")
                || text.contains("route sheet")
                || text.contains("approved playbook")
                || text.contains("unknown"),
            "expected cannot go rejection for '{dir}': {text}"
        );
    }

    // Returning to Floor 4 requires the teleport command
    let tp_out = runtime
        .run_turn("teleport teleport_platform")
        .expect("teleport runs");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "teleport_platform",
        "teleporting to teleport_platform moves player to floor 4"
    );
    assert!(
        tp_out
            .text()
            .contains("brass platform glows with blue light")
    );

    // Teleporting back to Floor 5 courtyard also works
    let tp_back = runtime
        .run_turn("teleport floor 5")
        .expect("teleport back runs");
    assert_eq!(
        runtime.current_room_id().unwrap(),
        "courtyard_center",
        "teleporting to floor 5 moves player to courtyard_center"
    );
    assert!(
        tp_back
            .text()
            .contains("brass platform glows with blue light")
    );
}

#[test]
fn floor5_house_hallways_and_thrones_navigate() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "courtyard_north".to_string();
    state
        .story_vars
        .set_unchecked("frost_wolf_access_open", "true");
    state
        .story_vars
        .set_unchecked("citadel_final_wave_open", "true");

    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime creates");

    // Walk north up House Frost-Wolf hallway to throne
    runtime.run_turn("go north").expect("go north");
    assert_eq!(runtime.current_room_id().unwrap(), "frost_wolf_gallery");

    runtime.run_turn("go north").expect("go north");
    assert_eq!(runtime.current_room_id().unwrap(), "frost_wolf_muster");

    runtime.run_turn("go north").expect("go north");
    assert_eq!(runtime.current_room_id().unwrap(), "frost_wolf_throne");

    // Walk back south to courtyard_north
    runtime.run_turn("go south").expect("go south");
    assert_eq!(runtime.current_room_id().unwrap(), "frost_wolf_muster");

    runtime.run_turn("go south").expect("go south");
    assert_eq!(runtime.current_room_id().unwrap(), "frost_wolf_gallery");

    runtime.run_turn("go south").expect("go south");
    assert_eq!(runtime.current_room_id().unwrap(), "courtyard_north");
}

#[test]
fn floor5_house_access_opens_with_timed_siege_waves() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "courtyard_center".to_string();
    state.active_objective_stage_ids = vec!["mq_free_prisoners".to_string()];
    state
        .stage_started_minutes
        .insert("mq_free_prisoners".to_string(), state.current_time_minutes);
    state.acquire_player_item(&pack, "courtyard-cage-key");

    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime creates");
    runtime.run_turn("unlock cages").expect("unlock cages");

    let preparation = runtime.export_state().expect("export preparation");
    assert!(
        preparation
            .active_objective_stage_ids
            .contains(&"siege_preparation".to_string())
    );
    assert_ne!(preparation.story_vars.get("citadel_siege_wave"), Some("1"));
    assert_ne!(
        preparation.story_vars.get("frost_wolf_access_open"),
        Some("true")
    );
    assert_eq!(spawned_count(&preparation, "frost_wolf_raider"), 0);

    let mut wave1_state = preparation;
    let preparation_started = wave1_state.stage_started_minutes["siege_preparation"];
    wave1_state.current_time_minutes = preparation_started + 10;
    wave1_state.current_room_id = "courtyard_center".to_string();
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let runtime = CinderRuntime::from_state(pack, wave1_state, false).expect("runtime creates");
    runtime.run_turn("go north").expect("advance into wave 1");

    let wave1 = runtime.export_state().expect("export wave 1");
    assert_eq!(wave1.story_vars.get("citadel_siege_wave"), Some("1"));
    assert_eq!(wave1.story_vars.get("frost_wolf_access_open"), Some("true"));
    assert_ne!(wave1.story_vars.get("iron_ram_access_open"), Some("true"));
    assert_ne!(
        wave1.story_vars.get("frost_leopard_access_open"),
        Some("true")
    );
    assert_eq!(spawned_count(&wave1, "frost_wolf_raider"), 3);
    for raider_id in wave1.spawned_actors.keys() {
        assert_eq!(
            wave1
                .actor_room_overrides
                .get(raider_id)
                .map(String::as_str),
            Some("frost_wolf_throne")
        );
    }

    let mut wave2_state = wave1;
    let wave1_started = wave2_state.stage_started_minutes["siege_wave_1"];
    wave2_state.current_time_minutes = wave1_started + 60;
    wave2_state.current_room_id = "courtyard_center".to_string();
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let runtime = CinderRuntime::from_state(pack, wave2_state, false).expect("runtime creates");
    runtime.run_turn("north").expect("advance into wave 2");

    let wave2 = runtime.export_state().expect("export wave 2");
    assert_eq!(wave2.story_vars.get("citadel_siege_wave"), Some("2"));
    assert_eq!(wave2.story_vars.get("iron_ram_access_open"), Some("true"));
    assert_eq!(spawned_count(&wave2, "frost_wolf_raider"), 21);
    assert_eq!(spawned_count(&wave2, "iron_ram_mauler"), 3);
    assert_ne!(
        wave2.story_vars.get("frost_leopard_access_open"),
        Some("true")
    );

    let mut wave3_state = wave2;
    let wave2_started = wave3_state.stage_started_minutes["siege_wave_2"];
    wave3_state.current_time_minutes = wave2_started + 60;
    wave3_state.current_room_id = "courtyard_center".to_string();
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let runtime = CinderRuntime::from_state(pack, wave3_state, false).expect("runtime creates");
    runtime.run_turn("south").expect("advance into wave 3");

    let wave3 = runtime.export_state().expect("export wave 3");
    assert_eq!(wave3.story_vars.get("citadel_siege_wave"), Some("3"));
    assert_eq!(
        wave3.story_vars.get("citadel_final_wave_open"),
        Some("true")
    );
    assert_eq!(
        wave3.story_vars.get("frost_leopard_access_open"),
        Some("true")
    );
    assert_eq!(spawned_count(&wave3, "frost_wolf_raider"), 30);
    assert_eq!(spawned_count(&wave3, "iron_ram_mauler"), 21);
    assert_eq!(spawned_count(&wave3, "frost_leopard_hunter"), 3);
    assert_eq!(spawned_count(&wave3, "lord_vane"), 1);
    assert_eq!(spawned_count(&wave3, "warmaster_torin"), 1);
    assert_eq!(spawned_count(&wave3, "lady_sylvan"), 1);

    let mut completed_dispatches = wave3;
    let leopard_started = completed_dispatches.stage_started_minutes["frost_leopard_dispatch"];
    completed_dispatches.current_time_minutes = leopard_started + 90;
    completed_dispatches.current_room_id = "courtyard_center".to_string();
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let runtime =
        CinderRuntime::from_state(pack, completed_dispatches, false).expect("runtime creates");
    runtime
        .run_turn("go north")
        .expect("complete dispatch queues");
    let completed = runtime.export_state().expect("export completed queues");
    assert_eq!(spawned_count(&completed, "frost_wolf_raider"), 30);
    assert_eq!(spawned_count(&completed, "iron_ram_mauler"), 30);
    assert_eq!(spawned_count(&completed, "frost_leopard_hunter"), 30);
}

#[test]
fn floor5_wave_one_raiders_march_until_their_lane_stops() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "courtyard_center".to_string();
    state.active_objective_stage_ids = vec!["mq_free_prisoners".to_string()];
    state
        .stage_started_minutes
        .insert("mq_free_prisoners".to_string(), state.current_time_minutes);
    state.acquire_player_item(&pack, "courtyard-cage-key");

    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime creates");
    runtime.run_turn("unlock cages").expect("unlock cages");

    let mut state = runtime.export_state().expect("export preparation");
    let preparation_started = state.stage_started_minutes["siege_preparation"];
    state.current_time_minutes = preparation_started + 10;
    state.current_room_id = "courtyard_center".to_string();
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime creates");
    runtime.run_turn("go north").expect("advance into wave 1");
    runtime.run_tick().expect("raiders march");

    let marched = runtime.export_state().expect("export marched state");
    let raider_ids: Vec<_> = marched
        .spawned_actors
        .keys()
        .filter(|actor_id| actor_id.starts_with("frost_wolf_raider-"))
        .cloned()
        .collect();
    assert_eq!(raider_ids.len(), 3);
    for raider_id in &raider_ids {
        assert_eq!(
            marched
                .actor_room_overrides
                .get(raider_id)
                .map(String::as_str),
            Some("frost_wolf_muster")
        );
    }

    let mut stopped = marched;
    stopped
        .story_vars
        .set_unchecked("frost_wolf_lane_stopped", "true");
    stopped.current_time_minutes += 30;
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let runtime = CinderRuntime::from_state(pack, stopped, false).expect("runtime creates");
    let routed = runtime.run_turn("go north").expect("stopped lane turn");

    let after_stop = runtime.export_state().expect("export stopped state");
    assert_eq!(spawned_count(&after_stop, "frost_wolf_raider"), 0);
    assert!(
        routed.text().contains("3 raiders break formation"),
        "lane stop should narrate the deployed soldiers routing: {}",
        routed.text()
    );
    for raider_id in &raider_ids {
        assert!(
            !after_stop.actor_room_overrides.contains_key(raider_id),
            "routed actor state should be removed"
        );
    }
}

#[test]
fn floor5_guard_blocks_wave_soldiers_at_the_north_approach() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "courtyard_center".to_string();
    state.turn_number = 1;
    state.set_stance("elf-knight-2", ActorStance::Allied);
    state.set_follows_player("elf-knight-2", false);
    state
        .party_orders
        .insert("elf-knight-2".to_string(), "guard".to_string());
    state
        .actor_room_overrides
        .insert("elf-knight-2".to_string(), "courtyard_north".to_string());

    let raider_id = match state.spawn_actor(
        &pack,
        SpawnActorConfig {
            template_id: "frost_wolf_raider",
            room_id: Some("courtyard_north"),
            stance: None,
            follows_player: false,
            scale_with_actor_id: None,
            scale_stat: None,
            max_active_instances: None,
        },
    ) {
        SpawnActorOutcome::Success(info) => info.instance_id,
        outcome => panic!("expected raider to spawn, got {outcome:?}"),
    };

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");
    runtime.run_tick().expect("guarded siege tick runs");
    let blocked = runtime.export_state().expect("export blocked state");
    assert_eq!(
        blocked
            .actor_room_overrides
            .get(&raider_id)
            .map(String::as_str),
        Some("courtyard_north"),
        "a living guard should stop the raider at the north approach"
    );

    let mut defeated_raider = blocked.clone();
    defeated_raider
        .actor_stats
        .entry(raider_id.clone())
        .or_default()
        .insert("hp".to_string(), 0);
    defeated_raider.relationships.remove(&raider_id);
    let runtime =
        CinderRuntime::from_state(pack.clone(), defeated_raider, false).expect("runtime creates");
    let defeated_tick = runtime.run_tick().expect("defeated raider tick runs");
    let defeated = runtime.export_state().expect("export defeated state");
    assert_eq!(
        defeated
            .actor_room_overrides
            .get(&raider_id)
            .map(String::as_str),
        Some("courtyard_north"),
        "a defeated raider must not resume its target-rule movement"
    );
    assert!(
        !defeated_tick.text().contains("steps away"),
        "defeated raiders must not narrate movement: {}",
        defeated_tick.text()
    );

    let mut defeated_guard = blocked;
    defeated_guard
        .actor_stats
        .entry("elf-knight-2".to_string())
        .or_default()
        .insert("hp".to_string(), 0);
    let runtime = CinderRuntime::from_state(pack, defeated_guard, false).expect("runtime creates");
    runtime.run_tick().expect("unguarded siege tick runs");
    let resumed = runtime.export_state().expect("export resumed state");
    assert_eq!(
        resumed
            .actor_room_overrides
            .get(&raider_id)
            .map(String::as_str),
        Some("courtyard_center"),
        "the raider should resume marching after the guard falls"
    );
}

#[test]
fn floor5_all_three_stopped_lanes_end_the_siege() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "frost_wolf_throne".to_string();
    state.active_objective_stage_ids = vec![
        "mq_frost_siege".to_string(),
        "sq_five_offerings".to_string(),
        "frost_wolf_dispatch".to_string(),
        "iron_ram_dispatch".to_string(),
        "frost_leopard_dispatch".to_string(),
    ];
    for stage_id in &state.active_objective_stage_ids {
        state
            .stage_started_minutes
            .insert(stage_id.clone(), state.current_time_minutes);
    }
    for template_id in [
        "frost_wolf_raider",
        "iron_ram_mauler",
        "frost_leopard_hunter",
    ] {
        let outcome = state.spawn_actor(
            &pack,
            SpawnActorConfig {
                template_id,
                room_id: Some("courtyard_center"),
                stance: Some(ActorStance::Hostile),
                follows_player: false,
                scale_with_actor_id: None,
                scale_stat: None,
                max_active_instances: None,
            },
        );
        assert!(matches!(outcome, SpawnActorOutcome::Success(_)));
    }
    for stopped_var in [
        "frost_wolf_lane_stopped",
        "iron_ram_lane_stopped",
        "frost_leopard_lane_stopped",
    ] {
        state.story_vars.set_unchecked(stopped_var, "true");
    }

    let siege_end = apply_events(&mut state, &pack, &[]);
    let siege_text = narrative_text(&siege_end.lines);

    assert_eq!(spawned_count(&state, "frost_wolf_raider"), 0);
    assert_eq!(spawned_count(&state, "iron_ram_mauler"), 0);
    assert_eq!(spawned_count(&state, "frost_leopard_hunter"), 0);
    assert!(
        state.completed_stage_ids.contains("mq_frost_siege"),
        "all stopped lanes should complete the siege objective"
    );
    assert!(
        state
            .active_objective_stage_ids
            .contains(&"mq_return_to_courtyard".to_string())
    );
    assert_eq!(state.story_vars.get("citadel_siege_resolved"), Some("true"));
    assert_eq!(state.story_vars.get("citadel_siege_complete"), None);
    assert!(!state.has_item("salt-reach-transit-seal"));
    assert!(
        siege_text.contains("The siege is over"),
        "siege completion should be announced: {siege_text}"
    );
    assert!(
        state.completed_stage_ids.contains("sq_five_offerings"),
        "the protection quest should complete when at least three survive"
    );

    apply_events(&mut state, &pack, &[]);
    assert!(
        state
            .active_objective_stage_ids
            .contains(&"mq_return_to_courtyard".to_string()),
        "the conclusion must wait while Layla is away from the courtyard center"
    );

    let settlement = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::PlayerMoved {
            from_room_id: "frost_wolf_throne".to_string(),
            to_room_id: "courtyard_center".to_string(),
        })],
    );
    let reward_text = narrative_text(&settlement.lines);
    assert_eq!(state.story_vars.get("citadel_siege_complete"), Some("true"));
    assert_eq!(state.story_vars.get("five_offerings_survivors"), Some("5"));
    for (received_var, unlock_var, token_id) in [
        (
            "town_salt_reach_token_received",
            "town_salt_reach_unlocked",
            "salt-reach-transit-seal",
        ),
        (
            "town_glassbank_token_received",
            "town_glassbank_unlocked",
            "glassbank-transit-prism",
        ),
        (
            "town_woolcross_token_received",
            "town_woolcross_unlocked",
            "woolcross-transit-knot",
        ),
        (
            "town_greenrest_token_received",
            "town_greenrest_unlocked",
            "greenrest-transit-seed",
        ),
        (
            "town_brass_yard_token_received",
            "town_brass_yard_unlocked",
            "brass-yard-transit-gear",
        ),
    ] {
        assert_eq!(state.story_vars.get(received_var), Some("true"));
        assert_eq!(state.story_vars.get(unlock_var), None);
        assert!(
            state.has_item(token_id),
            "missing survivor token {token_id}"
        );
    }
    for expected in [
        "Nivi Olsen: You stood between us",
        "Nivi Olsen gives you the Salt Reach Teleportation Token.",
        "Eliška Nováková: You gave us back a future.",
        "Eliška Nováková gives you the Glassbank Teleportation Token.",
        "Amaru Quispe: You held the line",
        "Amaru Quispe gives you the Woolcross Teleportation Token.",
        "Abeni Adeyemi: You kept hope alive",
        "Abeni Adeyemi gives you the Greenrest Teleportation Token.",
        "Chen Yu-xin: Those who fix things",
        "Chen Yu-xin gives you the Brass Yard Teleportation Token.",
    ] {
        assert!(
            reward_text.contains(expected),
            "missing structured survivor reward line {expected:?}: {reward_text}"
        );
    }

    apply_events(&mut state, &pack, &[]);
    assert!(
        state.completed_stage_ids.contains("mq_secure_courtyard"),
        "the main Floor 5 quest should close after survivor resolution"
    );
    let repeated = apply_events(&mut state, &pack, &[]);
    assert!(repeated.lines.is_empty(), "rewards must not repeat");
    assert_eq!(state.item_count("salt-reach-transit-seal"), 1);
}

#[test]
fn floor5_survivor_rewards_unlock_only_living_civilians_towns() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "courtyard_center".to_string();
    state.active_objective_stage_ids = vec![
        "mq_frost_siege".to_string(),
        "sq_five_offerings".to_string(),
    ];
    for stage_id in &state.active_objective_stage_ids {
        state
            .stage_started_minutes
            .insert(stage_id.clone(), state.current_time_minutes);
    }
    for actor_id in ["nivi_olsen", "eliska_novakova"] {
        state
            .actor_stats
            .entry(actor_id.to_string())
            .or_default()
            .insert("hp".to_string(), 0);
    }
    for stopped_var in [
        "frost_wolf_lane_stopped",
        "iron_ram_lane_stopped",
        "frost_leopard_lane_stopped",
    ] {
        state.story_vars.set_unchecked(stopped_var, "true");
    }

    apply_events(&mut state, &pack, &[]);
    apply_events(&mut state, &pack, &[]);
    apply_events(&mut state, &pack, &[]);

    assert_eq!(state.story_vars.get("five_offerings_survivors"), Some("3"));
    assert_eq!(state.story_vars.get("town_salt_reach_unlocked"), None);
    assert_eq!(state.story_vars.get("town_glassbank_unlocked"), None);
    assert_eq!(
        state.story_vars.get("town_woolcross_token_received"),
        Some("true")
    );
    assert_eq!(
        state.story_vars.get("town_greenrest_token_received"),
        Some("true")
    );
    assert_eq!(
        state.story_vars.get("town_brass_yard_token_received"),
        Some("true")
    );
    assert!(!state.has_item("salt-reach-transit-seal"));
    assert!(!state.has_item("glassbank-transit-prism"));
    assert!(state.has_item("woolcross-transit-knot"));
    assert_eq!(state.story_vars.get("town_woolcross_unlocked"), None);
    assert_eq!(pack.resolve_teleport_target(&state, "woolcross"), None);

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");
    let used = runtime
        .run_turn("use woolcross teleportation token")
        .expect("token use resolves");
    assert!(
        used.text()
            .contains("Woolcross is now a permanent destination"),
        "token use should explain the new destination: {}",
        used.text()
    );
    let state = runtime.export_state().expect("export token state");
    assert!(!state.has_item("woolcross-transit-knot"));
    assert_eq!(
        state.story_vars.get("town_woolcross_unlocked"),
        Some("true")
    );
    assert_eq!(
        pack.resolve_teleport_target(&state, "woolcross")
            .map(|(room_id, _)| room_id),
        Some("woolcross_square".to_string())
    );
    assert_eq!(pack.resolve_teleport_target(&state, "salt reach"), None);
}

#[test]
fn floor5_failed_protection_still_rewards_survivors_and_opens_floor6() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "courtyard_center".to_string();
    state.active_objective_stage_ids = vec![
        "mq_frost_siege".to_string(),
        "sq_five_offerings_failed".to_string(),
    ];
    for stage_id in &state.active_objective_stage_ids {
        state
            .stage_started_minutes
            .insert(stage_id.clone(), state.current_time_minutes);
    }
    state
        .story_vars
        .set_unchecked("five_offerings_failed", "true");
    for actor_id in ["nivi_olsen", "eliska_novakova", "amaru_quispe"] {
        state
            .actor_stats
            .entry(actor_id.to_string())
            .or_default()
            .insert("hp".to_string(), 0);
    }
    for stopped_var in [
        "frost_wolf_lane_stopped",
        "iron_ram_lane_stopped",
        "frost_leopard_lane_stopped",
    ] {
        state.story_vars.set_unchecked(stopped_var, "true");
    }

    apply_events(&mut state, &pack, &[]);
    apply_events(&mut state, &pack, &[]);
    apply_events(&mut state, &pack, &[]);

    assert_eq!(state.story_vars.get("five_offerings_survivors"), Some("2"));
    assert_eq!(
        state.story_vars.get("town_greenrest_token_received"),
        Some("true")
    );
    assert_eq!(
        state.story_vars.get("town_brass_yard_token_received"),
        Some("true")
    );
    assert_eq!(state.story_vars.get("town_woolcross_unlocked"), None);
    assert_eq!(state.story_vars.get("town_greenrest_unlocked"), None);
    assert_eq!(state.story_vars.get("town_brass_yard_unlocked"), None);
    assert_eq!(state.story_vars.get("citadel_siege_complete"), Some("true"));
    assert!(
        state
            .completed_stage_ids
            .contains("sq_five_offerings_failed"),
        "the quest must remain failed after survivor rewards resolve"
    );

    let south = pack.room("courtyard_south").expect("south gate room");
    assert!(
        south
            .exits
            .iter()
            .all(|exit| exit.room_id != "citadel_sanctum_gate"),
        "citadel_sanctum_gate is removed; Floor 6 entry is via clandestine token"
    );
    assert!(
        state.has_item("clandestine-sanctuary-token"),
        "siege completion advances to mq_infiltrate_sanctuary which grants clandestine sanctuary token"
    );
}

#[test]
fn floor5_spawned_siege_armies_do_not_emit_npc_tick_soft_errors() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "courtyard_center".to_string();
    state.current_time_minutes = 90;
    state.active_objective_stage_ids = vec![
        "frost_wolf_dispatch".to_string(),
        "iron_ram_dispatch".to_string(),
        "frost_leopard_dispatch".to_string(),
    ];
    for stage_id in &state.active_objective_stage_ids {
        state.stage_started_minutes.insert(stage_id.clone(), 0);
    }
    for access_var in [
        "frost_wolf_access_open",
        "iron_ram_access_open",
        "frost_leopard_access_open",
    ] {
        state.story_vars.set_unchecked(access_var, "true");
    }

    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime creates");
    runtime.run_turn("go north").expect("dispatch siege armies");
    let mut state = runtime.export_state().expect("export dispatched armies");
    assert_eq!(spawned_count(&state, "frost_wolf_raider"), 30);
    assert_eq!(spawned_count(&state, "iron_ram_mauler"), 30);
    assert_eq!(spawned_count(&state, "frost_leopard_hunter"), 30);
    state.current_room_id = "courtyard_center".to_string();

    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime creates");

    for tick in 1..=6 {
        let outcome = runtime.run_tick().expect("spawned siege actor tick runs");
        assert!(
            !outcome.text().contains("goes still, listening to the dark"),
            "tick {tick} produced NPC soft error: {}",
            outcome.text()
        );
    }
}

#[test]
fn floor5_spawned_hostile_continues_attacking_on_its_cooldown() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "courtyard_north".to_string();
    state.turn_number = 1;
    state
        .story_vars
        .set_unchecked("frost_wolf_lane_stopped", "true");
    let actor_id = match state.spawn_actor(
        &pack,
        SpawnActorConfig {
            template_id: "frost_wolf_raider",
            room_id: Some("courtyard_north"),
            stance: None,
            follows_player: false,
            scale_with_actor_id: None,
            scale_stat: None,
            max_active_instances: None,
        },
    ) {
        SpawnActorOutcome::Success(info) => info.instance_id,
        outcome => panic!("expected spawned hostile, got {outcome:?}"),
    };
    assert_eq!(state.stance(&actor_id), ActorStance::Hostile);
    let initial_hp = state.actor_stat("player", "hp");

    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime creates");
    runtime.run_tick().expect("first autonomous strike");
    let after_first = runtime.export_state().expect("export first strike");
    assert!(
        after_first.actor_stat("player", "hp") < initial_hp,
        "spawned hostile should initiate an attack without a player attack"
    );

    runtime.run_tick().expect("cooldown tick");
    let during_cooldown = runtime.export_state().expect("export cooldown");
    assert_eq!(
        during_cooldown.actor_stat("player", "hp"),
        after_first.actor_stat("player", "hp"),
        "spawned hostile should respect its attack interval"
    );

    runtime.run_tick().expect("second autonomous strike");
    let after_second = runtime.export_state().expect("export second strike");
    assert!(
        after_second.actor_stat("player", "hp") < during_cooldown.actor_stat("player", "hp"),
        "spawned hostile should attack again once its cooldown expires"
    );
}

#[test]
fn floor5_house_gates_prevent_premature_exploration() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");

    for (start, command, blocked_destination) in [
        ("courtyard_north", "go north", "frost_wolf_gallery"),
        ("courtyard_southeast", "go southeast", "iron_ram_gallery"),
        (
            "courtyard_southwest",
            "go southwest",
            "frost_leopard_gallery",
        ),
    ] {
        let mut state = WorldState::new(&pack);
        state.current_room_id = start.to_string();
        let runtime =
            CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");
        runtime
            .run_turn(command)
            .expect("blocked movement resolves");
        assert_eq!(runtime.current_room_id().unwrap(), start);
        assert_ne!(runtime.current_room_id().unwrap(), blocked_destination);
    }
}

#[test]
fn floor5_throne_rooms_stay_sealed_until_final_wave() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    for passage_id in [
        "throne_passage_northeast",
        "throne_passage_south",
        "throne_passage_northwest",
    ] {
        assert!(
            pack.room(passage_id).is_none(),
            "removed outer passage {passage_id} should not load"
        );
    }

    for (start, command, throne) in [
        ("frost_wolf_muster", "go north", "frost_wolf_throne"),
        ("iron_ram_muster", "go southeast", "iron_ram_throne"),
        (
            "frost_leopard_muster",
            "go southwest",
            "frost_leopard_throne",
        ),
    ] {
        let mut state = WorldState::new(&pack);
        state.current_room_id = start.to_string();
        let runtime =
            CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");

        runtime.run_turn(command).expect("blocked throne movement");
        assert_eq!(runtime.current_room_id().unwrap(), start);

        let mut final_wave = runtime.export_state().expect("export state");
        final_wave
            .story_vars
            .set_unchecked("citadel_final_wave_open", "true");
        let runtime =
            CinderRuntime::from_state(pack.clone(), final_wave, false).expect("runtime creates");

        runtime.run_turn(command).expect("open throne movement");
        assert_eq!(runtime.current_room_id().unwrap(), throne);
    }
}

#[test]
fn floor5_house_leaders_march_to_the_courtyard_in_final_wave() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "courtyard_center".to_string();
    state.turn_number = 1;
    state
        .story_vars
        .set_unchecked("citadel_final_wave_open", "true");
    let leaders = [
        ("lord_vane", "frost_wolf_throne", "frost_wolf_muster"),
        ("warmaster_torin", "iron_ram_throne", "iron_ram_muster"),
        (
            "lady_sylvan",
            "frost_leopard_throne",
            "frost_leopard_muster",
        ),
    ];
    let mut instances = Vec::new();
    for (template_id, throne, muster) in leaders {
        let instance_id = match state.spawn_actor(
            &pack,
            SpawnActorConfig {
                template_id,
                room_id: Some(throne),
                stance: None,
                follows_player: false,
                scale_with_actor_id: None,
                scale_stat: None,
                max_active_instances: None,
            },
        ) {
            SpawnActorOutcome::Success(info) => info.instance_id,
            outcome => panic!("expected {template_id} to spawn, got {outcome:?}"),
        };
        instances.push((instance_id, muster));
    }
    assert!(
        state
            .onstage_actors(&pack)
            .any(|actor| actor.id.starts_with("lord_vane-")),
        "spawned leader should be onstage"
    );

    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime creates");
    runtime.run_tick().expect("final-wave leaders march");
    let marched = runtime.export_state().expect("export marched state");

    for (leader, muster) in instances {
        assert_eq!(
            marched
                .actor_room_overrides
                .get(&leader)
                .map(String::as_str),
            Some(muster),
            "{leader} should leave the throne room when the final wave opens"
        );
    }
}

#[test]
fn floor5_defeating_spawned_house_leaders_stops_their_armies() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");

    for (template_id, room_id, command, stopped_var) in [
        (
            "lord_vane",
            "frost_wolf_throne",
            "attack lord vane",
            "frost_wolf_lane_stopped",
        ),
        (
            "warmaster_torin",
            "iron_ram_throne",
            "attack warmaster torin",
            "iron_ram_lane_stopped",
        ),
        (
            "lady_sylvan",
            "frost_leopard_throne",
            "attack lady sylvan",
            "frost_leopard_lane_stopped",
        ),
    ] {
        let mut state = WorldState::new(&pack);
        state.current_room_id = room_id.to_string();
        let leader_id = match state.spawn_actor(
            &pack,
            SpawnActorConfig {
                template_id,
                room_id: Some(room_id),
                stance: None,
                follows_player: false,
                scale_with_actor_id: None,
                scale_stat: None,
                max_active_instances: None,
            },
        ) {
            SpawnActorOutcome::Success(info) => info.instance_id,
            outcome => panic!("expected {template_id} to spawn, got {outcome:?}"),
        };
        state
            .actor_stats
            .entry(leader_id)
            .or_default()
            .insert("hp".to_string(), 1);

        let runtime =
            CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");
        runtime.run_turn(command).expect("leader attack resolves");
        let defeated = runtime.export_state().expect("export defeated state");
        assert_eq!(
            defeated.story_vars.get(stopped_var),
            Some("true"),
            "defeating spawned {template_id} should stop its house"
        );
    }
}

#[test]
fn floor5_arrival_starts_free_prisoners_quest_and_unlock_completes_it() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    // Player is on Floor 4 teleport platform with the quest to descend
    state.current_room_id = "teleport_platform".to_string();
    state.active_objective_stage_ids = vec!["mq_use_teleport_platform".to_string()];
    state.story_vars.set_unchecked("knows_teleport", "true");
    state.story_vars.set_unchecked("anchor_floor5_gate", "true");
    state.acquire_player_item(&pack, "courtyard-cage-key");

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");

    // Before descent: active quest is The Teleportation Scroll / mq_use_teleport_platform
    let objs0 = runtime.current_objective_summaries().unwrap();
    assert!(
        objs0
            .iter()
            .any(|o| o.stage_id == "mq_use_teleport_platform"),
        "mq_use_teleport_platform must be active before descent: {:?}",
        objs0
    );

    // Teleport down to Floor 5 courtyard
    runtime
        .run_turn("teleport floor5_start")
        .expect("teleport to floor 5");
    assert_eq!(runtime.current_room_id().unwrap(), "courtyard_center");

    // Arriving at Floor 5 activates mq_free_prisoners quest
    let objs1 = runtime.current_objective_summaries().unwrap();
    let free_prisoners_quest = objs1
        .iter()
        .find(|o| o.stage_id == "mq_free_prisoners")
        .expect("Free prisoners quest must be active upon Floor 5 arrival");
    assert_eq!(
        free_prisoners_quest.quest_id.as_deref(),
        Some("free_courtyard_prisoners")
    );
    assert_eq!(
        free_prisoners_quest.quest_title.as_deref(),
        Some("What They Put in the Cages")
    );
    assert!(
        !free_prisoners_quest.summary.trim().is_empty(),
        "Free-prisoners stage must expose a player-facing summary"
    );

    // Layla unlocks the cages
    let unlock_out = runtime.run_turn("unlock cages").expect("unlock cages");
    let text = unlock_out.text();
    assert!(
        text.contains("Take this sensory enhancer capsule"),
        "Einar must give sensory enhancer upon cage release: {text}"
    );

    let state_after = runtime.export_state().unwrap();
    assert!(
        state_after.has_item("sensory-enhancer"),
        "Player must receive sensory enhancer as reward"
    );
    assert!(
        state_after
            .completed_stage_ids
            .contains("mq_free_prisoners"),
        "mq_free_prisoners must be completed in state"
    );

    // Quest is now complete and cleared from active objectives
    let objs2 = runtime.current_objective_summaries().unwrap();
    assert!(
        !objs2.iter().any(|o| o.stage_id == "mq_free_prisoners"),
        "Completed quest must no longer be in active objectives: {:?}",
        objs2
    );
}

#[test]
fn floor5_entry_activates_free_prisoners_quest_from_initial_listener() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "courtyard_north".to_string();

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");

    let objs0 = runtime.current_objective_summaries().unwrap();
    assert!(!objs0.iter().any(|o| o.stage_id == "mq_free_prisoners"));

    // Walk into courtyard_center
    runtime.run_turn("south").expect("walk south");
    assert_eq!(runtime.current_room_id().unwrap(), "courtyard_center");

    let objs1 = runtime.current_objective_summaries().unwrap();
    assert!(
        objs1.iter().any(|o| o.stage_id == "mq_free_prisoners"),
        "Stepping into courtyard_center must activate mq_free_prisoners: {:?}",
        objs1
    );
}

#[test]
fn tracing_platform_sigil_does_not_count_as_descending() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "teleport_platform".to_string();
    state.active_objective_stage_ids = vec!["mq_use_teleport_platform".to_string()];
    state.story_vars.set_unchecked("knows_teleport", "true");
    state
        .story_vars
        .set_unchecked("anchor_floor4_platform", "true");
    state.story_vars.set_unchecked("anchor_floor5_gate", "true");
    state.acquire_player_item(&pack, "magic-chalk");
    state
        .actor_stats
        .entry("player".to_string())
        .or_default()
        .insert("mp".to_string(), 20);

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");

    runtime
        .run_turn("trace teleport sigil")
        .expect("trace platform sigil");

    // The platform activates, but Layla is still standing on Floor 4.
    assert_eq!(runtime.current_room_id().unwrap(), "teleport_platform");
    let after = runtime.export_state().unwrap();
    assert_eq!(after.story_vars.get("platform_activated"), Some("true"));

    // "Get below the wall." must survive activation.
    assert!(
        !after
            .completed_stage_ids
            .contains("mq_use_teleport_platform"),
        "descent quest completed by tracing the sigil: {:?}",
        after.completed_stage_ids
    );
    assert!(
        after
            .active_objective_stage_ids
            .contains(&"mq_use_teleport_platform".to_string()),
        "descent quest lost before arriving on Floor 5: {:?}",
        after.active_objective_stage_ids
    );
    assert_ne!(after.story_vars.get("descend_floor_5"), Some("true"));

    // Only actual arrival completes it (transit is exclusively via teleportation).
    runtime
        .run_turn("teleport floor 5")
        .expect("teleport floor 5");
    assert_eq!(runtime.current_room_id().unwrap(), "courtyard_center");
    let arrived = runtime.export_state().unwrap();
    assert!(
        arrived
            .completed_stage_ids
            .contains("mq_use_teleport_platform")
    );
}

#[test]
fn teleport_from_floor4_to_floor5_brings_following_party_members() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);

    // Player is on Floor 4 teleport platform
    state.current_room_id = "teleport_platform".to_string();
    state.story_vars.set_unchecked("knows_teleport", "true");
    state
        .story_vars
        .set_unchecked("anchor_floor4_platform", "true");
    state.story_vars.set_unchecked("anchor_floor5_gate", "true");

    // Sakhra is with player on teleport_platform following player and allied
    state.set_actor_stance(&pack, "sakhra", ActorStance::Allied, true);
    state
        .actor_room_overrides
        .insert("sakhra".to_string(), "teleport_platform".to_string());

    // Zayd is left behind in village_square on Floor 4, also following player
    state.set_follows_player("zayd", true);
    state
        .actor_room_overrides
        .insert("zayd".to_string(), "village_square".to_string());

    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime creates");

    // Teleport down from Floor 4 to Floor 5 courtyard
    let outcome = runtime
        .run_turn("teleport floor5_start")
        .expect("teleport runs");
    assert_eq!(runtime.current_room_id().unwrap(), "courtyard_center");
    assert!(
        outcome.text().contains("Sakhra crosses the room with you")
            || outcome.text().contains("Sakhra"),
        "Sakhra should accompany the player through the teleport platform: {}",
        outcome.text()
    );

    let s1 = runtime.export_state().unwrap();
    assert_eq!(
        s1.actor_room_id("sakhra", "teleport_platform"),
        "courtyard_center",
        "Sakhra must travel with player to Floor 5 courtyard"
    );
    assert_eq!(
        s1.actor_room_id("zayd", "village_square"),
        "village_square",
        "Zayd was left in village_square and must NOT magically warp to Floor 5"
    );

    // Layla can issue party orders to Sakhra on Floor 5
    let order_out = runtime.run_turn("order sakhra guard").expect("order runs");
    assert!(
        order_out.text().contains("guard") || !order_out.text().contains("unavailable"),
        "Sakhra can be ordered on Floor 5: {}",
        order_out.text()
    );

    // Moving on Floor 5: Sakhra is on guard, stays in courtyard_center; Zayd stays on Floor 4
    runtime.run_turn("go north").expect("move runs");
    assert_eq!(runtime.current_room_id().unwrap(), "courtyard_north");
    let s2 = runtime.export_state().unwrap();
    assert_eq!(
        s2.actor_room_id("sakhra", "teleport_platform"),
        "courtyard_center",
        "Guarding Sakhra stays in courtyard_center"
    );
    assert_eq!(
        s2.actor_room_id("zayd", "village_square"),
        "village_square",
        "Zayd still stays on Floor 4"
    );
}

#[test]
fn floor5_sentries_do_not_attack_caged_offerings_when_player_absent() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    // Player is on Floor 4 teleport chamber
    state.current_room_id = "r4_teleport_chamber".to_string();

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");

    // Run background ticks while player is absent from the courtyard
    for _ in 0..5 {
        runtime.run_tick().expect("tick runs successfully");
    }

    let exported = runtime.export_state().unwrap();
    let health_stat = &pack.settings.combat.health_stat_id;
    let offerings = [
        "nivi_olsen",
        "eliska_novakova",
        "amaru_quispe",
        "abeni_adeyemi",
        "chen_yuxin",
    ];

    for offering in offerings {
        assert!(
            !exported.actor_is_defeated(offering, health_stat),
            "Offering {offering} must NOT be killed by sentries while caged in courtyard!"
        );
    }
    assert_eq!(
        exported.story_vars.get("five_offerings_deaths"),
        None,
        "No offering deaths should occur while sentries guard the cages"
    );
}

#[test]
fn floor5_squad_party_marches_together_and_promotes_follower_on_leader_defeat() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "courtyard_center".to_string();
    state.turn_number = 1;
    state
        .story_vars
        .set_unchecked("citadel_final_wave_open", "true");

    // 1. Spawn squad of 3 frost wolf raiders at frost_wolf_throne
    let l_id = match state.spawn_actor(
        &pack,
        SpawnActorConfig {
            template_id: "frost_wolf_raider",
            room_id: Some("frost_wolf_throne"),
            stance: None,
            follows_player: false,
            scale_with_actor_id: None,
            scale_stat: None,
            max_active_instances: None,
        },
    ) {
        SpawnActorOutcome::Success(info) => info.instance_id,
        outcome => panic!("expected leader raider, got {outcome:?}"),
    };
    let f1_id = match state.spawn_actor(
        &pack,
        SpawnActorConfig {
            template_id: "frost_wolf_raider",
            room_id: Some("frost_wolf_throne"),
            stance: None,
            follows_player: false,
            scale_with_actor_id: None,
            scale_stat: None,
            max_active_instances: None,
        },
    ) {
        SpawnActorOutcome::Success(info) => info.instance_id,
        outcome => panic!("expected follower 1 raider, got {outcome:?}"),
    };
    let f2_id = match state.spawn_actor(
        &pack,
        SpawnActorConfig {
            template_id: "frost_wolf_raider",
            room_id: Some("frost_wolf_throne"),
            stance: None,
            follows_player: false,
            scale_with_actor_id: None,
            scale_stat: None,
            max_active_instances: None,
        },
    ) {
        SpawnActorOutcome::Success(info) => info.instance_id,
        outcome => panic!("expected follower 2 raider, got {outcome:?}"),
    };

    // Configure squad: l_id deploys to courtyard_center, f1 and f2 follow l_id
    state
        .party_orders
        .insert(l_id.clone(), "deploy courtyard_center".to_string());
    state
        .party_orders
        .insert(f1_id.clone(), "follow".to_string());
    state.set_followed_actor(&f1_id, Some(&l_id));
    state
        .party_orders
        .insert(f2_id.clone(), "follow".to_string());
    state.set_followed_actor(&f2_id, Some(&l_id));

    // 2. Tick: leader marches toward courtyard_center (frost_wolf_muster), followers sync
    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");
    runtime.run_tick().expect("squad march 1");
    let state = runtime.export_state().expect("export state");

    assert_eq!(state.actor_room_id(&l_id, ""), "frost_wolf_muster");
    assert_eq!(state.actor_room_id(&f1_id, ""), "frost_wolf_muster");
    assert_eq!(state.actor_room_id(&f2_id, ""), "frost_wolf_muster");

    // 3. Player is at frost_wolf_muster and defeats the squad leader
    let mut state = state;
    state.current_room_id = "frost_wolf_muster".to_string();
    state
        .actor_stats
        .entry(l_id.clone())
        .or_default()
        .insert("hp".to_string(), 1);

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");
    let out = runtime
        .run_turn("attack raider")
        .expect("attack leader resolves");
    let state = runtime.export_state().expect("export state");

    assert!(
        state.actor_is_defeated(&l_id, "hp"),
        "attack output: {}",
        out.text()
    );

    // Follower 1 should now be promoted to leader with "deploy courtyard_center"
    assert_eq!(
        state.party_order(&pack, &f1_id),
        Some("deploy courtyard_center".to_string())
    );
    assert_eq!(state.followed_actor(&f1_id), None);
    // Follower 2 should now follow follower 1
    assert_eq!(state.followed_actor(&f2_id), Some(f1_id.as_str()));

    // 4. Player returns to courtyard_center; new leader and follower resume march to frost_wolf_gallery
    let mut state = state;
    state.current_room_id = "courtyard_center".to_string();
    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime creates");
    runtime.run_tick().expect("squad march 2");
    let state = runtime.export_state().expect("export state");

    assert_eq!(state.actor_room_id(&f1_id, ""), "frost_wolf_gallery");
    assert_eq!(state.actor_room_id(&f2_id, ""), "frost_wolf_gallery");
}

#[test]
fn caged_actors_cannot_be_charmed_or_follow_player_before_unlocking_cages() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads and validates");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "courtyard_center".to_string();

    // Verify caged actors are marked as imprisoned initially
    assert!(state.actor_is_imprisoned(&pack, "commander_astrid"));
    assert!(state.actor_is_imprisoned(&pack, "einar"));
    assert!(state.actor_is_imprisoned(&pack, "nivi_olsen"));
    assert!(state.actor_is_imprisoned(&pack, "eliska_novakova"));

    // If an imprisoned actor has follows_player = true, moving player must NOT move them
    state.set_follows_player("commander_astrid", true);
    state.set_follows_player("nivi_olsen", true);

    let runtime = CinderRuntime::from_state(pack.clone(), state, false).expect("runtime creates");
    runtime
        .run_turn("go north")
        .expect("walk to courtyard_north");
    let s_after_move = runtime.export_state().unwrap();

    assert_eq!(s_after_move.current_room_id, "courtyard_north");
    assert_eq!(
        s_after_move.actor_room_id("commander_astrid", "courtyard_center"),
        "courtyard_center",
        "Astrid must remain locked in courtyard_center cage"
    );
    assert_eq!(
        s_after_move.actor_room_id("nivi_olsen", "courtyard_center"),
        "courtyard_center",
        "Civilians must remain locked in courtyard_center cage"
    );

    // Now unlock cages and verify they are no longer imprisoned
    let mut s_unlocked = s_after_move;
    s_unlocked
        .story_vars
        .set_unchecked("courtyard_cages_opened", "true");
    assert!(!s_unlocked.actor_is_imprisoned(&pack, "commander_astrid"));
    assert!(!s_unlocked.actor_is_imprisoned(&pack, "nivi_olsen"));
}
