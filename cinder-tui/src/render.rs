use crate::effects::{TimedTextPlayback, TranscriptAnimationSnapshot};
use crate::projector;
use crate::theme::Theme;
use crate::transcript;
use cinder_core::content::types::UiTextDefinition;
use cinder_core::MenuChoiceOption;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Margin, Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, Borders, Clear, List, ListItem, ListState, Paragraph,
    Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
};
use std::time::Duration;

pub(crate) struct RenderSnapshot {
    pub title: String,
    pub time: String,
    pub transcript: Vec<String>,
    pub transcript_scroll: u16,
    pub transcript_animation: Option<TranscriptAnimationSnapshot>,
    pub pending_transcript_animation_entries: Vec<usize>,
    pub ui_text: UiTextDefinition,
    pub theme: Theme,
    pub pane_focus: PaneFocus,
    pub input: String,
    pub game_over: bool,
    pub menu: Option<MenuSnapshot>,
    pub shell_modal: Option<ShellModalSnapshot>,
}

pub(crate) struct MenuSnapshot {
    pub options: Vec<MenuChoiceOption>,
    pub selected_index: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PaneFocus {
    Command,
    Transcript,
}

pub(crate) enum ShellModalSnapshot {
    Root {
        selected_index: usize,
        options: Vec<String>,
    },
    Detail {
        title: String,
        body: String,
        hint: String,
        scroll: u16,
    },
    Selection {
        title: String,
        selected_index: usize,
        options: Vec<String>,
        hint: String,
    },
    YelpReview {
        rating: u32,
        review_text: String,
        hint: String,
    },
}

const SHELL_MODAL_WIDTH_PERCENT: u16 = 72;
const SHELL_MODAL_MIN_HEIGHT: u16 = 12;
const SHELL_MODAL_MAX_HEIGHT: u16 = 24;

pub(crate) fn draw(
    frame: &mut Frame,
    snapshot: &RenderSnapshot,
    projector_playback: Option<&mut TimedTextPlayback>,
    frame_interval: Duration,
) {
    frame.render_widget(Clear, frame.area());
    frame.render_widget(
        Block::default().style(Style::default().bg(snapshot.theme.base)),
        frame.area(),
    );

    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(8),
        Constraint::Length(3),
    ])
    .margin(1)
    .split(frame.area());

    let shell = Line::from(vec![
        Span::styled(
            format!(" {} ", snapshot.title),
            Style::default()
                .fg(snapshot.theme.foam)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("• ", Style::default().fg(snapshot.theme.muted)),
        Span::styled(
            snapshot.time.clone(),
            Style::default()
                .fg(snapshot.theme.gold)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" • ", Style::default().fg(snapshot.theme.muted)),
        Span::styled(
            snapshot.ui_text.menu_button_label.clone(),
            Style::default().fg(snapshot.theme.muted),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(shell)
            .style(Style::default().bg(snapshot.theme.base))
            .alignment(Alignment::Left),
        chunks[0],
    );

    let transcript_text = Text::from(transcript::lines(
        &snapshot.transcript,
        snapshot.transcript_animation,
        &snapshot.pending_transcript_animation_entries,
        &snapshot.ui_text,
        &snapshot.theme,
    ));
    let transcript = Paragraph::new(transcript_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(
                    if snapshot.pane_focus == PaneFocus::Transcript {
                        snapshot.theme.foam
                    } else {
                        snapshot.theme.highlight_high
                    },
                ))
                .style(Style::default().bg(snapshot.theme.surface)),
        )
        .style(
            Style::default()
                .fg(snapshot.theme.text)
                .bg(snapshot.theme.surface),
        )
        .wrap(Wrap { trim: false })
        .scroll((snapshot.transcript_scroll, 0));
    frame.render_widget(transcript, chunks[1]);
    let transcript_lines = transcript::content_lines(&snapshot.transcript, chunks[1].width);
    let mut scrollbar_state =
        ScrollbarState::new(transcript_lines).position(snapshot.transcript_scroll as usize);
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .thumb_style(Style::default().fg(snapshot.theme.foam))
        .track_style(Style::default().fg(snapshot.theme.highlight_high))
        .begin_symbol(None)
        .end_symbol(None);
    frame.render_stateful_widget(
        scrollbar,
        chunks[1].inner(Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut scrollbar_state,
    );

    let input_title = if snapshot.game_over {
        Some(snapshot.ui_text.session_ended_title.as_str())
    } else {
        None
    };
    let input_title_style = if snapshot.game_over {
        Style::default()
            .fg(snapshot.theme.muted)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(snapshot.theme.rose)
            .add_modifier(Modifier::BOLD)
    };
    let input = Paragraph::new(Line::from(vec![
        Span::styled(
            if snapshot.game_over {
                snapshot.ui_text.game_over_hint.as_str()
            } else {
                "> "
            },
            Style::default()
                .fg(if snapshot.game_over {
                    snapshot.theme.muted
                } else {
                    snapshot.theme.pine
                })
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            if snapshot.game_over {
                ""
            } else {
                snapshot.input.as_str()
            },
            Style::default().fg(snapshot.theme.text),
        ),
    ]))
    .block({
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(
                Style::default().fg(if snapshot.pane_focus == PaneFocus::Command {
                    snapshot.theme.rose
                } else {
                    snapshot.theme.highlight_high
                }),
            )
            .style(Style::default().bg(snapshot.theme.overlay));
        if let Some(title) = input_title {
            block.title(title).title_style(input_title_style)
        } else {
            block
        }
    })
    .wrap(Wrap { trim: false })
    .style(Style::default().bg(snapshot.theme.overlay));
    frame.render_widget(input, chunks[2]);

    if let Some(playback) = projector_playback {
        projector::render_modal(frame, playback, &snapshot.ui_text, &snapshot.theme, frame_interval);
    } else if let Some(shell_modal) = &snapshot.shell_modal {
        render_shell_modal(frame, shell_modal, &snapshot.ui_text, &snapshot.theme);
    } else if let Some(menu) = &snapshot.menu {
        render_menu(frame, menu, &snapshot.ui_text, &snapshot.theme);
    }

    if !snapshot.game_over
        && snapshot.menu.is_none()
        && snapshot.shell_modal.is_none()
        && snapshot.pane_focus == PaneFocus::Command
        && snapshot.transcript_animation.is_none()
        && snapshot.pending_transcript_animation_entries.is_empty()
    {
        let inner_width = chunks[2].width.saturating_sub(2);
        let prompt_width: u16 = 2;
        let first_line_width = inner_width.saturating_sub(prompt_width);
        let input_len = snapshot.input.chars().count() as u16;
        if input_len <= first_line_width {
            let cursor_offset = prompt_width + input_len;
            frame.set_cursor_position(Position::new(
                chunks[2].x + 1 + cursor_offset,
                chunks[2].y + 1,
            ));
        } else {
            let remaining = input_len - first_line_width;
            let other_line_width = inner_width.max(1);
            let col = remaining % other_line_width;
            let row = (remaining - 1) / other_line_width + 1;
            let max_row = chunks[2].height.saturating_sub(2);
            let row = row.min(max_row);
            frame.set_cursor_position(Position::new(chunks[2].x + 1 + col, chunks[2].y + 1 + row));
        }
    }
}

fn word_wrap(text: &str, max_width: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for word in text.split(' ') {
        let can_append = lines
            .last()
            .is_some_and(|last: &Line| last.width() + 1 + word.len() <= max_width);
        if can_append {
            let last = lines.last_mut().unwrap();
            last.push_span(Span::raw(format!(" {word}")));
        } else {
            lines.push(Line::from(word.to_string()));
        }
    }
    if lines.is_empty() {
        lines.push(Line::from(""));
    }
    lines
}

 fn render_menu(frame: &mut Frame, menu: &MenuSnapshot, ui_text: &UiTextDefinition, theme: &Theme) {
    let inner_width = (frame.area().width * 70 / 100).saturating_sub(6) as usize;
    let wrapped_lines: Vec<Vec<Line>> = menu
        .options
        .iter()
        .map(|option| {
            let mut lines = Vec::new();
            lines.push(Line::from(Span::styled(
                &option.title,
                Style::default()
                    .fg(theme.iris)
                    .add_modifier(Modifier::BOLD),
            )));
            for wrapped_line in word_wrap(&option.menu_text, inner_width) {
                lines.push(wrapped_line);
            }
            lines
        })
        .collect();
    let total_lines: u16 = wrapped_lines.iter().map(|lines| lines.len() as u16).sum();
    let height = (total_lines + 4).min(24);
    let area = centered_rect(70, height, frame.area());
    let sections = modal_block(frame, area, &ui_text.menu_option_list_title, theme);
    let items = wrapped_lines
        .into_iter()
        .map(ListItem::new)
        .collect::<Vec<_>>();
    let list = List::new(items)
        .style(
            Style::default()
                .fg(theme.text)
                .bg(theme.overlay),
        )
        .highlight_style(
            Style::default()
                .fg(theme.base)
                .bg(theme.foam)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("› ");
    let mut state = ListState::default();
    state.select(Some(menu.selected_index));
    frame.render_stateful_widget(list, sections[0], &mut state);
    frame.render_widget(
        Paragraph::new(ui_text.menu_choice_hint.clone())
            .alignment(Alignment::Right)
            .style(
                Style::default()
                    .fg(theme.muted)
                    .bg(theme.overlay),
            ),
        sections[1],
    );
}

fn render_shell_modal(frame: &mut Frame, modal: &ShellModalSnapshot, ui_text: &UiTextDefinition, theme: &Theme) {
    let area = match modal {
        ShellModalSnapshot::Detail { title, body, .. } => {
            detail_modal_area(frame.area(), &detail_modal_body_text(title, body))
        }
        ShellModalSnapshot::YelpReview { .. } => centered_rect(62, 14, frame.area()),
        _ => centered_rect(62, 12, frame.area()),
    };
    let block_title = match modal {
        ShellModalSnapshot::YelpReview { .. } => "Session Complete",
        _ => &ui_text.shell_menu_title,
    };
    let sections = modal_block(frame, area, block_title, theme);
    match modal {
        ShellModalSnapshot::Root {
            selected_index,
            options,
        } => {
            let items = options
                .iter()
                .enumerate()
                .map(|(index, option)| {
                    ListItem::new(Line::from(format!("{}. {}", index + 1, option)))
                })
                .collect::<Vec<_>>();
            let list = List::new(items)
                .style(
                    Style::default()
                        .fg(theme.text)
                        .bg(theme.overlay),
                )
                .highlight_style(
                    Style::default()
                        .fg(theme.base)
                        .bg(theme.foam)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("› ");
            let mut state = ListState::default();
            state.select(Some(*selected_index));
            frame.render_stateful_widget(list, sections[0], &mut state);
            frame.render_widget(
                Paragraph::new(ui_text.shell_menu_close_hint.clone())
                    .alignment(Alignment::Right)
                    .style(
                        Style::default()
                            .fg(theme.muted)
                            .bg(theme.overlay),
                    ),
                sections[1],
            );
        }
        ShellModalSnapshot::Selection {
            title,
            selected_index,
            options,
            hint,
        } => {
            let items = options
                .iter()
                .enumerate()
                .map(|(index, option)| {
                    ListItem::new(Line::from(format!("{}. {}", index + 1, option)))
                })
                .collect::<Vec<_>>();
            frame.render_widget(
                Paragraph::new(title.clone()).style(
                    Style::default()
                        .fg(theme.iris)
                        .bg(theme.overlay)
                        .add_modifier(Modifier::BOLD),
                ),
                sections[0].inner(Margin {
                    vertical: 0,
                    horizontal: 0,
                }),
            );
            let list_area = sections[0].inner(Margin {
                vertical: 1,
                horizontal: 0,
            });
            let list = List::new(items)
                .style(
                    Style::default()
                        .fg(theme.text)
                        .bg(theme.overlay),
                )
                .highlight_style(
                    Style::default()
                        .fg(theme.base)
                        .bg(theme.foam)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("› ");
            let mut state = ListState::default();
            state.select(Some(*selected_index));
            frame.render_stateful_widget(list, list_area, &mut state);
            frame.render_widget(
                Paragraph::new(hint.clone())
                    .alignment(Alignment::Right)
                    .style(
                        Style::default()
                            .fg(theme.muted)
                            .bg(theme.overlay),
                    ),
                sections[1],
            );
        }
        ShellModalSnapshot::YelpReview {
            rating,
            review_text,
            hint,
        } => {
            let inner =
                Layout::vertical([Constraint::Length(2), Constraint::Fill(1)]).split(sections[0]);
            let top = inner[0];
            let review_area = inner[1];

            frame.render_widget(
                Paragraph::new(format!("  {}", rating_stars(*rating)))
                    .style(Style::default().fg(theme.gold)),
                top,
            );

            frame.render_widget(
                Paragraph::new(review_text.clone())
                    .wrap(Wrap { trim: false })
                    .style(Style::default().fg(theme.text)),
                review_area,
            );

            frame.render_widget(
                Paragraph::new(hint.clone())
                    .alignment(Alignment::Right)
                    .style(Style::default().fg(theme.muted).bg(theme.overlay)),
                sections[1],
            );
        }
        ShellModalSnapshot::Detail {
            title,
            body,
            hint,
            scroll,
        } => {
            let body_text = detail_modal_body_text(title, body);
            frame.render_widget(
                Paragraph::new(body_text.clone())
                    .wrap(Wrap { trim: false })
                    .scroll((*scroll, 0))
                    .style(
                        Style::default()
                            .fg(theme.text)
                            .bg(theme.overlay),
                    ),
                sections[0],
            );
            let mut scrollbar_state =
                ScrollbarState::new(detail_modal_content_lines(&body_text, sections[0].width))
                    .position(*scroll as usize);
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .thumb_style(Style::default().fg(theme.foam))
                .track_style(Style::default().fg(theme.highlight_high))
                .begin_symbol(None)
                .end_symbol(None);
            frame.render_stateful_widget(
                scrollbar,
                sections[0].inner(Margin {
                    vertical: 0,
                    horizontal: 0,
                }),
                &mut scrollbar_state,
            );
            frame.render_widget(
                Paragraph::new(hint.clone())
                    .alignment(Alignment::Right)
                    .style(
                        Style::default()
                            .fg(theme.muted)
                            .bg(theme.overlay),
                    ),
                sections[1],
            );
        }
    }
}

pub(crate) fn detail_modal_max_scroll(frame_area: Rect, title: &str, body: &str) -> u16 {
    let body_text = detail_modal_body_text(title, body);
    let area = detail_modal_area(frame_area, &body_text);
    let sections = Layout::vertical([Constraint::Min(3), Constraint::Length(1)])
        .margin(1)
        .split(area);
    let visible = sections[0].height.max(1) as usize;
    detail_modal_content_lines(&body_text, sections[0].width).saturating_sub(visible) as u16
}

pub(crate) fn detail_modal_page_size(frame_area: Rect, title: &str, body: &str) -> u16 {
    let body_text = detail_modal_body_text(title, body);
    let area = detail_modal_area(frame_area, &body_text);
    let sections = Layout::vertical([Constraint::Min(3), Constraint::Length(1)])
        .margin(1)
        .split(area);
    sections[0].height.max(1)
}

fn detail_modal_body_text(title: &str, body: &str) -> String {
    format!("{title}\n\n{body}")
}

fn detail_modal_area(frame_area: Rect, body_text: &str) -> Rect {
    centered_rect(
        SHELL_MODAL_WIDTH_PERCENT,
        detail_modal_height(frame_area, body_text),
        frame_area,
    )
}

fn detail_modal_height(frame_area: Rect, body_text: &str) -> u16 {
    let max_height = frame_area
        .height
        .saturating_sub(4)
        .clamp(SHELL_MODAL_MIN_HEIGHT, SHELL_MODAL_MAX_HEIGHT);
    let content_width = frame_area
        .width
        .saturating_mul(SHELL_MODAL_WIDTH_PERCENT)
        .saturating_div(100)
        .saturating_sub(2);
    let desired = detail_modal_content_lines(body_text, content_width).saturating_add(3) as u16;
    desired.clamp(SHELL_MODAL_MIN_HEIGHT, max_height)
}

fn detail_modal_content_lines(text: &str, width: u16) -> usize {
    let width = width.max(1) as usize;
    text.lines()
        .map(|line| {
            let count = line.chars().count();
            if count == 0 { 1 } else { count.div_ceil(width) }
        })
        .sum()
}

fn centered_rect(width_percent: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .split(area);
    let horizontal = Layout::horizontal([Constraint::Percentage(width_percent)])
        .flex(Flex::Center)
        .split(vertical[0]);
    horizontal[0]
}

fn rating_stars(rating: u32) -> String {
    let filled = "★".repeat(rating.min(5) as usize);
    let empty = "☆".repeat((5 - rating.min(5)) as usize);
    filled + &empty
}

fn modal_block(frame: &mut Frame, area: Rect, title: &str, theme: &Theme) -> [Rect; 2] {
    frame.render_widget(Clear, area);
    let block = Block::default()
        .title(title)
        .title_style(
            Style::default()
                .fg(theme.iris)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.iris))
        .style(Style::default().bg(theme.overlay));
    frame.render_widget(block, area);
    let sections = Layout::vertical([Constraint::Min(1), Constraint::Length(1)])
        .margin(1)
        .split(area);
    [sections[0], sections[1]]
}
