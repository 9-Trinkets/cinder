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

/// Names the transcript can highlight as interactable: actors, features, and
/// items present here (everything except the room itself).
pub(crate) fn build_interactable_labels(look_options: &[LookOptionData]) -> Vec<String> {
    let mut labels: Vec<String> = look_options
        .iter()
        .filter(|option| option.id != "__room__")
        .map(|option| option.title.clone())
        .collect();
    labels.sort();
    labels.dedup();
    labels
}
