pub(crate) mod action_bar;
pub(crate) mod equipment;
pub(crate) mod options;
pub(crate) mod overflow;
pub(crate) mod runtime;
pub(crate) mod shared;

pub(crate) use super::PanelOptionData;

pub(super) use action_bar::{
    build_action_bar_items, build_drop_panel_options, build_use_panel_options,
};
#[cfg(test)]
pub(super) use action_bar::build_action_bar_and_take;
pub(super) use equipment::build_equipment_panel_options;
pub(super) use options::build_panel_options;
pub(super) use overflow::build_overflow_actions;
pub(super) use runtime::{
    build_active_menu, build_interactable_labels, build_look_options, build_talk_options,
};
pub(super) use shared::{panel_config_data, title_case};

#[cfg(test)]
mod panel_tests;