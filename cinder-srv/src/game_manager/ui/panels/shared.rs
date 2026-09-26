use super::super::PanelConfigData;
use cinder_core::content::types::PanelConfig;

pub(crate) fn panel_config_data(pc: &PanelConfig) -> PanelConfigData {
    PanelConfigData {
        title: pc.title.clone(),
        prompt: pc.prompt.clone(),
        data_source: pc.data_source.clone(),
        on_select: pc.on_select.clone(),
    }
}

pub(crate) fn title_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut capitalize_next = true;
    for c in s.chars() {
        if c.is_alphanumeric() {
            if capitalize_next {
                for uc in c.to_uppercase() {
                    result.push(uc);
                }
                capitalize_next = false;
            } else {
                result.push(c);
            }
        } else {
            result.push(c);
            if c != '\'' {
                capitalize_next = true;
            }
        }
    }
    result
}