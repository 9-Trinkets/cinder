use super::*;
use cinder_core::content::types::{
    ActionAvailability, ActionDefinition, ActionItemCreation, ActionUi, ItemDefinition,
    ItemStorageTarget,
};
use cinder_core::engine::test_fixtures::{minimal_test_pack, rebuild_test_pack_indexes};

#[test]
fn bar_shows_actions_that_are_bar_only_even_when_not_typed_command() {
    let mut content = minimal_test_pack();
    content.actions = vec![
        ActionDefinition {
            id: "look".to_string(),
            player_enabled: true,
            ui: ActionUi {
                bar: true,
                ..ActionUi::default()
            },
            ..ActionDefinition::default()
        },
        ActionDefinition {
            id: "follow".to_string(),
            player_enabled: false,
            ui: ActionUi {
                bar: true,
                ..ActionUi::default()
            },
            available: ActionAvailability::default(),
            ..ActionDefinition::default()
        },
    ];
    rebuild_test_pack_indexes(&mut content);
    let state = WorldState::new(&content);

    let (bar, _) = build_action_bar_and_take(&content, &state);

    let ids: Vec<&str> = bar.iter().map(|action| action.id.as_str()).collect();
    assert!(ids.contains(&"look"));
    assert!(ids.contains(&"follow"));
}

#[test]
fn overflow_label_prefers_the_authored_action_label() {
    let action = ActionDefinition {
        id: "use_salve".to_string(),
        label: "Use Moss Poultice".to_string(),
        ..ActionDefinition::default()
    };
    assert_eq!(overflow_action_title(&action), "Use Moss Poultice");

    let legacy = ActionDefinition {
        id: "use_salve".to_string(),
        label: String::new(),
        ..ActionDefinition::default()
    };
    assert_eq!(overflow_action_title(&legacy), "Use Salve");
}

#[test]
fn takeable_items_exclude_trace_marks() {
    let mut content = minimal_test_pack();
    content.items.extend([
        ItemDefinition {
            id: "sigil".to_string(),
            label: "sigil".to_string(),
            description: "A fixed mark.".to_string(),
            trace_mark: true,
            ..ItemDefinition::default()
        },
        ItemDefinition {
            id: "scroll".to_string(),
            label: "scroll".to_string(),
            description: "A portable scroll.".to_string(),
            ..ItemDefinition::default()
        },
    ]);
    let mut state = WorldState::new(&content);
    let room_id = state.current_room_id.clone();
    state.add_item_to_storage("sigil", ItemStorageTarget::CurrentRoom, &room_id);

    assert!(takeable_loose_items(&content, &state).is_empty());

    state.add_item_to_storage("scroll", ItemStorageTarget::CurrentRoom, &room_id);
    assert_eq!(
        takeable_loose_items(&content, &state),
        vec![("scroll".to_string(), 1)]
    );
}

#[test]
fn trace_panel_keeps_already_traced_unlocked_marks_visible() {
    let mut content = minimal_test_pack();
    content.ui_text.trace_mark_present_label = "Already traced here".to_string();
    content.items.extend(
        ["charm-sigil", "drain-sigil", "spawn-sigil"]
            .into_iter()
            .map(|id| ItemDefinition {
                id: id.to_string(),
                label: id.to_string(),
                trace_mark: true,
                ..ItemDefinition::default()
            }),
    );
    let action = ActionDefinition {
        item_creation: Some(ActionItemCreation {
            craftable_items: vec![
                "charm-sigil".to_string(),
                "drain-sigil".to_string(),
                "spawn-sigil".to_string(),
            ],
            craftable_item_gates: BTreeMap::from([
                ("drain-sigil".to_string(), "knows_drain".to_string()),
                ("spawn-sigil".to_string(), "knows_spawn".to_string()),
            ]),
            ..ActionItemCreation::default()
        }),
        ..ActionDefinition::default()
    };
    let mut state = WorldState::new(&content);
    state.story_vars.set_unchecked("knows_drain", "true");
    state.story_vars.set_unchecked("knows_spawn", "true");
    let room_id = state.current_room_id.clone();
    state.add_item_to_storage("drain-sigil", ItemStorageTarget::CurrentRoom, &room_id);

    let options = craftable_item_panel_options(&content, &state, &action, "trace");

    assert_eq!(options.len(), 3);
    let drain = options
        .iter()
        .find(|option| option.id == "drain-sigil")
        .unwrap();
    assert!(drain.disabled);
    assert_eq!(drain.subtitle.as_deref(), Some("Already traced here"));
    assert!(drain.command.is_none());
}

#[test]
fn equipment_panel_lists_each_equipped_item_once_and_held_gear_separately() {
    let mut content = minimal_test_pack();
    content.settings.equipment_slots = ["weapon".to_string(), "off-hand".to_string()]
        .into_iter()
        .collect();
    content.items.extend([
        ItemDefinition {
            id: "greatsword".to_string(),
            label: "greatsword".to_string(),
            equip_slots: vec!["weapon".to_string(), "off-hand".to_string()],
            ..ItemDefinition::default()
        },
        ItemDefinition {
            id: "dagger".to_string(),
            label: "dagger".to_string(),
            equip_slots: vec!["weapon".to_string()],
            ..ItemDefinition::default()
        },
    ]);
    let mut state = WorldState::new(&content);
    state
        .equipment
        .insert("weapon".to_string(), "greatsword".to_string());
    state
        .equipment
        .insert("off-hand".to_string(), "greatsword".to_string());
    state.add_item("greatsword");
    state.add_item("dagger");

    let options = build_equipment_panel_options(&content, &state);

    assert_eq!(options.len(), 2);
    assert_eq!(options[0].id, "unequip:greatsword");
    assert_eq!(options[0].command.as_deref(), Some("unequip greatsword"));
    assert_eq!(options[1].id, "equip:dagger");
    assert_eq!(options[1].command.as_deref(), Some("equip dagger"));
    assert!(
        options[1]
            .subtitle
            .as_deref()
            .is_some_and(|subtitle| subtitle.contains("replaces greatsword"))
    );

    let runtime = CinderRuntime::new(content.clone(), false).unwrap();
    let overflow =
        build_overflow_actions(&runtime, &content, &state, &[], &[], &options).unwrap();
    assert_eq!(
        overflow
            .iter()
            .filter(|action| action.id == "equipment")
            .count(),
        1
    );
}
