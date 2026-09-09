use crate::engine::commands::{player_command_help_text, player_command_suggestions};
use crate::content::types::{ContentPack, PackMessageVoice};
use crate::engine::narrative::NarrativeLines;
use crate::engine::reducer::observation::render_actor_speech_line;
use crate::engine::state::WorldState;

pub(crate) fn handle_narrative_line(text: &str, lines: &mut NarrativeLines) {
    lines.narration(text.to_string());
}

pub(crate) fn handle_action_rejected(
    message: &str,
    content: &ContentPack,
    lines: &mut NarrativeLines,
) {
    if !message.is_empty() {
        push_feedback_line(lines, content, message.to_string(), |lines, text| {
            lines.error(text);
        });
    }
}

pub(crate) fn handle_help_shown(
    _state: &mut WorldState,
    content: &ContentPack,
    lines: &mut NarrativeLines,
) {
    let available_commands = player_command_help_text(content);
    let help = content.render_template(
        &content.opening.help_text,
        &[("available_commands", available_commands.as_str())],
    );
    push_feedback_line(lines, content, help, |lines, text| {
        lines.narration(text);
    });
}

pub(crate) fn handle_unknown_input(
    content: &ContentPack,
    raw_input: &str,
    lines: &mut NarrativeLines,
) {
    let available_commands = player_command_suggestions(content);
    let text = content.render_template(
        &content.presentation.error_text.unknown_input,
        &[
            ("raw_input", raw_input),
            ("available_commands", available_commands.as_str()),
        ],
    );
    push_feedback_line(lines, content, text, |lines, text| {
        lines.narration(text);
    });
}

/// Resolve the handler speaker that fronts deterministic operational feedback:
/// the pack's `feedback_channel_id` (a declared direct channel) names a fixed
/// roster whose non-player participant (e.g. the "handler" actor) gets the
/// lines. `None` when the pack renders feedback with default styling instead.
pub(crate) fn handler_speaker_name(content: &ContentPack) -> Option<String> {
    let channel = content.channel(&content.settings.feedback_channel_id)?;
    let speaker_id = channel
        .participants
        .iter()
        .find(|participant| !content.is_player_actor(participant))?;
    content.actor(speaker_id).map(|actor| actor.name.clone())
}

/// Render `text` as a handler-attributed comms line ("Handler: <text>" via the
/// pack's `actor_speech` template), when the pack fronts feedback through a
/// handler channel. `None` means the caller uses its default styling.
pub(crate) fn handler_attributed_line(content: &ContentPack, text: &str) -> Option<String> {
    let speaker_name = handler_speaker_name(content)?;
    Some(render_actor_speech_line(
        content, &speaker_name, None, text,
    ))
}

/// Push a line, preferring handler attribution when the pack drives feedback
/// through a handler channel; `fallback` styles the line otherwise.
pub(crate) fn push_feedback_line(
    lines: &mut NarrativeLines,
    content: &ContentPack,
    text: String,
    fallback: impl FnOnce(&mut NarrativeLines, String),
) {
    match handler_attributed_line(content, &text) {
        Some(attributed) => lines.channel(attributed),
        None => fallback(lines, text),
    }
}

/// Push an already-rendered pack message, honoring its per-key voice:
/// handler-voiced messages become handler-attributed comms; everything else
/// stays world narration.
pub(crate) fn push_rendered_message(
    lines: &mut NarrativeLines,
    content: &ContentPack,
    text: String,
    voice: PackMessageVoice,
) {
    match voice {
        PackMessageVoice::Handler => match handler_attributed_line(content, &text) {
            Some(attributed) => lines.channel(attributed),
            None => lines.narration(text),
        },
        PackMessageVoice::Narration => lines.narration(text),
    }
}

/// Render and push a pack-authored message, honoring its per-key voice. Skips
/// keys the pack does not define and empty overrides (the authored
/// suppression signal).
pub(crate) fn push_message(
    lines: &mut NarrativeLines,
    content: &ContentPack,
    key: &str,
    replacements: &[(&str, &str)],
) {
    let Some(text) = content.render_message(key, replacements) else {
        return;
    };
    if text.trim().is_empty() {
        return;
    }
    push_rendered_message(lines, content, text, content.message_voice(key));
}