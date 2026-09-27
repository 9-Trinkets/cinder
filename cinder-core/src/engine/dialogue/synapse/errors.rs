use crate::engine::neuron::RoleExecutionError;

pub(crate) fn format_role_execution_error(error: Box<RoleExecutionError>) -> String {
    if error.plan_rejections.is_empty() {
        return error.to_string();
    }
    let rejection_lines = error
        .plan_rejections
        .iter()
        .enumerate()
        .map(|(index, rejection)| {
            if let Some(step_text) = rejection.step_text.as_deref() {
                format!(
                    "{}. {} => {:?} ({})",
                    index + 1,
                    rejection.step_kind,
                    step_text,
                    rejection.error_message
                )
            } else if let Some(tool_name) = rejection.tool_name.as_deref() {
                format!(
                    "{}. {} {} {:?} ({})",
                    index + 1,
                    rejection.step_kind,
                    tool_name,
                    rejection.tool_args,
                    rejection.error_message
                )
            } else {
                format!(
                    "{}. {} ({})",
                    index + 1,
                    rejection.step_kind,
                    rejection.error_message
                )
            }
        })
        .collect::<Vec<_>>()
        .join(" | ");
    format!(
        "{}. Rejected planner steps: {}",
        error.message, rejection_lines
    )
}
