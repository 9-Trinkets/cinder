//! Floor 6 (the High Sanctuary): the invisibility potion, the patrolling knights,
//! the servants' clues, and the Seer and his dragon.

use cinder_core::content::loader::load_named_pack;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::{ActorStance, WorldState};

const RING: &[&str] = &[
    "sanctuary_nave",
    "sanctuary_cistern_tower",
    "sanctuary_knights_hall",
    "sanctuary_censer_tower",
    "sanctuary_chapel",
    "sanctuary_bell_tower",
    "sanctuary_scriptorium",
    "sanctuary_lamp_tower",
];

fn runtime_in(room: &str, setup: impl FnOnce(&mut WorldState)) -> CinderRuntime {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = room.to_string();
    state.clock.turn_number = 1;
    setup(&mut state);
    CinderRuntime::from_state(pack, state, false).expect("runtime creates")
}

fn put(state: &mut WorldState, actor: &str, room: &str) {
    state
        .actors
        .actor_room_overrides
        .insert(actor.to_string(), room.to_string());
}

fn player_hp(runtime: &CinderRuntime) -> i32 {
    runtime.export_state().unwrap().actor_stat("player", "hp")
}

fn set_hp(state: &mut WorldState, actor: &str, hp: i32) {
    state
        .combat
        .actor_stats
        .get_mut(actor)
        .unwrap_or_else(|| panic!("{actor} has stats"))
        .insert("hp".to_string(), hp);
}

fn strong_player(state: &mut WorldState) {
    let stats = state
        .combat
        .actor_stats
        .entry("player".to_string())
        .or_default();
    stats.insert("strength".to_string(), 100);
}

// --- the potion ---------------------------------------------------------------

#[test]
fn the_potion_hides_layla_for_eight_minutes_then_wears_off() {
    let runtime = runtime_in("sanctuary_nave", |s| s.add_item("invisibility-potion"));
    assert!(!runtime.export_state().unwrap().is_player_invisible());

    let drink = runtime
        .run_turn("drink invisibility potion")
        .expect("drink");
    assert!(drink.text().contains("fade from sight"), "{}", drink.text());
    let state = runtime.export_state().unwrap();
    assert!(state.is_player_invisible());
    assert_eq!(state.story_vars.get("is_invisible"), Some("true"));
    assert_eq!(state.player_invisibility_remaining_minutes(), 8);

    runtime.run_turn("go east").expect("move");
    assert_eq!(
        runtime
            .export_state()
            .unwrap()
            .player_invisibility_remaining_minutes(),
        7
    );
    let mut worn_off = String::new();
    for i in 0..7 {
        let command = if i % 2 == 0 { "go west" } else { "go east" };
        worn_off = runtime.run_turn(command).expect("move").text();
    }
    assert!(worn_off.contains("invisibility wears off"), "{worn_off}");
    assert!(!runtime.export_state().unwrap().is_player_invisible());
}

#[test]
fn a_knight_strikes_a_visible_layla_but_ignores_an_invisible_one() {
    let visible = runtime_in("sanctuary_nave", |s| {
        put(s, "sanctuary_knight_1", "sanctuary_nave")
    });
    let before = player_hp(&visible);
    visible.run_tick().expect("tick");
    assert!(player_hp(&visible) < before, "the knight should strike");

    let hidden = runtime_in("sanctuary_nave", |s| {
        put(s, "sanctuary_knight_1", "sanctuary_nave");
        s.add_item("invisibility-potion");
    });
    hidden.run_turn("drink invisibility potion").expect("drink");
    let before = player_hp(&hidden);
    hidden.run_tick().expect("tick");
    assert_eq!(player_hp(&hidden), before, "the knight must not see her");
}

#[test]
fn attacking_ends_the_invisibility_early() {
    let runtime = runtime_in("sanctuary_nave", |s| {
        put(s, "sanctuary_knight_1", "sanctuary_nave");
        s.add_item("invisibility-potion");
        strong_player(s);
    });
    runtime
        .run_turn("drink invisibility potion")
        .expect("drink");
    assert!(runtime.export_state().unwrap().is_player_invisible());
    let out = runtime.run_turn("attack knight").expect("attack");
    assert!(
        out.text().contains("breaks your invisibility"),
        "{}",
        out.text()
    );
    assert!(!runtime.export_state().unwrap().is_player_invisible());
}

// --- the patrol ----------------------------------------------------------------

#[test]
fn the_knights_walk_the_ring_clockwise() {
    let runtime = runtime_in("sanctuary_garden", |s| s.clock.current_time_minutes = 0);
    let mut seen: Vec<String> = Vec::new();
    for _ in 0..60 {
        runtime.run_turn("look").expect("time passes");
        runtime.run_tick().expect("tick");
        let state = runtime.export_state().unwrap();
        let room = state
            .actor_room_id("sanctuary_knight_1", "sanctuary_knights_hall")
            .to_string();
        if seen.last() != Some(&room) {
            seen.push(room);
        }
    }
    assert!(seen.len() >= 5, "the knight should keep moving: {seen:?}");
    for pair in seen.windows(2) {
        let from = RING
            .iter()
            .position(|r| *r == pair[0])
            .expect("on the ring");
        let to = RING
            .iter()
            .position(|r| *r == pair[1])
            .expect("on the ring");
        assert_eq!(to, (from + 1) % RING.len(), "not clockwise: {seen:?}");
    }
}

#[test]
fn the_ring_can_be_walked_by_name() {
    let runtime = runtime_in("sanctuary_nave", |_| {});
    assert_eq!(
        {
            runtime.run_turn("go clockwise").expect("turn");
            runtime.current_room_id().unwrap()
        },
        "sanctuary_cistern_tower"
    );
    runtime.run_turn("go counterclockwise").expect("turn");
    assert_eq!(runtime.current_room_id().unwrap(), "sanctuary_nave");
}

// --- the servants ----------------------------------------------------------------

#[test]
fn the_servants_point_toward_cell_seven() {
    let runtime = runtime_in("sanctuary_garden", |_| {});
    let four = runtime.run_turn("ask servant 4").expect("ask");
    assert!(four.text().contains("Number Seven"), "{}", four.text());
    let six = runtime.run_turn("ask servant 6").expect("ask");
    assert!(six.text().contains("third time"), "{}", six.text());
    let two = runtime.run_turn("greet servant 2").expect("greet");
    assert!(
        two.text().contains("May the sanctuary be spotless"),
        "{}",
        two.text()
    );
}

// --- the Seer and the dragon -------------------------------------------------

#[test]
fn the_dragon_falling_wounds_the_seer_and_killing_him_ends_the_fight() {
    let runtime = runtime_in("sanctuary_sanctum", |s| {
        strong_player(s);
        set_hp(s, "sanctuary_dragon", 1);
    });
    let out = runtime.run_turn("attack dragon").expect("attack");
    assert!(out.text().contains("storm of grey dust"), "{}", out.text());
    let state = runtime.export_state().unwrap();
    assert_eq!(state.story_vars.get("dragon_defeated"), Some("true"));
    assert_eq!(state.actor_stat("the_seer", "hp"), 10);

    let out = runtime.run_turn("attack seer valerius").expect("attack");
    assert!(out.text().contains("Valerius is down"), "{}", out.text());
    assert_eq!(
        runtime
            .export_state()
            .unwrap()
            .story_vars
            .get("seer_defeated"),
        Some("true")
    );
}

#[test]
fn killing_the_seer_first_dissolves_the_dragon() {
    let runtime = runtime_in("sanctuary_sanctum", |s| {
        strong_player(s);
        set_hp(s, "the_seer", 1);
    });
    runtime.run_turn("attack seer valerius").expect("attack");
    let state = runtime.export_state().unwrap();
    assert_eq!(state.story_vars.get("seer_defeated"), Some("true"));
    assert_eq!(state.actor_room_id("sanctuary_dragon", "x"), "");
}

// --- no allies until the platform opens --------------------------------------

#[test]
fn layla_arrives_alone_and_her_golems_stay_behind() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.current_room_id = "courtyard_center".to_string();
    state.acquire_player_item(&pack, "clandestine-sanctuary-token");
    state.set_actor_stance(&pack, "golem-dark-nw", ActorStance::Allied, true);
    put(&mut state, "golem-dark-nw", "courtyard_center");
    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime creates");

    runtime
        .run_turn("use clandestine-sanctuary-token")
        .expect("token");
    assert_eq!(runtime.current_room_id().unwrap(), "sanctuary_terrace");
    let state = runtime.export_state().unwrap();
    assert_eq!(
        state.actor_room_id("golem-dark-nw", ""),
        "courtyard_center",
        "allies must not follow her through the token"
    );
}

#[test]
fn valerius_drops_the_bind_scroll_and_reading_it_teaches_the_sigil() {
    let runtime = runtime_in("sanctuary_sanctum", |s| {
        strong_player(s);
        set_hp(s, "the_seer", 1);
    });
    runtime.run_turn("attack seer valerius").expect("attack");
    let state = runtime.export_state().unwrap();
    assert!(
        state
            .loose_room_items("sanctuary_sanctum")
            .iter()
            .any(|(item, count)| item == "bind-scroll" && *count > 0),
        "the scroll should lie where he fell"
    );

    runtime.run_turn("take bind scroll").expect("take");
    assert!(runtime.export_state().unwrap().has_item("bind-scroll"));
    let read = runtime.run_turn("read bind scroll").expect("read");
    assert!(read.text().contains("Bind Sigil"), "{}", read.text());
    let state = runtime.export_state().unwrap();
    assert!(state.has_item("bind-sigil"));
    assert_eq!(state.story_vars.get("knows_bind"), Some("true"));
}

#[test]
fn orva_joins_the_party_as_a_healer_after_she_opens_the_door() {
    let runtime = runtime_in("sanctuary_cliff_garden", |_| {});
    let state = runtime.export_state().unwrap();
    assert!(!state.follows_player("orva"));

    runtime.run_turn("say the words").expect("talk");
    let state = runtime.export_state().unwrap();
    assert!(state.follows_player("orva"));
    assert_eq!(state.stance("orva"), ActorStance::Allied);

    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let orva = pack.actor("orva").expect("orva");
    assert!(orva.tags.iter().any(|tag| tag == "healer"));

    // She walks down the stair with Layla.
    runtime.run_turn("go down").expect("down");
    runtime.run_turn("go down").expect("down");
    assert_eq!(runtime.current_room_id().unwrap(), "sanctuary_sanctum");
    assert_eq!(
        runtime.export_state().unwrap().actor_room_id("orva", ""),
        "sanctuary_sanctum"
    );
}
