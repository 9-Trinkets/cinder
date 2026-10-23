use super::super::PanelConfigData;
use cinder_core::content::types::ContentPack;
use cinder_core::content::types::PanelConfig;

pub(crate) fn panel_config_data(pc: &PanelConfig) -> PanelConfigData {
    PanelConfigData {
        title: pc.title.clone(),
        prompt: pc.prompt.clone(),
        data_source: pc.data_source.clone(),
        on_select: pc.on_select.clone(),
        layout: None,
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

pub(crate) fn msg(content: &ContentPack, key: &str, fallback: &str) -> String {
    content
        .render_message(key, &[])
        .unwrap_or_else(|| fallback.to_string())
}

pub(crate) fn msg_fmt(
    content: &ContentPack,
    key: &str,
    fallback: &str,
    replacements: &[(&str, &str)],
) -> String {
    content
        .render_message(key, replacements)
        .unwrap_or_else(|| fallback.to_string())
}
