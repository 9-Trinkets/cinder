use super::super::{ActiveMenuData, LookOptionData, MenuOptionData};
use cinder_core::content::types::PanelDataSource;
use cinder_core::engine::runtime::CinderRuntime;

pub(crate) fn build_look_options(runtime: &CinderRuntime) -> Result<Vec<LookOptionData>, String> {
    Ok(runtime
        .room_interactable_options()
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|option| LookOptionData {
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

    #[test]
    fn interactable_labels_include_room_exits() {
        let look_opts = vec![
            LookOptionData {
                id: "__room__".to_string(),
                title: "Village Square".to_string(),
                command: "look".to_string(),
            },
            LookOptionData {
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
