use super::*;
use cinder_core::content::types::{
    ActionDefinition, ActionItemCreation, ItemDefinition, ItemStorageTarget,
};
use cinder_core::engine::test_fixtures::minimal_test_pack;

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
    state.add_item_to_storage(
        "drain-sigil",
        ItemStorageTarget::CurrentRoom,
        &room_id,
    );

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
