use crate::effects::{TimedTextPlayback, TranscriptAnimationSnapshot, TranscriptTypewriter};
use crate::input::{self, InputAction, InputContext, ShellModalKind};
use crate::projector;
use crate::render::{self, MenuSnapshot, PaneFocus, RenderSnapshot, ShellModalSnapshot};
use crate::theme::Theme;
use crate::transcript;
use cinder_core::content::loader::{
    LocaleOption, available_locales, load_pack_from_dir_with_locale, pack_dir,
};
use cinder_core::content::types::{ShellMenuItem, UiTextDefinition};
use cinder_core::{CinderRuntime, MenuChoiceOption, SessionFeedback, TurnOutcome};
use crossterm::cursor::SetCursorStyle;
use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout, Rect};
use std::collections::VecDeque;
use std::error::Error;
use std::io::{self, Stdout};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::{Duration, Instant};

const UI_FRAME_INTERVAL: Duration = Duration::from_millis(50);
const PROJECTOR_COALESCE_MS: u32 = 1500;
pub fn run(runtime: CinderRuntime) -> Result<(), Box<dyn Error>> {
    let mut ui = TuiApp::new(runtime)?;
    ui.run()
}

struct TuiApp {
    runtime: CinderRuntime,
    transcript: Vec<String>,
    input: String,
    history: Vec<String>,
    history_index: Option<usize>,
    game_over: bool,
    pending_turn: Option<Receiver<Result<TurnOutcome, String>>>,
    tick_updates: Receiver<Result<TurnOutcome, String>>,
    tick_paused: Arc<AtomicBool>,
    transcript_scroll: u16,
    ui_text: UiTextDefinition,
    content_root: PathBuf,
    current_locale: String,
    available_locales: Vec<LocaleOption>,
    pane_focus: PaneFocus,
    menu_index: usize,
    shell_menu_index: usize,
    language_menu_index: usize,
    room_menu_index: usize,
    follow_menu_index: usize,
    last_seen_day: u32,
    chapter_start_transcript_index: usize,
    day_start_transcript_index: usize,
    pending_day_summary_days: VecDeque<u32>,
    pending_final_summary: bool,
    shell_modal_scroll: u16,
    shell_modal: Option<ShellModalState>,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    transcript_typewriter: TranscriptTypewriter,
}

enum ShellModalState {
    Root,
    Submenu { items: Vec<ShellMenuItem> },
    Help,
    ThingsToDo,
    Language,
    Rooms,
    Follow,
    About,
    ExitConfirm,
    DaySummary { day_number: u32, body: String },
    SessionFeedback { data: SessionFeedback },
    Projector(TimedTextPlayback),
}

impl TuiApp {
    fn new(runtime: CinderRuntime) -> Result<Self, Box<dyn Error>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, SetCursorStyle::SteadyBar)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        let content_root = pack_dir(&runtime.content().opening.id);
        let available_locales = available_locales(&content_root)?;
        let initial_title = format!("== {} ==", runtime.content().opening.title);
        let initial_intro = runtime.content().opening.intro_text.clone();
        let initial_room = runtime.run_turn("look")?.text;
        let ui_text = runtime.content().ui_text.clone();
        let current_locale = runtime.content().locale.clone();
        let last_seen_day = runtime.current_day_number()?;
        let transcript_char_ms = runtime.content().settings.typewriter_char_ms;
        let (tick_rx, tick_paused) = Self::spawn_tick_loop(runtime.clone());

        let mut app = Self {
            runtime,
            transcript: Vec::new(),
            input: String::new(),
            history: Vec::new(),
            history_index: None,
            game_over: false,
            pending_turn: None,
            tick_updates: tick_rx,
            tick_paused,
            transcript_scroll: 0,
            ui_text,
            content_root,
            current_locale,
            available_locales,
            pane_focus: PaneFocus::Command,
            menu_index: 0,
            shell_menu_index: 0,
            language_menu_index: 0,
            room_menu_index: 0,
            follow_menu_index: 0,
            last_seen_day,
            chapter_start_transcript_index: 0,
            day_start_transcript_index: 0,
            pending_day_summary_days: VecDeque::new(),
            pending_final_summary: false,
            shell_modal_scroll: 0,
            shell_modal: None,
            terminal,
            transcript_typewriter: TranscriptTypewriter::new(transcript_char_ms),
        };
        app.push_transcript_entry(initial_title, false);
        app.push_transcript_entry(initial_intro, true);
        app.push_transcript_entry(initial_room, true);
        app.chapter_start_transcript_index = app.transcript.len();
        app.day_start_transcript_index = app.transcript.len();
        Ok(app)
    }

    fn spawn_tick_loop(
        runtime: CinderRuntime,
    ) -> (Receiver<Result<TurnOutcome, String>>, Arc<AtomicBool>) {
        let (tick_tx, tick_rx) = mpsc::channel();
        let tick_paused = Arc::new(AtomicBool::new(false));
        let tick_pause_flag = Arc::clone(&tick_paused);
        let tick_interval = Duration::from_millis(runtime.content().settings.npc_tick_interval_ms);
        thread::spawn(move || {
            loop {
                thread::sleep(tick_interval);
                if tick_pause_flag.load(Ordering::Relaxed) {
                    continue;
                }
                let result = runtime.run_tick().map_err(|error| error.to_string());
                if tick_tx.send(result).is_err() {
                    break;
                }
            }
        });
        (tick_rx, tick_paused)
    }

    fn sync_tick_pause(&self) {
        self.tick_paused.store(
            self.pending_turn.is_some() || self.shell_modal.is_some(),
            Ordering::Relaxed,
        );
    }

    fn format_day_summary_title(&self, day_number: u32) -> String {
        let day_number = day_number.to_string();
        self.runtime.content().render_template(
            &self.ui_text.day_summary_title,
            &[("day_number", day_number.as_str())],
        )
    }

    fn queue_day_summaries(&mut self) -> Result<(), Box<dyn Error>> {
        if !self.runtime.content().settings.show_day_summary {
            self.pending_day_summary_days.clear();
            return Ok(());
        }
        let current_day = self.runtime.current_day_number()?;
        if current_day <= self.last_seen_day {
            return Ok(());
        }
        for completed_day in self.last_seen_day..current_day {
            self.pending_day_summary_days.push_back(completed_day);
        }
        self.last_seen_day = current_day;
        Ok(())
    }

    fn maybe_open_queued_day_summary(&mut self) -> Result<(), Box<dyn Error>> {
        if self.shell_modal.is_some() {
            self.sync_tick_pause();
            return Ok(());
        }
        let Some(day_number) = self.pending_day_summary_days.pop_front() else {
            self.sync_tick_pause();
            return Ok(());
        };
        let body = self.build_day_summary_body()?;
        self.day_start_transcript_index = self.transcript.len();
        self.set_shell_modal(Some(ShellModalState::DaySummary { day_number, body }));
        self.sync_tick_pause();
        Ok(())
    }

    fn maybe_open_pending_summary_modal(&mut self) -> Result<(), Box<dyn Error>> {
        if self.pending_final_summary {
            return self.maybe_open_final_summary();
        }
        self.maybe_open_queued_day_summary()
    }

    fn maybe_open_final_summary(&mut self) -> Result<(), Box<dyn Error>> {
        if self.shell_modal.is_some() {
            self.sync_tick_pause();
            return Ok(());
        }
        if !self.pending_final_summary {
            self.sync_tick_pause();
            return Ok(());
        }
        let data = self.runtime.session_feedback()?.unwrap_or(SessionFeedback {
            rating: 0,
            review_text: "Session ended.".to_string(),
        });
        self.pending_final_summary = false;
        self.set_shell_modal(Some(ShellModalState::SessionFeedback { data }));
        self.sync_tick_pause();
        Ok(())
    }

    fn build_day_summary_body(&self) -> Result<String, Box<dyn Error>> {
        let focus_lines = self.runtime.current_objective_summaries()?;
        let focus_lines: Vec<String> = focus_lines.into_iter().map(|(s, _)| s).collect();
        let highlight_lines =
            summarize_day_highlights(&self.transcript, self.day_start_transcript_index);
        let relationship_lines = self.runtime.relationship_status_lines()?;
        let relationship_lines = if relationship_lines.is_empty() {
            vec![self.ui_text.day_summary_empty_relationships.clone()]
        } else {
            relationship_lines.into_iter().take(6).collect()
        };
        Ok(format!(
            "{}\n{}\n\n{}\n{}\n\n{}\n{}",
            self.ui_text.day_summary_current_focus_label,
            bullet_join(if focus_lines.is_empty() {
                vec![self.ui_text.things_to_do_empty.clone()]
            } else {
                focus_lines
            }),
            self.ui_text.day_summary_highlights_label,
            bullet_join(if highlight_lines.is_empty() {
                vec![self.ui_text.day_summary_empty_highlights.clone()]
            } else {
                highlight_lines
            }),
            self.ui_text.day_summary_relationships_label,
            bullet_join(relationship_lines)
        ))
    }

    fn run(&mut self) -> Result<(), Box<dyn Error>> {
        loop {
            self.poll_pending_turn();
            self.poll_tick_updates();
            self.advance_projector_sequence();
            self.advance_transcript_animation();
            let snapshot = self.render_snapshot();
            let projector_playback = match self.shell_modal.as_mut() {
                Some(ShellModalState::Projector(playback)) => Some(playback),
                _ => None,
            };
            let terminal = &mut self.terminal;
            terminal.draw(|frame| {
                render::draw(frame, &snapshot, projector_playback, UI_FRAME_INTERVAL)
            })?;

            if !event::poll(UI_FRAME_INTERVAL)? {
                continue;
            }

            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                let action = input::resolve(self.input_context()?, key);
                if self.apply_input_action(action)? {
                    break;
                }
            }
        }
        Ok(())
    }

    fn submit_input(&mut self) -> Result<(), Box<dyn Error>> {
        if self.command_input_locked() {
            return Ok(());
        }
        let command = self.input.trim().to_string();
        if command.is_empty() {
            return Ok(());
        }
        if self.runtime.content().settings.channel_surfing_only
            && command.eq_ignore_ascii_case("move")
        {
            self.input.clear();
            self.history.push(command);
            self.history_index = None;
            return self.open_room_menu();
        }
        if self.runtime.content().settings.channel_surfing_only
            && command.eq_ignore_ascii_case("follow")
        {
            self.input.clear();
            self.history.push(command);
            self.history_index = None;
            return self.open_follow_menu();
        }
        self.submit_command(command, None)
    }

    fn submit_menu_choice(&mut self) -> Result<(), Box<dyn Error>> {
        let Some(options) = self.menu_options()? else {
            return Ok(());
        };
        if options.is_empty() {
            return Ok(());
        }
        let selected_index = self.menu_index.min(options.len() - 1);
        let selected = &options[selected_index];
        self.submit_command(
            selected.command.clone(),
            selected.transcript_label.clone().or_else(|| {
                Some(
                    self.ui_text
                        .menu_choice_transcript
                        .replace("{title}", &selected.title),
                )
            }),
        )
    }

    fn open_shell_menu(&mut self) {
        self.shell_menu_index = 0;
        self.language_menu_index = self.current_language_index();
        self.room_menu_index = 0;
        self.follow_menu_index = 0;
        self.set_shell_modal(Some(ShellModalState::Root));
        self.sync_tick_pause();
    }

    fn close_shell_menu(&mut self) -> Result<(), Box<dyn Error>> {
        let was_projector = matches!(self.shell_modal, Some(ShellModalState::Projector(_)));
        self.set_shell_modal(None);
        if was_projector {
            self.flush_projector_narrative()?;
        }
        self.maybe_open_pending_summary_modal()?;
        Ok(())
    }

    fn back_shell_menu(&mut self) {
        self.set_shell_modal(match self.shell_modal {
            Some(
                ShellModalState::Help
                | ShellModalState::ThingsToDo
                | ShellModalState::Language
                | ShellModalState::Rooms
                | ShellModalState::Follow
                | ShellModalState::About,
            ) => Some(ShellModalState::Root),
            Some(ShellModalState::Submenu { .. }) => Some(ShellModalState::Root),
            Some(ShellModalState::ExitConfirm)
            | Some(ShellModalState::DaySummary { .. })
            | Some(ShellModalState::SessionFeedback { .. }) => None,
            Some(ShellModalState::Projector(_)) | Some(ShellModalState::Root) | None => None,
        });
        self.sync_tick_pause();
    }

    fn step_shell_menu(&mut self, delta: isize) {
        match self.shell_modal {
            Some(ShellModalState::Root) => {
                let len = self.shell_menu_options().len();
                let next = (self.shell_menu_index as isize + delta).clamp(0, len as isize - 1);
                self.shell_menu_index = next as usize;
            }
            Some(ShellModalState::Submenu { ref items, .. }) => {
                let len = items.len();
                let next = (self.shell_menu_index as isize + delta).clamp(0, len as isize - 1);
                self.shell_menu_index = next as usize;
            }
            Some(ShellModalState::Language) => {
                let len = self.available_locales.len();
                if len == 0 {
                    self.language_menu_index = 0;
                    return;
                }
                let next = (self.language_menu_index as isize + delta).clamp(0, len as isize - 1);
                self.language_menu_index = next as usize;
            }
            Some(ShellModalState::Rooms) => {
                let len = self
                    .room_switch_options()
                    .map(|options| options.len())
                    .unwrap_or(0);
                if len == 0 {
                    self.room_menu_index = 0;
                    return;
                }
                let next = (self.room_menu_index as isize + delta).clamp(0, len as isize - 1);
                self.room_menu_index = next as usize;
            }
            Some(ShellModalState::Follow) => {
                let len = self
                    .follow_actor_options()
                    .map(|options| options.len())
                    .unwrap_or(0);
                if len == 0 {
                    self.follow_menu_index = 0;
                    return;
                }
                let next = (self.follow_menu_index as isize + delta).clamp(0, len as isize - 1);
                self.follow_menu_index = next as usize;
            }
            _ => {}
        }
    }

    fn route_shell_menu_id(&mut self, id: &str) -> Option<ShellModalState> {
        match id {
            "help" => Some(ShellModalState::Help),
            "goals" => Some(ShellModalState::ThingsToDo),
            "language" => {
                self.language_menu_index = self.current_language_index();
                Some(ShellModalState::Language)
            }
            "about" => Some(ShellModalState::About),
            "exit" => Some(ShellModalState::ExitConfirm),
            _ => None,
        }
    }

    fn submit_shell_menu_choice(&mut self) -> Result<bool, Box<dyn Error>> {
        let next_modal = match self.shell_modal.take() {
            Some(ShellModalState::Root) => {
                let items = self.ui_text.shell_menu.items.clone();
                if items.is_empty() {
                    Some(match self.shell_menu_index {
                        0 => {
                            self.sync_tick_pause();
                            return Ok(false);
                        }
                        1 => ShellModalState::Help,
                        2 => ShellModalState::ThingsToDo,
                        3 => {
                            self.language_menu_index = self.current_language_index();
                            ShellModalState::Language
                        }
                        4 => ShellModalState::About,
                        _ => ShellModalState::ExitConfirm,
                    })
                } else if self.shell_menu_index < items.len() {
                    let item = &items[self.shell_menu_index];
                    if item.id == "resume" {
                        self.sync_tick_pause();
                        return Ok(false);
                    }
                    if item.children.is_empty() {
                        self.route_shell_menu_id(&item.id)
                    } else {
                        self.shell_menu_index = 0;
                        Some(ShellModalState::Submenu {
                            items: item.children.clone(),
                        })
                    }
                } else {
                    Some(ShellModalState::ExitConfirm)
                }
            }
            Some(ShellModalState::Submenu { items }) => {
                let index = self.shell_menu_index;
                if index < items.len() {
                    if items[index].id == "resume" {
                        self.sync_tick_pause();
                        return Ok(false);
                    }
                    self.route_shell_menu_id(&items[index].id)
                } else {
                    None
                }
            }
            Some(ShellModalState::Language) => {
                let locale = self
                    .available_locales
                    .get(self.language_menu_index)
                    .map(|option| option.code.clone());
                if let Some(locale) = locale {
                    self.apply_language(&locale)?;
                }
                self.shell_modal.take()
            }
            Some(ShellModalState::Rooms) => {
                if let Some(option) = self
                    .room_switch_options()?
                    .get(self.room_menu_index)
                    .cloned()
                {
                    self.set_shell_modal(None);
                    self.switch_room_view(option.command, option.title)?;
                }
                self.shell_modal.take()
            }
            Some(ShellModalState::Follow) => {
                if let Some(option) = self
                    .follow_actor_options()?
                    .get(self.follow_menu_index)
                    .cloned()
                {
                    self.set_shell_modal(None);
                    self.follow_actor(option.command, option.title)?;
                }
                self.shell_modal.take()
            }
            Some(
                ShellModalState::Help
                | ShellModalState::ThingsToDo
                | ShellModalState::About
                | ShellModalState::DaySummary { .. }
                | ShellModalState::SessionFeedback { .. }
                | ShellModalState::Projector(_),
            ) => None,
            Some(ShellModalState::ExitConfirm) => {
                self.sync_tick_pause();
                return Ok(true);
            }
            None => None,
        };
        self.set_shell_modal(next_modal);
        if self.shell_modal.is_none() {
            self.maybe_open_pending_summary_modal()?;
        } else {
            self.sync_tick_pause();
        }
        Ok(false)
    }

    fn projector_playing(&self) -> bool {
        matches!(self.shell_modal, Some(ShellModalState::Projector(_)))
    }

    fn select_shell_modal_digit(&mut self, index: usize) -> Result<(), Box<dyn Error>> {
        match self.shell_modal {
            Some(ShellModalState::Root) if index < self.shell_menu_options().len() => {
                self.shell_menu_index = index;
                self.submit_shell_menu_choice().map(|_| ())
            }
            Some(ShellModalState::Submenu { ref items, .. }) if index < items.len() => {
                self.shell_menu_index = index;
                self.submit_shell_menu_choice().map(|_| ())
            }
            Some(ShellModalState::Language) if index < self.available_locales.len() => {
                self.language_menu_index = index;
                self.submit_shell_menu_choice().map(|_| ())
            }
            Some(ShellModalState::Rooms) if index < self.room_switch_options()?.len() => {
                self.room_menu_index = index;
                self.submit_shell_menu_choice().map(|_| ())
            }
            Some(ShellModalState::Follow) if index < self.follow_actor_options()?.len() => {
                self.follow_menu_index = index;
                self.submit_shell_menu_choice().map(|_| ())
            }
            _ => Ok(()),
        }
    }

    fn current_language_index(&self) -> usize {
        self.available_locales
            .iter()
            .position(|locale| locale.code == self.current_locale)
            .unwrap_or(0)
    }

    fn apply_language(&mut self, locale: &str) -> Result<(), Box<dyn Error>> {
        if locale == self.current_locale {
            self.set_shell_modal(None);
            self.sync_tick_pause();
            return Ok(());
        }
        let pack = load_pack_from_dir_with_locale(&self.content_root, Some(locale))?;
        let language_name = pack.ui_text.language_name.clone();
        let runtime = self.runtime.with_content(pack);
        runtime.relocalize_story_vars()?;
        let ui_text = runtime.content().ui_text.clone();
        let language_changed = runtime.content().render_template(
            &ui_text.language_changed_text,
            &[("language_name", language_name.as_str())],
        );
        let (tick_updates, tick_paused) = Self::spawn_tick_loop(runtime.clone());
        self.runtime = runtime;
        self.tick_updates = tick_updates;
        self.tick_paused = tick_paused;
        self.current_locale = locale.to_string();
        self.ui_text = ui_text;
        self.transcript_typewriter
            .set_char_ms(self.runtime.content().settings.typewriter_char_ms);
        self.language_menu_index = self.current_language_index();
        self.room_menu_index = 0;
        self.follow_menu_index = 0;
        self.shell_menu_index = 0;
        self.set_shell_modal(None);
        self.sync_tick_pause();
        self.push_transcript(language_changed);
        Ok(())
    }

    fn maybe_open_projector_sequence(&mut self) {
        if let Ok(Some(sequence)) = self.runtime.consume_pending_projector_sequence() {
            let playback = projector::movie_playback(
                sequence,
                tachyonfx::fx::coalesce((PROJECTOR_COALESCE_MS, tachyonfx::Interpolation::QuintIn)),
            );
            self.set_shell_modal(Some(ShellModalState::Projector(playback)));
            self.sync_tick_pause();
        }
    }

    fn advance_projector_sequence(&mut self) {
        let Some(ShellModalState::Projector(playback)) = self.shell_modal.as_mut() else {
            return;
        };
        playback.advance(Instant::now());
    }

    fn finish_or_close_projector(&mut self) -> Result<(), Box<dyn Error>> {
        let Some(ShellModalState::Projector(playback)) = self.shell_modal.as_mut() else {
            return Ok(());
        };
        if playback.is_finished() {
            self.set_shell_modal(None);
            self.flush_projector_narrative()?;
            self.maybe_open_queued_day_summary()?;
        } else {
            playback.finish();
        }
        self.sync_tick_pause();
        Ok(())
    }

    fn submit_command(
        &mut self,
        command: String,
        transcript_label: Option<String>,
    ) -> Result<(), Box<dyn Error>> {
        self.push_transcript_entry(
            format!("> {}", transcript_label.unwrap_or_else(|| command.clone())),
            false,
        );
        self.history.push(command.clone());
        self.history_index = None;
        self.input.clear();
        let runtime = self.runtime.clone();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let result = runtime
                .run_turn(&command)
                .map_err(|error| error.to_string());
            let _ = tx.send(result);
        });
        self.pending_turn = Some(rx);
        self.sync_tick_pause();
        Ok(())
    }

    fn switch_room_view(
        &mut self,
        room_id: String,
        room_title: String,
    ) -> Result<(), Box<dyn Error>> {
        self.push_transcript_entry(
            format!(
                "> {}",
                self.ui_text
                    .room_switch_transcript
                    .replace("{title}", &room_title)
            ),
            false,
        );
        let outcome = self.runtime.switch_room_view(&room_id)?;
        self.apply_turn_outcome(outcome);
        Ok(())
    }

    fn follow_actor(
        &mut self,
        actor_id: String,
        _actor_name: String,
    ) -> Result<(), Box<dyn Error>> {
        let is_stop = actor_id == "none";
        let outcome = self
            .runtime
            .follow_actor((!is_stop).then_some(actor_id.as_str()))?;
        self.apply_turn_outcome(outcome);
        Ok(())
    }

    fn apply_turn_outcome(&mut self, outcome: TurnOutcome) {
        if !outcome.text.trim().is_empty() {
            self.push_transcript(outcome.text);
        }
        self.game_over = outcome.game_over;
        if outcome.game_over {
            self.pending_final_summary = true;
        }
        if let Some(options) = self.menu_options().ok().flatten() {
            self.menu_index = self.menu_index.min(options.len().saturating_sub(1));
        } else {
            self.menu_index = 0;
        }
        self.maybe_open_projector_sequence();
        if !outcome.game_over
            && let Err(error) = self.queue_day_summaries()
        {
            self.push_transcript(format!("{} {error}", self.ui_text.error_prefix));
        }
        if let Err(error) = self.maybe_open_pending_summary_modal() {
            self.push_transcript(format!("{} {error}", self.ui_text.error_prefix));
        }
    }

    fn step_menu(&mut self, delta: isize, len: usize) {
        if len == 0 {
            self.menu_index = 0;
            return;
        }
        let next = (self.menu_index as isize + delta).clamp(0, len as isize - 1);
        self.menu_index = next as usize;
    }

    fn step_history(&mut self, delta: isize) {
        if self.command_input_locked() {
            return;
        }
        if self.history.is_empty() {
            return;
        }
        let len = self.history.len() as isize;
        let next = match self.history_index {
            Some(index) => (index as isize + delta).clamp(0, len - 1),
            None if delta < 0 => len - 1,
            None => return,
        };
        self.history_index = Some(next as usize);
        self.input = self.history[next as usize].clone();
    }

    fn render_snapshot(&self) -> RenderSnapshot {
        let menu = self
            .visible_menu_options()
            .ok()
            .flatten()
            .map(|options| MenuSnapshot {
                selected_index: self.menu_index.min(options.len().saturating_sub(1)),
                options,
            });
        let shell_modal = self.shell_modal_snapshot();
        RenderSnapshot {
            title: self.runtime.content().opening.title.clone(),
            time: self
                .runtime
                .current_time_label()
                .unwrap_or_else(|_| "--:--".to_string()),
            transcript: self.transcript.clone(),
            transcript_scroll: self.transcript_scroll,
            transcript_animation: self.current_transcript_animation_snapshot(),
            pending_transcript_animation_entries: self.transcript_typewriter.pending_entries(),
            ui_text: self.ui_text.clone(),
            theme: Theme::from(&self.runtime.content().settings.theme),
            pane_focus: self.pane_focus,
            input: self.input.clone(),
            game_over: self.game_over,
            menu,
            shell_modal,
        }
    }

    fn current_transcript_animation_snapshot(&self) -> Option<TranscriptAnimationSnapshot> {
        self.transcript_typewriter.snapshot(&self.transcript)
    }

    fn command_input_locked(&self) -> bool {
        false
    }

    fn visible_menu_options(&self) -> Result<Option<Vec<MenuChoiceOption>>, Box<dyn Error>> {
        if self.command_input_locked() {
            Ok(None)
        } else {
            self.menu_options()
        }
    }

    fn transcript_area(&self) -> Rect {
        let size = self
            .terminal
            .size()
            .unwrap_or_else(|_| ratatui::layout::Size::new(120, 40));
        let area = Rect::new(0, 0, size.width, size.height);
        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .margin(1)
        .split(area);
        chunks[1]
    }

    fn transcript_page_size(&self) -> u16 {
        self.transcript_area().height.saturating_sub(3).max(1)
    }

    fn detail_modal_content(&self) -> Option<(String, String)> {
        match &self.shell_modal {
            Some(ShellModalState::Help) => {
                Some((self.ui_text.help_label.clone(), self.runtime.help_text()))
            }
            Some(ShellModalState::ThingsToDo) => {
                let summaries = self
                    .runtime
                    .current_objective_summaries()
                    .unwrap_or_default();
                let body = if summaries.is_empty() {
                    self.ui_text.things_to_do_empty.clone()
                } else {
                    summaries
                        .into_iter()
                        .map(|(_, message)| message)
                        .filter(|m| !m.is_empty())
                        .collect::<Vec<_>>()
                        .join("\n\n")
                };
                let body = if body.is_empty() {
                    self.ui_text.things_to_do_empty.clone()
                } else {
                    body
                };
                Some((self.ui_text.things_to_do_label.clone(), body))
            }
            Some(ShellModalState::About) => Some((
                self.ui_text.about_label.clone(),
                self.ui_text.about_body.clone(),
            )),
            Some(ShellModalState::ExitConfirm) => Some((
                self.ui_text.exit_confirm_title.clone(),
                self.ui_text.exit_confirm_body.clone(),
            )),
            Some(ShellModalState::DaySummary { day_number, body }) => {
                Some((self.format_day_summary_title(*day_number), body.clone()))
            }
            Some(ShellModalState::SessionFeedback { .. }) => None,
            _ => None,
        }
    }

    fn input_context(&self) -> Result<InputContext, Box<dyn Error>> {
        Ok(InputContext {
            game_over: self.game_over,
            pending_turn: self.pending_turn.is_some(),
            projector_playing: self.projector_playing(),
            shell_modal: self.shell_modal_kind(),
            visible_menu_len: self.visible_menu_options()?.map(|options| options.len()),
            pane_focus: self.pane_focus,
            command_input_locked: self.command_input_locked(),
            input_is_empty: self.input.is_empty(),
        })
    }

    fn shell_modal_kind(&self) -> Option<ShellModalKind> {
        match self.shell_modal {
            Some(ShellModalState::Root) => Some(ShellModalKind::Root),
            Some(ShellModalState::Language) => Some(ShellModalKind::Language),
            Some(ShellModalState::Rooms) => Some(ShellModalKind::Rooms),
            Some(ShellModalState::Follow) => Some(ShellModalKind::Follow),
            Some(_) => Some(ShellModalKind::Other),
            None => None,
        }
    }

    fn transcript_max_scroll(&self) -> u16 {
        let area = self.transcript_area();
        transcript::max_scroll_for_area(&self.transcript, area.width, area.height)
    }

    fn scroll_transcript_to_end(&mut self) {
        self.transcript_scroll = self.transcript_max_scroll();
    }

    fn step_transcript_scroll(&mut self, delta: isize) {
        if self.detail_modal_content().is_some() {
            self.step_shell_modal_scroll(delta);
            return;
        }
        let max_scroll = self.transcript_max_scroll() as isize;
        self.transcript_scroll =
            (self.transcript_scroll as isize + delta).clamp(0, max_scroll) as u16;
    }

    fn scroll_transcript_start(&mut self) {
        if self.detail_modal_content().is_some() {
            self.shell_modal_scroll = 0;
        } else {
            self.transcript_scroll = 0;
        }
    }

    fn scroll_transcript_end(&mut self) {
        if self.detail_modal_content().is_some() {
            self.shell_modal_scroll = self.shell_modal_max_scroll();
        } else {
            self.scroll_transcript_to_end();
        }
    }

    fn step_shell_modal_scroll(&mut self, delta: isize) {
        let max_scroll = self.shell_modal_max_scroll() as isize;
        self.shell_modal_scroll =
            (self.shell_modal_scroll as isize + delta).clamp(0, max_scroll) as u16;
    }

    fn shell_modal_max_scroll(&self) -> u16 {
        let Some((title, body)) = self.detail_modal_content() else {
            return 0;
        };
        render::detail_modal_max_scroll(self.frame_area(), &title, &body)
    }

    fn shell_modal_page_size(&self) -> u16 {
        let Some((title, body)) = self.detail_modal_content() else {
            return 1;
        };
        render::detail_modal_page_size(self.frame_area(), &title, &body)
    }

    fn frame_area(&self) -> Rect {
        let size = self
            .terminal
            .size()
            .unwrap_or_else(|_| ratatui::layout::Size::new(120, 40));
        Rect::new(0, 0, size.width, size.height)
    }

    fn set_shell_modal(&mut self, modal: Option<ShellModalState>) {
        self.shell_modal = modal;
        self.shell_modal_scroll = 0;
    }

    fn push_transcript(&mut self, entry: String) {
        self.push_transcript_entry(entry, true);
    }

    fn push_transcript_entry(&mut self, entry: String, animate: bool) {
        let follow = self.transcript_scroll >= self.transcript_max_scroll();
        self.transcript.push(entry);
        if animate {
            self.transcript_typewriter
                .enqueue(self.transcript.len() - 1);
        }
        if follow {
            self.scroll_transcript_to_end();
        } else {
            self.transcript_scroll = self.transcript_scroll.min(self.transcript_max_scroll());
        }
    }

    fn advance_transcript_animation(&mut self) {
        self.transcript_typewriter.advance(&self.transcript);
        if self.transcript_scroll >= self.transcript_max_scroll() {
            self.scroll_transcript_to_end();
        }
    }

    fn apply_input_action(&mut self, action: InputAction) -> Result<bool, Box<dyn Error>> {
        match action {
            InputAction::Quit => return Ok(true),
            InputAction::NoOp => {}
            InputAction::FocusNextPane => {
                self.pane_focus = match self.pane_focus {
                    PaneFocus::Command => PaneFocus::Transcript,
                    PaneFocus::Transcript => PaneFocus::Command,
                };
            }
            InputAction::CloseShellMenu => self.close_shell_menu()?,
            InputAction::FinishOrCloseProjector => self.finish_or_close_projector()?,
            InputAction::SubmitShellMenuChoice => {
                if self.submit_shell_menu_choice()? {
                    return Ok(true);
                }
            }
            InputAction::SubmitMenuChoice => self.submit_menu_choice()?,
            InputAction::SubmitInput => self.submit_input()?,
            InputAction::BackShellMenu => self.back_shell_menu(),
            InputAction::DeleteInputChar => {
                self.input.pop();
                self.history_index = None;
            }
            InputAction::StepShellMenu(delta) => self.step_shell_menu(delta),
            InputAction::StepMenu(delta, len) => self.step_menu(delta, len),
            InputAction::StepTranscriptScroll(delta) => self.step_transcript_scroll(delta),
            InputAction::StepTranscriptPage(delta) => {
                let page_size = if self.detail_modal_content().is_some() {
                    self.shell_modal_page_size()
                } else {
                    self.transcript_page_size()
                };
                self.step_transcript_scroll(delta * page_size as isize)
            }
            InputAction::ScrollTranscriptStart => self.scroll_transcript_start(),
            InputAction::ScrollTranscriptEnd => self.scroll_transcript_end(),
            InputAction::StepHistory(delta) => self.step_history(delta),
            InputAction::OpenShellMenu => self.open_shell_menu(),
            InputAction::SelectShellModalDigit(index) => self.select_shell_modal_digit(index)?,
            InputAction::SelectMenuDigit(index) => {
                self.menu_index = index;
                self.submit_menu_choice()?;
            }
            InputAction::AppendInput(ch) => {
                self.input.push(ch);
                self.history_index = None;
            }
        }
        Ok(false)
    }

    fn menu_options(&self) -> Result<Option<Vec<MenuChoiceOption>>, Box<dyn Error>> {
        self.runtime.menu_choice_options()
    }

    fn room_switch_options(&self) -> Result<Vec<MenuChoiceOption>, Box<dyn Error>> {
        self.runtime.room_switch_options()
    }

    fn follow_actor_options(&self) -> Result<Vec<MenuChoiceOption>, Box<dyn Error>> {
        self.runtime.follow_actor_options()
    }

    fn open_room_menu(&mut self) -> Result<(), Box<dyn Error>> {
        if self.room_switch_options()?.is_empty() {
            return Ok(());
        }
        self.room_menu_index = 0;
        self.set_shell_modal(Some(ShellModalState::Rooms));
        Ok(())
    }

    fn open_follow_menu(&mut self) -> Result<(), Box<dyn Error>> {
        if self.follow_actor_options()?.is_empty() {
            return Ok(());
        }
        self.follow_menu_index = 0;
        self.set_shell_modal(Some(ShellModalState::Follow));
        Ok(())
    }

    fn shell_menu_options(&self) -> Vec<String> {
        if self.ui_text.shell_menu.items.is_empty() {
            vec![
                self.ui_text.resume_label.clone(),
                self.ui_text.help_label.clone(),
                self.ui_text.things_to_do_label.clone(),
                self.ui_text.language_menu_label.clone(),
                self.ui_text.about_label.clone(),
                self.ui_text.exit_label.clone(),
            ]
        } else {
            self.ui_text
                .shell_menu
                .items
                .iter()
                .map(|item| item.label.clone())
                .collect()
        }
    }

    fn shell_modal_snapshot(&self) -> Option<ShellModalSnapshot> {
        match &self.shell_modal {
            Some(ShellModalState::Root) => Some(ShellModalSnapshot::Root {
                selected_index: self
                    .shell_menu_index
                    .min(self.shell_menu_options().len().saturating_sub(1)),
                options: self.shell_menu_options(),
            }),
            Some(ShellModalState::Submenu { items }) => {
                let index = self.shell_menu_index.min(items.len().saturating_sub(1));
                Some(ShellModalSnapshot::Selection {
                    title: self.ui_text.shell_menu_title.clone(),
                    selected_index: index,
                    options: items.iter().map(|item| item.label.clone()).collect(),
                    hint: self.ui_text.modal_close_hint.clone(),
                })
            }
            Some(ShellModalState::Help) => Some(ShellModalSnapshot::Detail {
                title: self.ui_text.help_label.clone(),
                body: self.runtime.help_text(),
                hint: self.ui_text.modal_close_hint.clone(),
                scroll: self.shell_modal_scroll,
            }),
            Some(ShellModalState::ThingsToDo) => {
                let summaries = self
                    .runtime
                    .current_objective_summaries()
                    .unwrap_or_default();
                let body = if summaries.is_empty() {
                    self.ui_text.things_to_do_empty.clone()
                } else {
                    summaries
                        .into_iter()
                        .map(|(_, message)| message)
                        .filter(|m| !m.is_empty())
                        .collect::<Vec<_>>()
                        .join("\n\n")
                };
                let body = if body.is_empty() {
                    self.ui_text.things_to_do_empty.clone()
                } else {
                    body
                };
                Some(ShellModalSnapshot::Detail {
                    title: self.ui_text.things_to_do_label.clone(),
                    body,
                    hint: self.ui_text.modal_close_hint.clone(),
                    scroll: self.shell_modal_scroll,
                })
            }
            Some(ShellModalState::Language) => Some(ShellModalSnapshot::Selection {
                title: self.ui_text.language_modal_title.clone(),
                selected_index: self
                    .language_menu_index
                    .min(self.available_locales.len().saturating_sub(1)),
                options: self
                    .available_locales
                    .iter()
                    .map(|locale| locale.label.clone())
                    .collect(),
                hint: self.ui_text.menu_choice_hint.clone(),
            }),
            Some(ShellModalState::Rooms) => {
                let options = self.room_switch_options().unwrap_or_default();
                Some(ShellModalSnapshot::Selection {
                    title: self.ui_text.room_switcher_title.clone(),
                    selected_index: self.room_menu_index.min(options.len().saturating_sub(1)),
                    options: options.into_iter().map(|option| option.menu_text).collect(),
                    hint: self.ui_text.menu_choice_hint.clone(),
                })
            }
            Some(ShellModalState::Follow) => {
                let options = self.follow_actor_options().unwrap_or_default();
                Some(ShellModalSnapshot::Selection {
                    title: self.ui_text.follow_actor_title.clone(),
                    selected_index: self.follow_menu_index.min(options.len().saturating_sub(1)),
                    options: options.into_iter().map(|option| option.menu_text).collect(),
                    hint: self.ui_text.menu_choice_hint.clone(),
                })
            }
            Some(ShellModalState::About) => Some(ShellModalSnapshot::Detail {
                title: self.ui_text.about_label.clone(),
                body: self.ui_text.about_body.clone(),
                hint: self.ui_text.modal_close_hint.clone(),
                scroll: self.shell_modal_scroll,
            }),
            Some(ShellModalState::ExitConfirm) => Some(ShellModalSnapshot::Detail {
                title: self.ui_text.exit_confirm_title.clone(),
                body: self.ui_text.exit_confirm_body.clone(),
                hint: self.ui_text.modal_close_hint.clone(),
                scroll: self.shell_modal_scroll,
            }),
            Some(ShellModalState::DaySummary { day_number, body }) => {
                Some(ShellModalSnapshot::Detail {
                    title: self.format_day_summary_title(*day_number),
                    body: body.clone(),
                    hint: self.ui_text.modal_close_hint.clone(),
                    scroll: self.shell_modal_scroll,
                })
            }
            Some(ShellModalState::SessionFeedback { data }) => {
                Some(ShellModalSnapshot::SessionFeedback {
                    rating: data.rating,
                    review_text: data.review_text.clone(),
                    hint: self.ui_text.modal_close_hint.clone(),
                })
            }
            Some(ShellModalState::Projector(_)) => None,
            None => None,
        }
    }

    fn poll_pending_turn(&mut self) {
        let Some(receiver) = self.pending_turn.take() else {
            return;
        };

        match receiver.try_recv() {
            Ok(Ok(outcome)) => {
                self.pending_turn = None;
                self.apply_turn_outcome(outcome);
            }
            Ok(Err(error)) => {
                self.pending_turn = None;
                self.push_transcript(format!("{} {error}", self.ui_text.error_prefix));
                self.sync_tick_pause();
            }
            Err(TryRecvError::Empty) => {
                self.pending_turn = Some(receiver);
            }
            Err(TryRecvError::Disconnected) => {
                self.pending_turn = None;
                self.push_transcript(format!(
                    "{} {}",
                    self.ui_text.error_prefix, self.ui_text.response_worker_disconnected
                ));
                self.sync_tick_pause();
            }
        }
    }

    fn poll_tick_updates(&mut self) {
        loop {
            match self.tick_updates.try_recv() {
                Ok(Ok(outcome)) => {
                    self.apply_turn_outcome(outcome);
                }
                Ok(Err(_error)) => {}
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
            }
        }
    }

    fn flush_projector_narrative(&mut self) -> Result<(), Box<dyn Error>> {
        let lines = self.runtime.consume_pending_projector_narrative_lines()?;
        if !lines.is_empty() {
            self.push_transcript(lines.join("\n\n"));
        }
        Ok(())
    }
}

fn bullet_join(lines: Vec<String>) -> String {
    lines
        .into_iter()
        .map(|line| format!("- {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn summarize_day_highlights(transcript: &[String], start_index: usize) -> Vec<String> {
    transcript
        .iter()
        .skip(start_index)
        .filter_map(|entry| {
            let trimmed = entry.trim();
            if trimmed.is_empty() || trimmed.starts_with('>') || trimmed.starts_with("== ") {
                return None;
            }
            let first_line = trimmed.lines().next()?.trim();
            if first_line.is_empty() {
                None
            } else {
                Some(first_line.to_string())
            }
        })
        .rev()
        .fold(Vec::new(), |mut acc, line| {
            if !acc.contains(&line) && acc.len() < 6 {
                acc.push(line);
            }
            acc
        })
        .into_iter()
        .rev()
        .collect()
}

impl Drop for TuiApp {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            SetCursorStyle::DefaultUserShape
        );
        let _ = self.terminal.show_cursor();
    }
}

#[cfg(test)]
mod tests {
    use super::summarize_day_highlights;
    use cinder_core::content::types::UiTextDefinition;
    use serde_json::json;

    fn build_final_summary_body(
        ui_text: &UiTextDefinition,
        highlights: &str,
        _relationships: &str,
        _preview_lines: &str,
    ) -> String {
        format!("{}\n{}", ui_text.final_summary_highlights_label, highlights)
    }

    fn format_summary_lines(text: &str) -> String {
        let lines = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>();
        if lines.len() <= 1 {
            return text.trim().to_string();
        }
        lines
            .into_iter()
            .map(|line| {
                if line.starts_with("- ") || line.starts_with("• ") {
                    line.to_string()
                } else {
                    format!("• {line}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn day_highlights_skip_commands_and_keep_recent_unique_lines() {
        let transcript = vec![
            "== Aera ==".to_string(),
            "> look".to_string(),
            "Aera: Hi.".to_string(),
            "Ren: Nice to meet you.".to_string(),
            "Aera: Hi.".to_string(),
            "> move kitchen".to_string(),
            "Everyone drifts toward the kitchen.".to_string(),
        ];
        assert_eq!(
            summarize_day_highlights(&transcript, 0),
            vec![
                "Ren: Nice to meet you.".to_string(),
                "Aera: Hi.".to_string(),
                "Everyone drifts toward the kitchen.".to_string(),
            ]
        );
    }

    #[test]
    fn final_summary_body_uses_labels_and_preview() {
        let ui_text: UiTextDefinition = serde_json::from_value(json!({})).expect("default ui text");
        let body = build_final_summary_body(
            &ui_text,
            "Dinner got everyone in the same room.",
            "Aera and Ren seem to be finding a real connection.",
            "Tomorrow starts the first real morning in the house.",
        );
        assert!(body.contains("What happened"));
        assert!(!body.contains("Relationship status"));
        assert!(!body.contains("Next chapter"));
        assert!(body.contains("Dinner got everyone in the same room."));
    }

    #[test]
    fn format_summary_lines_bulletizes_multi_line_boards() {
        assert_eq!(
            format_summary_lines(
                "Aera / Ren — they kept drifting back together.\nAera / Mio — still a little jagged."
            ),
            "• Aera / Ren — they kept drifting back together.\n• Aera / Mio — still a little jagged."
        );
    }
}
