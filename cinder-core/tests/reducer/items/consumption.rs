//! Item consumption tests: generic `use`/`eat`/`drink` commands and hook effect reduction.

use super::super::common::*;
use super::item;
use cinder_core::content::loader::load_named_pack;
use cinder_core::engine::events::{TimestampedWorldEvent, WorldEvent};
use cinder_core::engine::reducer::apply_events;
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::WorldState;

#[test]
fn player_used_item_event_consumes_item_and_runs_hook() {
    let mut pack = reducer_test_pack();
    let mut potion = item("healing-potion", "healing potion", "Restores vitality.");
    potion.use_hook = "item.potion_used".to_string();
    pack.items = vec![potion];
    pack.hooks.insert(
        "item.potion_used".to_string(),
        serde_json::json!({
            "rule": "effect_table",
            "rule_config": {
                "cases_path": "rules",
                "next_on_match": "complete",
                "next_on_default": "complete",
                "default_payload_template": {
                    "effects": []
                }
            },
            "input_overlay": {
                "rules": [
                    {
                        "conditions": [],
                        "payload_template": {
                            "kind": "adjust_actor_stat",
                            "actor_id": "$input.actor_id",
                            "stat": "stamina",
                            "delta": 5
                        }
                    }
                ]
            }
        }),
    );
    rebuild_test_pack_indexes(&mut pack);

    let mut state = WorldState::new(&pack);
    let player_id = &pack.settings.combat.player_actor_id;
    state
        .actor_stats
        .entry(player_id.clone())
        .or_default()
        .insert("stamina".to_string(), 5);
    state.add_item("healing-potion");
    assert_eq!(state.item_count("healing-potion"), 1);

    let outcome = apply_events(
        &mut state,
        &pack,
        &[TimestampedWorldEvent::now(WorldEvent::PlayerUsedItem {
            item_id: "healing-potion".to_string(),
        })],
    );

    assert_eq!(state.item_count("healing-potion"), 0);
    assert_eq!(state.actor_stat(player_id, "stamina"), 10);
    assert!(outcome.lines.iter().any(|line| line.text.contains("healing potion")));
}

#[test]
fn player_can_eat_and_use_flatbread_in_layla_pack() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    let player_id = pack.settings.combat.player_actor_id.clone();
    // Injure player slightly
    state
        .actor_stats
        .entry(player_id.clone())
        .or_default()
        .insert("hp".to_string(), 6);
    state.add_item("date-flatbread");
    state.add_item("date-flatbread");

    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime creates");

    // 1. "eat date flatbread"
    let outcome = runtime.run_turn("eat date flatbread").expect("turn runs");
    assert!(outcome.text.contains("You break the warm date flatbread and eat"));
    {
        let s = runtime.export_state().unwrap();
        assert_eq!(s.item_count("date-flatbread"), 1);
        assert_eq!(s.actor_stat(&player_id, "hp"), 10); // 6 + 4 = 10
    }

    // 2. "use date-flatbread"
    let outcome2 = runtime.run_turn("use date-flatbread").expect("turn runs");
    assert!(outcome2.text.contains("You break the warm date flatbread and eat"));
    {
        let s = runtime.export_state().unwrap();
        assert_eq!(s.item_count("date-flatbread"), 0);
    }
}

#[test]
fn player_can_consume_with_short_name_eat_bread() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.add_item("date-flatbread");

    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime creates");

    let outcome = runtime.run_turn("eat bread").expect("turn runs");
    assert!(outcome.text.contains("You break the warm date flatbread and eat"));
    {
        let s = runtime.export_state().unwrap();
        assert_eq!(s.item_count("date-flatbread"), 0);
    }
}

#[test]
fn rejects_using_unheld_or_non_usable_item() {
    let pack = load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.add_item("brass-gear");

    let runtime = CinderRuntime::from_state(pack, state, false).expect("runtime creates");

    // 1. Not carried
    let outcome1 = runtime.run_turn("eat date flatbread").expect("turn runs");
    assert!(outcome1.text.contains("not carrying") || outcome1.text.contains("don't have"));

    // 2. Carried but has no use_hook
    let outcome2 = runtime.run_turn("use brass-gear").expect("turn runs");
    assert!(outcome2.text.contains("cannot use"));
}
