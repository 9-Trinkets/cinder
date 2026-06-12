use crate::tui::effects::TranscriptAnimationSnapshot;
use crate::tui::theme::RosePineMoon;
use cinder_core::content::types::UiTextDefinition;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

pub fn max_scroll_for_area(transcript: &[String], width: u16, height: u16) -> u16 {
    let visible = height.saturating_sub(2) as usize;
    let lines = content_lines(transcript, width);
    lines.saturating_sub(visible) as u16
}

pub fn lines(
    transcript: &[String],
    animation: Option<TranscriptAnimationSnapshot>,
    pending_entries: &[usize],
    ui_text: &UiTextDefinition,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for (index, entry) in transcript.iter().enumerate() {
        if let Some(animation) = animation
            && animation.entry_index == index
        {
            lines.extend(animated_lines(entry, animation.visible_chars, ui_text));
        } else if pending_entries.contains(&index) {
            continue;
        } else {
            for line in entry.lines() {
                lines.push(styled_line(line, ui_text));
            }
        }
        lines.push(Line::default());
    }
    lines
}

pub fn content_lines(transcript: &[String], width: u16) -> usize {
    let content_width = width.saturating_sub(3).max(1) as usize;
    transcript
        .iter()
        .map(|entry| {
            entry
                .lines()
                .map(|line| wrapped_line_count(line, content_width))
                .sum::<usize>()
                + 1
        })
        .sum::<usize>()
}

fn animated_lines(
    entry: &str,
    visible_chars: usize,
    ui_text: &UiTextDefinition,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut remaining = visible_chars;

    for line in entry.split('\n') {
        let line_len = line.chars().count();
        let visible_in_line = remaining.min(line_len);
        let partial = line.chars().take(visible_in_line).collect::<String>();
        let line_complete = visible_in_line == line_len;
        if !partial.is_empty() || line_complete {
            lines.push(styled_line(&partial, ui_text));
        }
        remaining = remaining.saturating_sub(visible_in_line);
        if !line_complete {
            break;
        }
    }

    lines
}

fn wrapped_line_count(line: &str, width: usize) -> usize {
    let count = line.chars().count();
    if count == 0 { 1 } else { count.div_ceil(width) }
}

fn styled_line(line: &str, ui_text: &UiTextDefinition) -> Line<'static> {
    let style = if line.starts_with("> ") {
        Style::default().fg(RosePineMoon::PINE)
    } else if is_npc_movement_line(line) {
        Style::default()
            .fg(RosePineMoon::MUTED)
            .add_modifier(Modifier::ITALIC)
    } else if line.starts_with(&ui_text.error_prefix) {
        Style::default().fg(RosePineMoon::LOVE)
    } else if line.starts_with("== ") && line.ends_with(" ==") {
        Style::default()
            .fg(RosePineMoon::FOAM)
            .add_modifier(Modifier::BOLD)
    } else if line.starts_with("You notice:")
        || line.starts_with("People here:")
        || line.starts_with("Exits:")
        || line.starts_with("Objective:")
    {
        Style::default().fg(RosePineMoon::GOLD)
    } else if line.contains(": ") && !line.starts_with("I don't") {
        Style::default().fg(RosePineMoon::ROSE)
    } else {
        Style::default().fg(RosePineMoon::TEXT)
    };
    Line::from(Span::styled(line.to_string(), style))
}

fn is_npc_movement_line(line: &str) -> bool {
    line.contains(" heads to the ") || line.contains(" comes in from the ")
}
