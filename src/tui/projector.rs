use crate::tui::effects::{TimedTextFrame, TimedTextPlayback};
use crate::tui::theme::RosePineMoon;
use crate::content::types::{OpeningMovieDefinition, UiTextDefinition};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use std::time::Duration;
use tachyonfx::EffectRenderer;

pub fn movie_playback(
    movie: OpeningMovieDefinition,
    reveal_effect: tachyonfx::Effect,
) -> TimedTextPlayback {
    let frames = movie
        .frames
        .into_iter()
        .map(|frame| TimedTextFrame {
            text: frame.text,
            duration: Duration::from_millis(frame.duration_ms.max(300)),
        })
        .collect::<Vec<_>>();
    TimedTextPlayback::new(
        movie.title,
        frames,
        reveal_effect,
        Duration::from_millis(800),
    )
}

pub fn render_modal(
    frame: &mut Frame,
    playback: &mut TimedTextPlayback,
    ui_text: &UiTextDefinition,
    frame_interval: Duration,
) {
    let bezel_area = projector_rect(frame.area());
    let screen_area = Rect {
        x: bezel_area.x.saturating_add(2),
        y: bezel_area.y.saturating_add(1),
        width: bezel_area.width.saturating_sub(4),
        height: bezel_area.height.saturating_sub(3),
    };
    frame.render_widget(Clear, bezel_area);
    frame.render_widget(
        Block::default().style(Style::default().bg(RosePineMoon::BASE)),
        bezel_area,
    );
    let bezel = Block::default()
        .title(format!(
            "{} • {}",
            ui_text.projector_title_prefix,
            playback.title()
        ))
        .title_style(
            Style::default()
                .fg(RosePineMoon::GOLD)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(RosePineMoon::HIGHLIGHT_HIGH))
        .style(Style::default().bg(RosePineMoon::CRT_BEZEL));
    frame.render_widget(bezel, bezel_area);
    let screen_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(RosePineMoon::CRT_DIM))
        .style(Style::default().bg(ratatui::style::Color::Black));
    frame.render_widget(screen_block, screen_area);
    let sections = Layout::vertical([Constraint::Min(3), Constraint::Length(1)])
        .margin(1)
        .split(screen_area);
    frame.render_widget(
        Paragraph::new(Text::from(projector_frame_lines(
            playback.current_text(),
            sections[0].width,
            sections[0].height,
        )))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false })
        .style(
            Style::default()
                .fg(RosePineMoon::CRT_GLOW)
                .bg(ratatui::style::Color::Black),
        ),
        sections[0],
    );
    frame.render_effect(
        playback.reveal_effect_mut(),
        sections[0],
        frame_interval.into(),
    );
    frame.render_widget(
        Paragraph::new(if playback.is_finished() {
            ui_text.modal_close_hint.clone()
        } else {
            ui_text.projector_skip_hint.clone()
        })
        .alignment(Alignment::Center)
        .style(
            Style::default()
                .fg(RosePineMoon::CRT_DIM)
                .bg(ratatui::style::Color::Black),
        ),
        sections[1],
    );
}

fn projector_rect(area: Rect) -> Rect {
    let max_width = area.width.saturating_sub(4);
    let max_height = area.height.saturating_sub(2);
    let width = max_width.saturating_div(2).max(40);
    let height = (((width as u32 * 51) / 100) as u16 + 4).clamp(12, max_height.max(12));

    let vertical = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .split(area);
    let horizontal = Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .split(vertical[0]);
    horizontal[0]
}

fn projector_frame_lines(
    frame_text: &str,
    available_width: u16,
    available_height: u16,
) -> Vec<Line<'static>> {
    let available_width = available_width as usize;
    let available_height = available_height as usize;
    if available_width == 0 || available_height == 0 {
        return Vec::new();
    }

    let source_lines = frame_text.lines().collect::<Vec<_>>();
    let source_width = source_lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    if source_lines.is_empty() || source_width == 0 {
        return vec![Line::default(); available_height];
    }

    let scale = (source_width as f32 / available_width as f32)
        .max(source_lines.len() as f32 / available_height as f32)
        .max(1.0);
    let target_width = ((source_width as f32) / scale)
        .round()
        .max(1.0)
        .min(available_width as f32) as usize;
    let target_height = ((source_lines.len() as f32) / scale)
        .round()
        .max(1.0)
        .min(available_height as f32) as usize;
    let fitted_lines = if scale > 1.0 {
        scale_projector_art(&source_lines, source_width, target_width, target_height)
    } else {
        source_lines
            .iter()
            .map(|line| (*line).to_string())
            .collect()
    };

    let left_padding = available_width
        .saturating_sub(target_width)
        .saturating_div(2);
    let art_lines = fitted_lines
        .into_iter()
        .map(|line| {
            Line::from(Span::styled(
                format!("{}{}", " ".repeat(left_padding), line),
                Style::default()
                    .fg(RosePineMoon::CRT_GLOW)
                    .bg(ratatui::style::Color::Black),
            ))
        })
        .collect::<Vec<_>>();
    if art_lines.len() >= available_height || available_height == 0 {
        return art_lines;
    }
    let top_padding = (available_height - art_lines.len()) / 2;
    let bottom_padding = available_height - art_lines.len() - top_padding;
    let mut centered = Vec::with_capacity(available_height);
    centered.extend(std::iter::repeat_with(Line::default).take(top_padding));
    centered.extend(art_lines);
    centered.extend(std::iter::repeat_with(Line::default).take(bottom_padding));
    centered
}

fn scale_projector_art(
    source_lines: &[&str],
    source_width: usize,
    target_width: usize,
    target_height: usize,
) -> Vec<String> {
    let source_grid = source_lines
        .iter()
        .map(|line| {
            let mut chars = line.chars().collect::<Vec<_>>();
            chars.resize(source_width, ' ');
            chars
        })
        .collect::<Vec<_>>();
    if source_grid.is_empty() || target_width == 0 || target_height == 0 {
        return Vec::new();
    }

    let source_height = source_grid.len();
    (0..target_height)
        .map(|row| {
            let source_row = ((row * source_height) / target_height).min(source_height - 1);
            let row_chars = &source_grid[source_row];
            let mut scaled_line = String::with_capacity(target_width);
            for col in 0..target_width {
                let source_col = ((col * source_width) / target_width).min(source_width - 1);
                scaled_line.push(row_chars[source_col]);
            }
            scaled_line.trim_end().to_string()
        })
        .collect()
}
