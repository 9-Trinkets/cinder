use crate::render::PaneFocus;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub(crate) struct InputContext {
    pub game_over: bool,
    pub pending_turn: bool,
    pub projector_playing: bool,
    pub shell_modal: Option<ShellModalKind>,
    pub visible_menu_len: Option<usize>,
    pub pane_focus: PaneFocus,
    pub command_input_locked: bool,
    pub input_is_empty: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShellModalKind {
    Root,
    Language,
    Rooms,
    Follow,
    Other,
}

pub(crate) enum InputAction {
    Quit,
    NoOp,
    FocusNextPane,
    CloseShellMenu,
    FinishOrCloseProjector,
    SubmitShellMenuChoice,
    SubmitMenuChoice,
    SubmitInput,
    BackShellMenu,
    DeleteInputChar,
    StepShellMenu(isize),
    StepMenu(isize, usize),
    StepTranscriptScroll(isize),
    StepTranscriptPage(isize),
    ScrollTranscriptStart,
    ScrollTranscriptEnd,
    StepHistory(isize),
    OpenShellMenu,
    SelectShellModalDigit(usize),
    SelectMenuDigit(usize),
    AppendInput(char),
}

pub(crate) fn resolve(context: InputContext, key: KeyEvent) -> InputAction {
    if context.game_over
        && !context.pending_turn
        && context.shell_modal.is_none()
        && !context.projector_playing
    {
        return resolve_game_over(key);
    }

    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => InputAction::Quit,
        KeyCode::Tab
            if context.shell_modal.is_none()
                && context.visible_menu_len.is_none()
                && !context.projector_playing =>
        {
            InputAction::FocusNextPane
        }
        KeyCode::Esc => {
            if context.shell_modal.is_some() {
                InputAction::CloseShellMenu
            } else if !context.pending_turn {
                InputAction::OpenShellMenu
            } else {
                InputAction::NoOp
            }
        }
        KeyCode::Enter if !context.pending_turn => {
            if context.projector_playing {
                InputAction::FinishOrCloseProjector
            } else if context.shell_modal.is_some() {
                InputAction::SubmitShellMenuChoice
            } else if context.visible_menu_len.is_some() {
                InputAction::SubmitMenuChoice
            } else if context.pane_focus == PaneFocus::Transcript || context.command_input_locked {
                InputAction::NoOp
            } else {
                InputAction::SubmitInput
            }
        }
        KeyCode::Backspace => {
            if context.projector_playing {
                InputAction::CloseShellMenu
            } else if context.shell_modal.is_some() {
                InputAction::BackShellMenu
            } else if context.visible_menu_len.is_none()
                && context.pane_focus == PaneFocus::Command
                && !context.command_input_locked
            {
                InputAction::DeleteInputChar
            } else {
                InputAction::NoOp
            }
        }
        KeyCode::Up => {
            if context.projector_playing {
                InputAction::NoOp
            } else if matches!(
                context.shell_modal,
                Some(
                    ShellModalKind::Root
                        | ShellModalKind::Language
                        | ShellModalKind::Rooms
                        | ShellModalKind::Follow
                )
            ) {
                InputAction::StepShellMenu(-1)
            } else if matches!(context.shell_modal, Some(ShellModalKind::Other)) {
                InputAction::StepTranscriptScroll(-1)
            } else if let Some(len) = context.visible_menu_len {
                InputAction::StepMenu(-1, len)
            } else if context.pane_focus == PaneFocus::Transcript {
                InputAction::StepTranscriptScroll(-1)
            } else {
                InputAction::StepHistory(-1)
            }
        }
        KeyCode::Down => {
            if context.projector_playing {
                InputAction::NoOp
            } else if matches!(
                context.shell_modal,
                Some(
                    ShellModalKind::Root
                        | ShellModalKind::Language
                        | ShellModalKind::Rooms
                        | ShellModalKind::Follow
                )
            ) {
                InputAction::StepShellMenu(1)
            } else if matches!(context.shell_modal, Some(ShellModalKind::Other)) {
                InputAction::StepTranscriptScroll(1)
            } else if let Some(len) = context.visible_menu_len {
                InputAction::StepMenu(1, len)
            } else if context.pane_focus == PaneFocus::Transcript {
                InputAction::StepTranscriptScroll(1)
            } else {
                InputAction::StepHistory(1)
            }
        }
        KeyCode::PageUp => {
            if matches!(context.shell_modal, Some(ShellModalKind::Other))
                || (context.shell_modal.is_none() && context.visible_menu_len.is_none())
            {
                InputAction::StepTranscriptPage(-1)
            } else {
                InputAction::NoOp
            }
        }
        KeyCode::PageDown => {
            if matches!(context.shell_modal, Some(ShellModalKind::Other))
                || (context.shell_modal.is_none() && context.visible_menu_len.is_none())
            {
                InputAction::StepTranscriptPage(1)
            } else {
                InputAction::NoOp
            }
        }
        KeyCode::Home => {
            if matches!(context.shell_modal, Some(ShellModalKind::Other))
                || (context.shell_modal.is_none() && context.visible_menu_len.is_none())
            {
                InputAction::ScrollTranscriptStart
            } else {
                InputAction::NoOp
            }
        }
        KeyCode::End => {
            if matches!(context.shell_modal, Some(ShellModalKind::Other))
                || (context.shell_modal.is_none() && context.visible_menu_len.is_none())
            {
                InputAction::ScrollTranscriptEnd
            } else {
                InputAction::NoOp
            }
        }
        KeyCode::Char(ch) => resolve_char(context, ch),
        _ => InputAction::NoOp,
    }
}

fn resolve_game_over(key: KeyEvent) -> InputAction {
    match key.code {
        KeyCode::Up => InputAction::StepTranscriptScroll(-1),
        KeyCode::Down => InputAction::StepTranscriptScroll(1),
        KeyCode::PageUp => InputAction::StepTranscriptPage(-1),
        KeyCode::PageDown => InputAction::StepTranscriptPage(1),
        KeyCode::Home => InputAction::ScrollTranscriptStart,
        KeyCode::End => InputAction::ScrollTranscriptEnd,
        KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter => InputAction::Quit,
        _ => InputAction::NoOp,
    }
}

fn resolve_char(context: InputContext, ch: char) -> InputAction {
    if ch == '?'
        && !context.pending_turn
        && context.input_is_empty
        && context.shell_modal.is_none()
        && context.visible_menu_len.is_none()
        && context.pane_focus == PaneFocus::Command
        && !context.command_input_locked
    {
        return InputAction::OpenShellMenu;
    }

    if context.projector_playing {
        return InputAction::NoOp;
    }

    if let Some(shell_modal) = context.shell_modal {
        if matches!(
            shell_modal,
            ShellModalKind::Root
                | ShellModalKind::Language
                | ShellModalKind::Rooms
                | ShellModalKind::Follow
        ) && let Some(index) = digit_index(ch)
        {
            return InputAction::SelectShellModalDigit(index);
        }
        return InputAction::NoOp;
    }

    if let Some(len) = context.visible_menu_len {
        if let Some(index) = digit_index(ch).filter(|index| *index < len) {
            return InputAction::SelectMenuDigit(index);
        }
        return InputAction::NoOp;
    }

    if context.pane_focus == PaneFocus::Command && !context.command_input_locked {
        InputAction::AppendInput(ch)
    } else {
        InputAction::NoOp
    }
}

fn digit_index(ch: char) -> Option<usize> {
    ch.to_digit(10)
        .and_then(|digit| digit.checked_sub(1))
        .map(|digit| digit as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_opens_shell_menu_when_not_already_in_modal() {
        let action = resolve(
            InputContext {
                game_over: false,
                pending_turn: false,
                projector_playing: false,
                shell_modal: None,
                visible_menu_len: None,
                pane_focus: PaneFocus::Command,
                command_input_locked: false,
                input_is_empty: true,
            },
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
        );
        assert!(matches!(action, InputAction::OpenShellMenu));
    }

    #[test]
    fn game_over_modal_can_still_close_before_quit_controls_take_over() {
        let action = resolve(
            InputContext {
                game_over: true,
                pending_turn: false,
                projector_playing: false,
                shell_modal: Some(ShellModalKind::Other),
                visible_menu_len: None,
                pane_focus: PaneFocus::Command,
                command_input_locked: false,
                input_is_empty: true,
            },
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
        );
        assert!(matches!(action, InputAction::CloseShellMenu));
    }

    #[test]
    fn detail_modal_uses_arrow_keys_for_scroll() {
        let action = resolve(
            InputContext {
                game_over: false,
                pending_turn: false,
                projector_playing: false,
                shell_modal: Some(ShellModalKind::Other),
                visible_menu_len: None,
                pane_focus: PaneFocus::Command,
                command_input_locked: false,
                input_is_empty: true,
            },
            KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
        );
        assert!(matches!(action, InputAction::StepTranscriptScroll(1)));
    }
}
