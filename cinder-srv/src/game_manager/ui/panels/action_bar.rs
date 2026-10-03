use super::super::{ActionBarAction, PanelOptionData, PartyMember, droppable_inventory_items};
use super::{panel_config_data, title_case};
use cinder_core::content::types::ContentPack;
use cinder_core::engine::state::WorldState;
use cinder_core::engine::turn_policies::action_is_available;

/// Builds the action bar actions along with options for the dynamic
/// `take` and `give` panels.
///
/// - `take`: Surfaces when loose room items or companion items exist.
/// - `give`: Surfaces when companions are present and the player has inventory items.
pub(crate) fn build_action_bar_items(
    content: &ContentPack,
    state: &WorldState,
    party: &[PartyMember],
) -> (
    Vec<ActionBarAction>,
    Vec<PanelOptionData>,
    Vec<PanelOptionData>,
) {
    let mut action_bar_actions: Vec<ActionBarAction> = if !content.actions.is_empty() {
        content
            .actions
            .iter()
            .filter(|a| {
                // 'take' and 'give' are dynamically placed when applicable
                if a.id == "take" || a.id == "give" {
                    return false;
                }
                a.ui.bar && action_is_available(content, state, a, &state.current_room_id)
            })
            .map(|a| ActionBarAction {
                id: a.id.clone(),
                label: a.label.clone(),
                panel: a.ui.panel.clone(),
                panel_config: a.ui.panel_config.as_ref().map(panel_config_data),
                shortcut: a.ui.shortcut,
            })
            .collect()
    } else {
        vec![]
    };
    action_bar_actions.sort_by_key(|a| a.shortcut.unwrap_or(u8::MAX));

    let present_party: Vec<&PartyMember> = party.iter().filter(|m| m.in_room).collect();

    // 1. Take items: from ground and/or companion packs/equipment
    let take_panel_options = if !content.player_can_take_items() {
        vec![]
    } else {
        let loose = takeable_loose_items(content, state);
        let has_companion_items = present_party
            .iter()
            .any(|m| !m.inventory.is_empty() || !m.equipped_items.is_empty());

        let mut options = Vec::new();
        // Ground items
        for (item_id, _) in &loose {
            let title = title_case(content.item_label(item_id));
            let subtitle = if has_companion_items {
                Some("On the ground".to_string())
            } else {
                None
            };
            options.push(PanelOptionData {
                id: format!("ground:{item_id}"),
                title,
                subtitle,
                command: Some(format!("take {item_id}")),
                disabled: false,
                selected: false,
                group: None,
            });
        }
        // Companion items
        for member in &present_party {
            for item in &member.inventory {
                let item_ref = item.id.as_deref().unwrap_or(&item.label);
                options.push(PanelOptionData {
                    id: format!("member_inv:{}:{item_ref}", member.id),
                    title: title_case(&item.label),
                    subtitle: Some(format!("From {}", member.label)),
                    command: Some(format!("take {item_ref} from {}", member.id)),
                    disabled: false,
                    selected: false,
                    group: None,
                });
            }
            for item in &member.equipped_items {
                let item_ref = item.id.as_deref().unwrap_or(&item.label);
                options.push(PanelOptionData {
                    id: format!("member_equip:{}:{item_ref}", member.id),
                    title: title_case(&item.label),
                    subtitle: Some(format!("From {} ({})", member.label, item.slot)),
                    command: Some(format!("take {item_ref} from {}", member.id)),
                    disabled: false,
                    selected: false,
                    group: None,
                });
            }
        }

        options
    };

    // 2. Give items: to party members
    // Step 1: list droppable items. If 1 companion exists, auto-gives to them.
    // If multiple companions exist, UI prompts for recipient in step 2.
    let give_panel_options = if present_party.is_empty() {
        vec![]
    } else {
        let droppable = droppable_inventory_items(state);
        if droppable.is_empty() {
            vec![]
        } else {
            let mut options = Vec::new();
            for item_id in &droppable {
                let command = if present_party.len() == 1 {
                    Some(format!("give {item_id} to {}", present_party[0].id))
                } else {
                    None
                };
                options.push(PanelOptionData {
                    id: item_id.clone(),
                    title: title_case(content.item_label(item_id)),
                    subtitle: None,
                    command,
                    disabled: false,
                    selected: false,
                    group: None,
                });
            }

            options
        }
    };

    (action_bar_actions, take_panel_options, give_panel_options)
}

#[cfg(test)]
pub(crate) fn build_action_bar_and_take(
    content: &ContentPack,
    state: &WorldState,
) -> (Vec<ActionBarAction>, Vec<PanelOptionData>) {
    let (actions, take, _) = build_action_bar_items(content, state, &[]);
    (actions, take)
}

pub(crate) fn takeable_loose_items(
    content: &ContentPack,
    state: &WorldState,
) -> Vec<(String, u32)> {
    state
        .loose_room_items(&state.current_room_id)
        .into_iter()
        .filter(|(item_id, _)| content.item(item_id).is_none_or(|item| item.is_takeable()))
        .collect()
}

/// Option rows for the generic `drop <item>` overflow action.
pub(crate) fn build_drop_panel_options(
    content: &ContentPack,
    state: &WorldState,
) -> Vec<PanelOptionData> {
    if !content.player_can_drop_items() {
        return vec![];
    }
    droppable_inventory_items(state)
        .into_iter()
        .map(|item_id| PanelOptionData {
            id: item_id.clone(),
            title: title_case(content.item_label(&item_id)),
            subtitle: None,
            command: Some(format!("drop {item_id}")),
            disabled: false,
            selected: false,
            group: None,
        })
        .collect()
}

/// Option rows for the generic `use <item>` overflow action.
pub(crate) fn build_use_panel_options(
    content: &ContentPack,
    state: &WorldState,
) -> Vec<PanelOptionData> {
    let current_room_id = &state.current_room_id;
    super::super::usable_inventory_items(content, state)
        .into_iter()
        .map(|item_id| {
            let label = content.item_label(&item_id);
            let item = content.item(&item_id);
            let (title, subtitle) = if let Some(item) = item
                && item.kind == cinder_core::content::types::ItemKind::Key
            {
                let matching_action = content.actions.iter().find(|action| {
                    (action.available.requires_item.as_deref() == Some(item_id.as_str())
                        || action.available.consumes_item.as_deref() == Some(item_id.as_str()))
                        && cinder_core::engine::turn_policies::action_is_available(
                            content,
                            state,
                            action,
                            current_room_id,
                        )
                });
                if let Some(act) = matching_action {
                    (act.label.clone(), Some(title_case(label)))
                } else {
                    (title_case(label), None)
                }
            } else if item_id.ends_with("-scroll") || label.ends_with("scroll") {
                (format!("Read {}", title_case(label)), None)
            } else {
                (title_case(label), None)
            };

            PanelOptionData {
                id: item_id.clone(),
                title,
                subtitle,
                command: Some(format!("use {item_id}")),
                disabled: false,
                selected: false,
                group: None,
            }
        })
        .collect()
}
