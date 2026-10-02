use super::super::PanelOptionGroups;
use super::super::PartyMember;
use super::action_bar::takeable_loose_items;
use super::options::craftable_item_panel_options;
use super::overflow::overflow_action_title;
use super::*;
use cinder_core::content::types::{
    ActionAvailability, ActionDefinition, ActionItemCreation, ActionUi, ItemDefinition,
    ItemStorageTarget,
};
use cinder_core::engine::runtime::CinderRuntime;
use cinder_core::engine::state::WorldState;
use cinder_core::engine::test_fixtures::{minimal_test_pack, rebuild_test_pack_indexes};
use std::collections::BTreeMap;

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
fn overflow_label_uses_the_authored_action_label() {
    let action = ActionDefinition {
        id: "read_scroll".to_string(),
        label: "Read Worn Scroll".to_string(),
        ..ActionDefinition::default()
    };
    assert_eq!(overflow_action_title(&action), "Read Worn Scroll");
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
    let overflow = build_overflow_actions(
        &runtime,
        &content,
        &state,
        &[],
        &PanelOptionGroups {
            equipment: options,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        overflow
            .iter()
            .filter(|action| action.id == "equipment")
            .count(),
        1
    );
}

#[test]
fn give_surfaces_in_overflow_above_drop_when_party_and_droppable_items_exist() {
    let mut content = minimal_test_pack();
    content.actions.push(ActionDefinition {
        id: "drop".to_string(),
        player_enabled: true,
        ..ActionDefinition::default()
    });
    content.items.push(ItemDefinition {
        id: "potion".to_string(),
        label: "potion".to_string(),
        ..ItemDefinition::default()
    });
    let mut state = WorldState::new(&content);
    state.add_item("potion");

    let party_member = PartyMember {
        id: "zayd".to_string(),
        label: "Zayd".to_string(),
        order: String::new(),
        level: 1,
        hp: 10,
        hp_max: 10,
        order_panel: "order:zayd".to_string(),
        equipped_items: vec![],
        inventory: vec![],
        skills: vec![],
        in_room: true,
    };

    let runtime = CinderRuntime::new(content.clone(), false).unwrap();
    let drop_opts = build_drop_panel_options(&content, &state);

    // Case 1: No party -> No give
    let (bar_no_party, _, give_opts_no_party) = build_action_bar_items(&content, &state, &[]);
    assert!(!bar_no_party.iter().any(|a| a.id == "give"));
    assert!(give_opts_no_party.is_empty());
    let overflow_no_party = build_overflow_actions(
        &runtime,
        &content,
        &state,
        &[],
        &PanelOptionGroups {
            give: give_opts_no_party,
            drop: drop_opts.clone(),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(!overflow_no_party.iter().any(|a| a.id == "give"));

    // Case 2: Party exists + item exists -> Give appears in overflow directly above drop
    let (bar_with_party, _, give_opts) =
        build_action_bar_items(&content, &state, std::slice::from_ref(&party_member));
    assert!(!bar_with_party.iter().any(|a| a.id == "give"));
    assert_eq!(give_opts.len(), 1);
    assert_eq!(give_opts[0].command.as_deref(), Some("give potion to zayd"));

    let overflow_with_party = build_overflow_actions(
        &runtime,
        &content,
        &state,
        &[],
        &PanelOptionGroups {
            give: give_opts,
            drop: drop_opts,
            ..Default::default()
        },
    )
    .unwrap();
    let give_idx = overflow_with_party.iter().position(|a| a.id == "give");
    let drop_idx = overflow_with_party.iter().position(|a| a.id == "drop");
    assert!(give_idx.is_some());
    assert!(drop_idx.is_some());
    assert_eq!(overflow_with_party[give_idx.unwrap()].group, "items");
    assert_eq!(overflow_with_party[drop_idx.unwrap()].group, "items");
    assert_eq!(give_idx.unwrap() + 1, drop_idx.unwrap());

    // Case 3: Party exists but no items in inventory -> No give
    let empty_state = WorldState::new(&content);
    let (bar_no_items, _, give_opts_no_items) =
        build_action_bar_items(&content, &empty_state, std::slice::from_ref(&party_member));
    assert!(!bar_no_items.iter().any(|a| a.id == "give"));
    assert!(give_opts_no_items.is_empty());
    let empty_drop_opts = build_drop_panel_options(&content, &empty_state);
    let overflow_no_items = build_overflow_actions(
        &runtime,
        &content,
        &empty_state,
        &[],
        &PanelOptionGroups {
            give: give_opts_no_items,
            drop: empty_drop_opts,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(!overflow_no_items.iter().any(|a| a.id == "give"));

    // Case 4: Multiple party members -> item list does NOT multiply combinatorially
    let bess = PartyMember {
        id: "bess".to_string(),
        label: "Bess".to_string(),
        order: String::new(),
        level: 1,
        hp: 12,
        hp_max: 12,
        order_panel: "order:bess".to_string(),
        equipped_items: vec![],
        inventory: vec![],
        skills: vec![],
        in_room: true,
    };
    let (_, _, multi_opts) = build_action_bar_items(&content, &state, &[party_member, bess]);
    assert_eq!(multi_opts.len(), 1);
    assert_eq!(multi_opts[0].id, "potion");
    assert_eq!(multi_opts[0].command, None);
}

#[test]
fn take_surfaces_in_overflow_items_group_when_companion_has_items_even_without_room_items() {
    let mut content = minimal_test_pack();
    content.actions.push(ActionDefinition {
        id: "take".to_string(),
        player_enabled: true,
        ..ActionDefinition::default()
    });
    let state = WorldState::new(&content);

    let party_member = PartyMember {
        id: "zayd".to_string(),
        label: "Zayd".to_string(),
        order: String::new(),
        level: 1,
        hp: 10,
        hp_max: 10,
        order_panel: "order:zayd".to_string(),
        equipped_items: vec![],
        inventory: vec![super::super::InventoryItem {
            id: Some("torch".to_string()),
            label: "torch".to_string(),
            count: 1,
            usable: false,
        }],
        skills: vec![],
        in_room: true,
    };

    let (bar, take_opts, _) = build_action_bar_items(&content, &state, &[party_member]);
    assert!(!bar.iter().any(|a| a.id == "take"));
    assert_eq!(take_opts.len(), 1);
    assert_eq!(
        take_opts[0].command.as_deref(),
        Some("take torch from zayd")
    );
    assert_eq!(take_opts[0].subtitle.as_deref(), Some("From Zayd"));

    let runtime = CinderRuntime::new(content.clone(), false).unwrap();
    let overflow = build_overflow_actions(
        &runtime,
        &content,
        &state,
        &[],
        &PanelOptionGroups {
            take: take_opts,
            ..Default::default()
        },
    )
    .unwrap();
    let take_action = overflow.iter().find(|a| a.id == "take");
    assert!(take_action.is_some());
    assert_eq!(take_action.unwrap().group, "items");
}

#[test]
fn items_section_orders_take_give_drop() {
    let mut content = minimal_test_pack();
    content.actions.push(ActionDefinition {
        id: "take".to_string(),
        player_enabled: true,
        ..ActionDefinition::default()
    });
    content.actions.push(ActionDefinition {
        id: "drop".to_string(),
        player_enabled: true,
        ..ActionDefinition::default()
    });
    content.items.push(ItemDefinition {
        id: "potion".to_string(),
        label: "potion".to_string(),
        ..ItemDefinition::default()
    });
    let mut state = WorldState::new(&content);
    state.add_item("potion");

    let party_member = PartyMember {
        id: "zayd".to_string(),
        label: "Zayd".to_string(),
        order: String::new(),
        level: 1,
        hp: 10,
        hp_max: 10,
        order_panel: "order:zayd".to_string(),
        equipped_items: vec![],
        inventory: vec![super::super::InventoryItem {
            id: Some("torch".to_string()),
            label: "torch".to_string(),
            count: 1,
            usable: false,
        }],
        skills: vec![],
        in_room: true,
    };

    let (bar, take_opts, give_opts) = build_action_bar_items(&content, &state, &[party_member]);
    assert!(!bar.iter().any(|a| a.id == "take" || a.id == "give"));

    let use_opts = vec![PanelOptionData {
        id: "potion".to_string(),
        title: "Potion".to_string(),
        subtitle: None,
        command: Some("use potion".to_string()),
        disabled: false,
        selected: false,
        group: None,
    }];
    let drop_opts = build_drop_panel_options(&content, &state);
    let runtime = CinderRuntime::new(content.clone(), false).unwrap();
    let overflow = build_overflow_actions(
        &runtime,
        &content,
        &state,
        &[],
        &PanelOptionGroups {
            take: take_opts,
            use_item: use_opts,
            give: give_opts,
            drop: drop_opts,
            ..Default::default()
        },
    )
    .unwrap();

    let take_idx = overflow.iter().position(|a| a.id == "take").unwrap();
    let use_idx = overflow.iter().position(|a| a.id == "use").unwrap();
    let give_idx = overflow.iter().position(|a| a.id == "give").unwrap();
    let drop_idx = overflow.iter().position(|a| a.id == "drop").unwrap();

    assert_eq!(overflow[take_idx].group, "items");
    assert_eq!(overflow[use_idx].group, "items");
    assert_eq!(overflow[give_idx].group, "items");
    assert_eq!(overflow[drop_idx].group, "items");

    assert!(take_idx < use_idx);
    assert!(use_idx < give_idx);
    assert!(give_idx < drop_idx);
}

#[test]
fn take_and_give_exclude_party_members_not_in_current_room() {
    let mut content = minimal_test_pack();
    content.actions.push(ActionDefinition {
        id: "take".to_string(),
        player_enabled: true,
        ..ActionDefinition::default()
    });
    content.items.push(ItemDefinition {
        id: "potion".to_string(),
        label: "potion".to_string(),
        ..ItemDefinition::default()
    });
    let mut state = WorldState::new(&content);
    state.add_item("potion");

    let distant_member = PartyMember {
        id: "zayd".to_string(),
        label: "Zayd".to_string(),
        order: "patrol".to_string(),
        level: 1,
        hp: 10,
        hp_max: 10,
        order_panel: "order:zayd".to_string(),
        equipped_items: vec![],
        inventory: vec![super::super::InventoryItem {
            id: Some("torch".to_string()),
            label: "torch".to_string(),
            count: 1,
            usable: false,
        }],
        skills: vec![],
        in_room: false,
    };

    let (_, take_opts, give_opts) = build_action_bar_items(&content, &state, &[distant_member]);

    // Distant member is not in the room: cannot take their torch, cannot give them potion
    assert!(take_opts.is_empty());
    assert!(give_opts.is_empty());
}

#[test]
fn use_panel_surfaces_leaf_paste_under_items() {
    let pack = cinder_core::loader::load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.add_item("leaf-paste");

    let use_opts = build_use_panel_options(&pack, &state);
    assert_eq!(use_opts.len(), 1);
    assert_eq!(use_opts[0].id, "leaf-paste");
    assert_eq!(use_opts[0].title, "Leaf Paste");
    assert_eq!(use_opts[0].command.as_deref(), Some("use leaf-paste"));
}

#[test]
fn use_panel_surfaces_bitter_moss_under_items() {
    let pack = cinder_core::loader::load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.add_item("bitter-moss");

    let use_opts = build_use_panel_options(&pack, &state);
    assert_eq!(use_opts.len(), 1);
    assert_eq!(use_opts[0].id, "bitter-moss");
    assert_eq!(use_opts[0].title, "Bitter Moss");
    assert_eq!(use_opts[0].command.as_deref(), Some("use bitter-moss"));
}

#[test]
fn give_and_drop_surface_when_equipped_item_has_additional_inventory_copies() {
    let mut content = minimal_test_pack();
    content.actions.push(ActionDefinition {
        id: "drop".to_string(),
        player_enabled: true,
        ..ActionDefinition::default()
    });
    content.settings.equipment_slots = ["chest".to_string()].into_iter().collect();
    content.items.push(ItemDefinition {
        id: "leaf-plate".to_string(),
        label: "leaf plate".to_string(),
        equip_slots: vec!["chest".to_string()],
        ..ItemDefinition::default()
    });

    let mut state = WorldState::new(&content);
    // 1 equipped on chest, 1 held in inventory
    state
        .equipment
        .insert("chest".to_string(), "leaf-plate".to_string());
    state.add_item("leaf-plate");

    let party_member = PartyMember {
        id: "zayd".to_string(),
        label: "Zayd".to_string(),
        order: String::new(),
        level: 1,
        hp: 10,
        hp_max: 10,
        order_panel: "order:zayd".to_string(),
        equipped_items: vec![],
        inventory: vec![],
        skills: vec![],
        in_room: true,
    };

    // Give panel options must list leaf-plate
    let (_, _, give_opts) =
        build_action_bar_items(&content, &state, std::slice::from_ref(&party_member));
    assert_eq!(give_opts.len(), 1);
    assert_eq!(give_opts[0].id, "leaf-plate");
    assert_eq!(
        give_opts[0].command.as_deref(),
        Some("give leaf-plate to zayd")
    );

    // Drop panel options must list leaf-plate
    let drop_opts = build_drop_panel_options(&content, &state);
    assert_eq!(drop_opts.len(), 1);
    assert_eq!(drop_opts[0].id, "leaf-plate");
    assert_eq!(drop_opts[0].command.as_deref(), Some("drop leaf-plate"));

    // If inventory count is 0 (only equipped copy remains), neither give nor drop should list it
    state.remove_item("leaf-plate");
    let (_, _, give_opts_empty) =
        build_action_bar_items(&content, &state, std::slice::from_ref(&party_member));
    assert!(give_opts_empty.is_empty());
    let drop_opts_empty = build_drop_panel_options(&content, &state);
    assert!(drop_opts_empty.is_empty());
}

#[test]
fn use_panel_surfaces_scrolls_with_read_prefix() {
    let pack = cinder_core::loader::load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.add_item("drain-scroll");
    state.add_item("teleport-scroll");

    let use_opts = build_use_panel_options(&pack, &state);
    assert_eq!(use_opts.len(), 2);
    assert_eq!(use_opts[0].id, "drain-scroll");
    assert_eq!(use_opts[0].title, "Read Worn Scroll");
    assert_eq!(use_opts[0].command.as_deref(), Some("use drain-scroll"));

    assert_eq!(use_opts[1].id, "teleport-scroll");
    assert_eq!(use_opts[1].title, "Read Teleport Scroll");
    assert_eq!(use_opts[1].command.as_deref(), Some("use teleport-scroll"));
}

#[test]
fn use_panel_surfaces_keys_only_when_in_lock_room() {
    let pack = cinder_core::loader::load_named_pack("layla", Some("en")).expect("layla loads");
    let mut state = WorldState::new(&pack);
    state.add_item("courtyard-cage-key");

    // Outside the courtyard: key should not be in use_panel
    state.current_room_id = "citadel_corridor_south".to_string();
    let use_opts_outside = build_use_panel_options(&pack, &state);
    assert!(use_opts_outside.is_empty());

    // In courtyard_center: key surfaces as an unlock option
    state.current_room_id = "courtyard_center".to_string();
    let use_opts_inside = build_use_panel_options(&pack, &state);
    assert_eq!(use_opts_inside.len(), 1);
    assert_eq!(use_opts_inside[0].id, "courtyard-cage-key");
    assert_eq!(use_opts_inside[0].title, "Unlock Cages");
    assert_eq!(
        use_opts_inside[0].subtitle.as_deref(),
        Some("Courtyard Cage Key")
    );
    assert_eq!(
        use_opts_inside[0].command.as_deref(),
        Some("use courtyard-cage-key")
    );

    // Once already opened: key is no longer usable
    let _ = state.story_vars.set("courtyard_cages_opened", "true");
    let use_opts_opened = build_use_panel_options(&pack, &state);
    assert!(use_opts_opened.is_empty());
}
