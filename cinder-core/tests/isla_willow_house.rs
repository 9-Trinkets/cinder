//! Willow House: the building Isla can walk around in.

use cinder_core::content::loader::load_named_pack;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::WorldState;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

fn runtime() -> CinderRuntime {
    let pack = load_named_pack("isla", Some("en")).expect("isla loads");
    let state = WorldState::new(&pack);
    CinderRuntime::from_state(pack, state, false).expect("runtime creates")
}

fn turn(runtime: &CinderRuntime, input: &str) -> String {
    runtime
        .run_turn(input)
        .unwrap_or_else(|e| panic!("turn '{input}' failed: {e}"))
        .text()
}

#[test]
fn every_room_is_reachable_from_the_porch() {
    let pack = load_named_pack("isla", Some("en")).expect("isla loads");
    let graph: BTreeMap<&str, Vec<&str>> = pack
        .rooms
        .iter()
        .map(|room| {
            (
                room.id.as_str(),
                room.exits.iter().map(|e| e.room_id.as_str()).collect(),
            )
        })
        .collect();
    let mut seen = BTreeSet::from(["porch"]);
    let mut queue = VecDeque::from(["porch"]);
    while let Some(room) = queue.pop_front() {
        for next in &graph[room] {
            if seen.insert(*next) {
                queue.push_back(*next);
            }
        }
    }
    let all: BTreeSet<&str> = graph.keys().copied().collect();
    assert_eq!(seen, all, "rooms not reachable from the porch");
    assert_eq!(all.len(), 17);
}

#[test]
fn every_way_in_has_a_way_back() {
    let pack = load_named_pack("isla", Some("en")).expect("isla loads");
    for room in &pack.rooms {
        for exit in &room.exits {
            let back = pack.room(&exit.room_id).expect("exit target exists");
            assert!(
                back.exits.iter().any(|e| e.room_id == room.id),
                "{} leads to {} but there is no way back",
                room.id,
                exit.room_id
            );
        }
    }
}

#[test]
fn every_room_is_worth_looking_at() {
    let pack = load_named_pack("isla", Some("en")).expect("isla loads");
    for room in &pack.rooms {
        assert!(
            !room.inspect_text.trim().is_empty(),
            "{} has no text",
            room.id
        );
        assert!(room.features.len() >= 3, "{} has too few features", room.id);
        for feature in &room.features {
            assert!(
                !feature.inspect_text.trim().is_empty(),
                "{}:{} has no text",
                room.id,
                feature.id
            );
        }
    }
}

#[test]
fn player_facing_text_has_no_parentheses() {
    let pack = load_named_pack("isla", Some("en")).expect("isla loads");
    let mut texts: Vec<(String, String)> =
        vec![("intro".to_string(), pack.opening.intro_text.clone())];
    for room in &pack.rooms {
        texts.push((room.id.clone(), room.inspect_text.clone()));
        texts.push((format!("{}:summary", room.id), room.summary.clone()));
        for feature in &room.features {
            texts.push((
                format!("{}:{}", room.id, feature.id),
                feature.inspect_text.clone(),
            ));
        }
    }
    for actor in &pack.actors {
        texts.push((actor.id.clone(), actor.inspect_text.clone()));
    }
    for item in &pack.items {
        texts.push((item.id.clone(), item.description.clone()));
    }
    for (name, text) in texts {
        assert!(
            !text.contains('(') && !text.contains(')'),
            "{name} contains parentheses: {text}"
        );
    }
}

#[test]
fn isla_starts_on_the_porch_and_can_walk_to_the_top_of_the_house() {
    let runtime = runtime();
    assert!(turn(&runtime, "look").contains("Front Porch"));
    for (command, expected) in [
        ("go inside", "Waiting Room"),
        ("go hall", "Ground-Floor Hall"),
        ("go upstairs", "Second-Floor Landing"),
        ("go upstairs", "Third-Floor Landing"),
        ("go book room", "Book Room"),
        ("go landing", "Third-Floor Landing"),
        ("go downstairs", "Second-Floor Landing"),
        ("go downstairs", "Ground-Floor Hall"),
        ("go downstairs", "Basement"),
    ] {
        let text = turn(&runtime, command);
        assert!(
            text.contains(expected),
            "'{command}' should reach {expected}: {text}"
        );
    }
}

#[test]
fn isla_can_look_at_things_and_people() {
    let runtime = runtime();
    turn(&runtime, "go inside");
    assert!(turn(&runtime, "look at fish tank").contains("snail"));
    assert!(turn(&runtime, "look at gloria").contains("nineteen years"));
    turn(&runtime, "go hall");
    turn(&runtime, "go playroom");
    assert!(turn(&runtime, "look at timer").contains("pear"));
}

/// The transcript renders a line shaped like `Name: text` as that person's
/// dialogue. Narration must not contain a colon, or a sentence such as
/// "A road runs across the rug: a bakery..." would turn into a speaker label.
#[test]
fn narration_has_no_colons() {
    let pack = load_named_pack("isla", Some("en")).expect("isla loads");
    let mut texts: Vec<(String, String)> =
        vec![("intro".to_string(), pack.opening.intro_text.clone())];
    for room in &pack.rooms {
        texts.push((room.id.clone(), room.inspect_text.clone()));
        texts.push((format!("{}:summary", room.id), room.summary.clone()));
        for feature in &room.features {
            texts.push((
                format!("{}:{}", room.id, feature.id),
                feature.inspect_text.clone(),
            ));
        }
    }
    for actor in &pack.actors {
        texts.push((actor.id.clone(), actor.inspect_text.clone()));
    }
    for item in &pack.items {
        texts.push((item.id.clone(), item.description.clone()));
    }
    for (name, text) in texts {
        let chars: Vec<char> = text.chars().collect();
        for (i, c) in chars.iter().enumerate() {
            let is_clock_time = i > 0
                && chars[i - 1].is_ascii_digit()
                && chars.get(i + 1).is_some_and(char::is_ascii_digit);
            assert!(
                *c != ':' || is_clock_time,
                "{name} contains a colon: {text}"
            );
        }
    }
}

/// Willow House is a place Isla knows, not a dungeon: every map shows in full
/// from the start. Layla keeps its fog of war.
#[test]
fn isla_maps_are_never_fogged_and_layla_maps_still_are() {
    use cinder_core::content::types::MapRevealCondition;
    let isla = load_named_pack("isla", Some("en")).expect("isla loads");
    for map in &isla.maps {
        assert!(
            map.reveal_conditions
                .iter()
                .any(|c| matches!(c, MapRevealCondition::Always)),
            "{} is fogged",
            map.id
        );
    }
    let layla = load_named_pack("layla", Some("en")).expect("layla loads");
    assert!(layla.maps.iter().all(|m| {
        m.reveal_conditions
            .iter()
            .all(|c| !matches!(c, MapRevealCondition::Always))
    }));
}
