use super::super::{
    ActionBarAction, ActiveMenuData, LookOptionData, MenuOptionData, PanelConfigData,
    PanelOptionData,
};
use super::msg;
use cinder_core::content::types::{ContentPack, PanelDataSource, PanelSelectAction};
use cinder_core::engine::runtime::CinderRuntime;

pub(crate) fn build_look_options(runtime: &CinderRuntime) -> Result<Vec<LookOptionData>, String> {
    Ok(runtime
        .room_interactable_options()
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|option| LookOptionData {
            group: option
                .id
                .strip_prefix("item:")
                .and_then(|item_id| item_group_heading(runtime.content(), item_id)),
            id: option.id,
            title: option.title,
            command: option.command,
        })
        .collect())
}

pub(crate) fn build_talk_options(runtime: &CinderRuntime) -> Result<Vec<MenuOptionData>, String> {
    Ok(runtime
        .panel_options(&PanelDataSource::ActorsInRoom)
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|option| MenuOptionData {
            id: option.id,
            title: option.title.clone(),
            menu_text: option.title,
        })
        .collect())
}

pub(crate) fn build_active_menu(runtime: &CinderRuntime) -> Result<Option<ActiveMenuData>, String> {
    Ok(runtime
        .current_active_menu_info()
        .map_err(|error| error.to_string())?
        .map(
            |info: cinder_core::engine::runtime::ActiveMenuInfo| ActiveMenuData {
                prompt: info.prompt,
                max_selections: info.max_selections,
                min_selections: info.min_selections,
                selected_ids: info.selected_ids,
                options: info
                    .options
                    .into_iter()
                    .map(|option| MenuOptionData {
                        id: option.id,
                        title: option.title,
                        menu_text: option.menu_text,
                    })
                    .collect(),
            },
        ))
}

/// The action-bar action that opens an open menu's choices, with the panel
/// options it shows. It is placed the way Take and Give are: dynamically, only
/// while there is something to choose.
///
/// A single-select menu answers with the option's position. A multi-select
/// menu toggles each option and ends with a Done option, using the engine's
/// `toggle:<id>` and `done` commands.
pub(crate) fn menu_action(
    content: &ContentPack,
    menu: &ActiveMenuData,
) -> (ActionBarAction, Vec<PanelOptionData>) {
    let multi = menu.max_selections > 0;
    let mut options: Vec<PanelOptionData> = menu
        .options
        .iter()
        .enumerate()
        .map(|(index, option)| PanelOptionData {
            id: option.id.clone(),
            title: option.title.clone(),
            subtitle: (!option.menu_text.trim().is_empty()
                && !option.menu_text.eq_ignore_ascii_case(&option.title))
            .then(|| option.menu_text.clone()),
            command: Some(if multi {
                format!("toggle:{}", option.id)
            } else {
                (index + 1).to_string()
            }),
            disabled: false,
            selected: multi && menu.selected_ids.contains(&option.id),
            group: None,
            panel: None,
        })
        .collect();
    if multi {
        let needed = menu.min_selections.max(1);
        options.push(PanelOptionData {
            id: "done".to_string(),
            title: msg(content, "ui.menu_done", "Done"),
            subtitle: None,
            command: Some("done".to_string()),
            disabled: menu.selected_ids.len() < needed,
            selected: false,
            group: None,
            panel: None,
        });
    }
    let title = if menu.prompt.trim().is_empty() {
        content.ui_text.menu_option_list_title.clone()
    } else {
        menu.prompt.clone()
    };
    let action = ActionBarAction {
        id: "menu".to_string(),
        label: content.ui_text.menu_action_label.clone(),
        panel: Some("menu".to_string()),
        panel_config: Some(PanelConfigData {
            title,
            prompt: String::new(),
            // The options are supplied directly, so the data source is unused.
            data_source: PanelDataSource::default(),
            on_select: PanelSelectAction::ExecuteCommand,
            layout: Some("list".to_string()),
        }),
        shortcut: None,
    };
    (action, options)
}

/// Names the transcript can highlight as interactable: actors, features,
/// items, and exits present here (everything except the room itself).
pub(crate) fn build_interactable_labels(
    look_options: &[LookOptionData],
    exit_labels: &[String],
    inventory_labels: &[String],
) -> Vec<String> {
    let mut labels: Vec<String> = look_options
        .iter()
        .filter(|option| option.id != "__room__")
        .map(|option| option.title.clone())
        .collect();
    labels.extend(exit_labels.iter().cloned());
    labels.extend(inventory_labels.iter().cloned());
    labels.sort();
    labels.dedup();
    labels
}

#[cfg(test)]
mod tests {
    use super::*;

    fn menu(max_selections: usize, selected: &[&str]) -> ActiveMenuData {
        ActiveMenuData {
            prompt: "Pick".to_string(),
            max_selections,
            min_selections: 1,
            selected_ids: selected.iter().map(|id| id.to_string()).collect(),
            options: ["a", "b"]
                .iter()
                .map(|id| MenuOptionData {
                    id: id.to_string(),
                    title: id.to_uppercase(),
                    menu_text: String::new(),
                })
                .collect(),
        }
    }

    #[test]
    fn single_select_menu_options_answer_by_position() {
        let pack = cinder_core::loader::load_named_pack("layla", Some("en")).expect("layla loads");
        let (action, options) = menu_action(&pack, &menu(0, &[]));
        assert_eq!(action.panel.as_deref(), Some("menu"));
        assert_eq!(
            options
                .iter()
                .map(|o| o.command.as_deref().unwrap())
                .collect::<Vec<_>>(),
            ["1", "2"]
        );
    }

    #[test]
    fn multi_select_menu_toggles_and_ends_with_a_done_option() {
        let pack = cinder_core::loader::load_named_pack("layla", Some("en")).expect("layla loads");
        let (_, none_chosen) = menu_action(&pack, &menu(2, &[]));
        assert_eq!(none_chosen[0].command.as_deref(), Some("toggle:a"));
        let done = none_chosen.last().unwrap();
        assert_eq!(done.command.as_deref(), Some("done"));
        assert!(done.disabled, "Done waits for a selection");

        let (_, one_chosen) = menu_action(&pack, &menu(2, &["b"]));
        assert!(one_chosen[1].selected);
        assert!(!one_chosen.last().unwrap().disabled);
    }

    #[test]
    fn interactable_labels_include_room_exits() {
        let look_opts = vec![
            LookOptionData {
                group: None,
                id: "__room__".to_string(),
                title: "Village Square".to_string(),
                command: "look".to_string(),
            },
            LookOptionData {
                group: None,
                id: "feature:f1".to_string(),
                title: "steam pipes".to_string(),
                command: "x steam pipes".to_string(),
            },
        ];
        let exit_labels = vec![
            "North Through the Store".to_string(),
            "South Through the Store".to_string(),
            "Down into the Cave Below".to_string(),
        ];
        let inventory_labels = vec!["Salt Reach Teleportation Token".to_string()];
        let labels = build_interactable_labels(&look_opts, &exit_labels, &inventory_labels);
        assert_eq!(
            labels,
            vec![
                "Down into the Cave Below",
                "North Through the Store",
                "Salt Reach Teleportation Token",
                "South Through the Store",
                "steam pipes",
            ]
        );
    }
}

/// The heading an item is listed under, when its group is named by the pack.
pub(crate) fn item_group_heading(content: &ContentPack, item_id: &str) -> Option<String> {
    let group = &content.item(item_id)?.group;
    if group.is_empty() {
        return None;
    }
    content.ui_text.item_groups.get(group).cloned()
}
