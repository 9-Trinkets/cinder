use crate::engine::commands::{player_command_help_text, player_command_suggestions};
use crate::content::types::ContentPack;
use crate::engine::narrative::NarrativeLines;
use crate::engine::state::WorldState;
pub(crate) fn handle_narrative_line(text: &str, lines: &mut NarrativeLines) {
    lines.narration(text.to_string());
}

pub(crate) fn handle_action_rejected(message: &str, lines: &mut NarrativeLines) {
    if !message.is_empty() {
        lines.error(message.to_string());
    }
}

pub(crate) fn handle_help_shown(
    _state: &mut WorldState,
    content: &ContentPack,
    lines: &mut NarrativeLines,
) {
    let available_commands = player_command_help_text(content);
    lines.narration(content.render_template(
        &content.opening.help_text,
        &[("available_commands", available_commands.as_str())],
    ));
}

pub(crate) fn handle_unknown_input(
    content: &ContentPack,
    raw_input: &str,
    lines: &mut NarrativeLines,
) {
    let available_commands = player_command_suggestions(content);
    lines.narration(content.render_template(
        &content.presentation.error_text.unknown_input,
        &[
            ("raw_input", raw_input),
            ("available_commands", available_commands.as_str()),
        ],
    ));
}