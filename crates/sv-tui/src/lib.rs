use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    BarChart, Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs, Wrap,
};
use ratatui::{Frame, Terminal};
use std::collections::{HashMap, HashSet};
use std::io::{self, Stdout};
use std::time::{Duration, Instant};

use sv_core::{DetectedChange, Entry, EntryStatus, EntryType, Rationale, SystemInfo, VaultRepository};
use sv_core::Tag;
use sv_detectors::{default_detectors, run_detectors};
use sv_fs::{resolve_vault_path, set_config_path, FsVault};

const TICK_RATE: Duration = Duration::from_millis(200);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Dashboard,
    Library,
    Inbox,
    Snoozed,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Focus {
    List,
    Detail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum InputMode {
    None,
    Rationale,
    Palette,
    Init,
    Filter,
    SnoozeQuery,
    SettingsPath,
    Confirm,
    ManualCapture,
}

#[derive(Debug, Clone, Copy)]
enum ConfirmAction {
    MoveVault,
    SwitchVault,
}

#[derive(Debug, Clone)]
struct PendingConfirm {
    action: ConfirmAction,
    target: std::path::PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CaptureStep {
    Title,
    Rationale,
    Command,
    Tags,
    EntryType,
    Verification,
}

#[derive(Debug, Clone)]
struct ManualCapture {
    step: CaptureStep,
    title: String,
    rationale: String,
    cmd: String,
    tags: Vec<String>,
    entry_type: EntryType,
    verification: Option<String>,
}

#[derive(Debug)]

struct App {
    tab: Tab,
    focus: Focus,
    inbox: Vec<DetectedChange>,
    library: Vec<Entry>,
    inbox_state: ListState,
    library_state: ListState,
    selected_inbox: HashSet<uuid::Uuid>,
    selected_library: HashSet<uuid::Uuid>,
    input_mode: InputMode,
    input: TextInput,
    status: Option<String>,
    show_help: bool,
    palette_input: TextInput,
    palette_state: ListState,
    commands: Vec<PaletteCommand>,
    filter_input: TextInput,
    active_filter: Option<String>,
    inbox_source_index: usize,
    snoozed: Vec<DetectedChange>,
    snoozed_state: ListState,

    selected_snoozed: HashSet<uuid::Uuid>,
    library_source_index: usize,
    current_vault_path: String,
    settings_path: String,
    pending_confirm: Option<PendingConfirm>,
    manual_capture: Option<ManualCapture>,
}

#[derive(Debug, Default, Clone)]
struct TextInput {
    content: String,
    cursor: usize,
}

impl TextInput {

    fn from(content: String) -> Self {
        let cursor = content.len();
        Self { content, cursor }
    }

    fn insert(&mut self, c: char) {
        if self.cursor <= self.content.len() {
            self.content.insert(self.cursor, c);
            self.cursor += 1;
        }
    }

    fn delete_back(&mut self) {
        if self.cursor > 0 && self.cursor <= self.content.len() {
            self.content.remove(self.cursor - 1);
            self.cursor -= 1;
        }
    }

    fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    fn move_right(&mut self) {
        if self.cursor < self.content.len() {
            self.cursor += 1;
        }
    }

    fn move_home(&mut self) {
        self.cursor = 0;
    }

    fn move_end(&mut self) {
        self.cursor = self.content.len();
    }
    
    fn reset(&mut self) {
        self.content.clear();
        self.cursor = 0;
    }
}

impl App {
    fn new() -> Self {
        let mut inbox_state = ListState::default();
        inbox_state.select(Some(0));
        let mut library_state = ListState::default();
        library_state.select(Some(0));
        let mut snoozed_state = ListState::default();
        snoozed_state.select(Some(0));
        Self {
            tab: Tab::Dashboard,
            focus: Focus::List,
            inbox: Vec::new(),
            library: Vec::new(),
            inbox_state,
            library_state,
            selected_inbox: HashSet::new(),
            selected_library: HashSet::new(),
            input_mode: InputMode::None,
            input: TextInput::default(),
            status: None,
            show_help: false,
            palette_input: TextInput::default(),
            palette_state: ListState::default(),
            commands: build_commands(),
            filter_input: TextInput::default(),
            active_filter: None,
            inbox_source_index: 0,
            snoozed: Vec::new(),
            snoozed_state,
            selected_snoozed: HashSet::new(),
            library_source_index: 0,
            current_vault_path: String::new(),
            settings_path: String::new(),
            pending_confirm: None,
            manual_capture: None,
        }
    }

    fn available_sources(&self) -> Vec<String> {
        let mut sources: Vec<String> = self.inbox
            .iter()
            .map(|item| item.source.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        sources.sort();
        let mut all = vec!["All".to_string()];
        all.extend(sources);
        all
    }

    fn next_source(&mut self) {
        let count = self.available_sources().len();
        if count > 0 {
            self.inbox_source_index = (self.inbox_source_index + 1) % count;
            self.inbox_state.select(Some(0));
        }
    }

    fn prev_source(&mut self) {
        let count = self.available_sources().len();
        if count > 0 {
            if self.inbox_source_index == 0 {
                self.inbox_source_index = count - 1;
            } else {
                self.inbox_source_index -= 1;
            }
            self.inbox_state.select(Some(0));
        }
    }

    fn available_library_sources(&self) -> Vec<String> {
        let mut sources: Vec<String> = self.library
            .iter()
            .map(|item| item.source.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        sources.sort();
        let mut all = vec!["All".to_string()];
        all.extend(sources);
        all
    }

    fn next_library_source(&mut self) {
        let count = self.available_library_sources().len();
        if count > 0 {
            self.library_source_index = (self.library_source_index + 1) % count;
            self.library_state.select(Some(0));
        }
    }

    fn prev_library_source(&mut self) {
        let count = self.available_library_sources().len();
        if count > 0 {
            if self.library_source_index == 0 {
                self.library_source_index = count - 1;
            } else {
                self.library_source_index -= 1;
            }
            self.library_state.select(Some(0));
        }
    }

    fn filtered_inbox(&self) -> Vec<&DetectedChange> {
        let sources = self.available_sources();
        let current_source = if self.inbox_source_index < sources.len() {
             &sources[self.inbox_source_index]
        } else {
             "All"
        };

        let source_filtered = self.inbox.iter().filter(|item| {
            current_source == "All" || &item.source == current_source
        });

        if let Some(query) = &self.active_filter {
            let query = query.to_lowercase();
            source_filtered
                .filter(|item| {
                     item.title.to_lowercase().contains(&query)
                        || item.cmd.to_lowercase().contains(&query)
                })
                .collect()
        } else {
            source_filtered.collect()
        }
    }

    fn filtered_library(&self) -> Vec<&Entry> {
        let sources = self.available_library_sources();
        let current_source = if self.library_source_index < sources.len() {
             &sources[self.library_source_index]
        } else {
             "All"
        };

        let source_filtered = self.library.iter().filter(|item| {
            current_source == "All" || &item.source == current_source
        });

        if let Some(query) = &self.active_filter {
            let query = query.to_lowercase();
            source_filtered
                .filter(|entry| {
                     entry.title.to_lowercase().contains(&query)
                        || entry.cmd.to_lowercase().contains(&query)
                })
                .collect()
        } else {
            source_filtered.collect()
        }
    }

    fn filtered_snoozed(&self) -> Vec<&DetectedChange> {
        if let Some(query) = &self.active_filter {
            let query = query.to_lowercase();
            self.snoozed
                .iter()
                .filter(|item| {
                     item.title.to_lowercase().contains(&query)
                        || item.cmd.to_lowercase().contains(&query)
                })
                .collect()
        } else {
            self.snoozed.iter().collect()
        }
    }


    fn next_tab(&mut self) {
        self.tab = match self.tab {
            Tab::Dashboard => Tab::Library,
            Tab::Library => Tab::Inbox,
            Tab::Inbox => Tab::Snoozed,
            Tab::Snoozed => Tab::Settings,
            Tab::Settings => Tab::Dashboard,
        };
        self.focus = Focus::List;
    }

    fn prev_tab(&mut self) {
        self.tab = match self.tab {
            Tab::Dashboard => Tab::Settings,
            Tab::Snoozed => Tab::Inbox,
            Tab::Inbox => Tab::Library,
            Tab::Library => Tab::Dashboard,
            Tab::Settings => Tab::Snoozed,
        };
        self.focus = Focus::List;
    }

    fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::List => Focus::Detail,
            Focus::Detail => Focus::List,
        };
    }

    fn select_next(list_state: &mut ListState, len: usize) {
        let i = match list_state.selected() {
            Some(i) => {
                if i + 1 >= len { 0 } else { i + 1 }
            }
            None => 0,
        };
        list_state.select(Some(i));
    }

    fn select_prev(list_state: &mut ListState, len: usize) {
        let i = match list_state.selected() {
            Some(i) => {
                if i == 0 { len.saturating_sub(1) } else { i - 1 }
            }
            None => 0,
        };
        list_state.select(Some(i));
    }

    fn select_first(list_state: &mut ListState) {
        list_state.select(Some(0));
    }

    fn select_last(list_state: &mut ListState, len: usize) {
        if len > 0 {
            list_state.select(Some(len - 1));
        }
    }

    fn select_page_down(list_state: &mut ListState, len: usize) {
        if len == 0 {
            return;
        }
        let i = list_state.selected().unwrap_or(0);
        let next = (i + 5).min(len - 1);
        list_state.select(Some(next));
    }

    fn select_page_up(list_state: &mut ListState) {
        let i = list_state.selected().unwrap_or(0);
        let next = i.saturating_sub(5);
        list_state.select(Some(next));
    }
}

pub fn run() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut vault = FsVault::new(resolve_vault_path()?);
    let mut app = App::new();

    if !vault.exists() {
        app.input_mode = InputMode::Init;
        app.input = TextInput::from(vault.path().to_string_lossy().to_string());
    } else {
        load_data(&vault, &mut app)?;
    }

    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|frame| render_app(frame, &app))?;

        let timeout = TICK_RATE.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press && handle_key(&mut vault, &mut app, key)? {
                    break;
                }
            }
        }

        if last_tick.elapsed() >= TICK_RATE {
            last_tick = Instant::now();
        }
    }

    restore_terminal(terminal)?;
    Ok(())
}

fn load_data(vault: &FsVault, app: &mut App) -> Result<()> {
    app.inbox = vault.load_inbox().unwrap_or_default();
    app.snoozed = vault.load_snoozed().unwrap_or_default();
    app.library = vault.list().unwrap_or_default();
    let current_path = vault.path().to_string_lossy().to_string();
    app.current_vault_path = current_path.clone();
    if app.settings_path.is_empty() || app.settings_path == app.current_vault_path {
        app.settings_path = current_path;
    }
    if app.inbox_state.selected().is_none() && !app.inbox.is_empty() {
        app.inbox_state.select(Some(0));
    }
    if app.snoozed_state.selected().is_none() && !app.snoozed.is_empty() {
        app.snoozed_state.select(Some(0));
    }
    if app.library_state.selected().is_none() && !app.library.is_empty() {
        app.library_state.select(Some(0));
    }
    Ok(())
}

fn handle_key(vault: &mut FsVault, app: &mut App, key: KeyEvent) -> Result<bool> {
    if matches!(app.input_mode, InputMode::Init) {
        return handle_init_input(vault, app, key);
    }
    if matches!(app.input_mode, InputMode::Rationale) {
        return handle_rationale_input(vault, app, key);
    }
    if matches!(app.input_mode, InputMode::Palette) {
        return handle_palette_input(vault, app, key);
    }
    if matches!(app.input_mode, InputMode::Filter) {
        return handle_filter_input(app, key);
    }
    if matches!(app.input_mode, InputMode::SnoozeQuery) {
        return handle_snooze_query(vault, app, key);
    }
    if matches!(app.input_mode, InputMode::SettingsPath) {
        return handle_settings_path_input(app, key);
    }
    if matches!(app.input_mode, InputMode::Confirm) {
        return handle_confirm_input(vault, app, key);
    }
    if matches!(app.input_mode, InputMode::ManualCapture) {
        return handle_manual_capture_input(vault, app, key);
    }

    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('u') => {
                handle_list_move(app, Move::PageUp);
                return Ok(false);
            }
            KeyCode::Char('d') => {
                handle_list_move(app, Move::PageDown);
                return Ok(false);
            }
            _ => {}
        }
    }

    match key.code {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Char('?') => {
            app.show_help = !app.show_help;
        }
        KeyCode::Char('p') | KeyCode::Char(':') => {
            open_palette(app);
        }
        KeyCode::Char('/') => {
            if matches!(app.tab, Tab::Inbox | Tab::Library | Tab::Snoozed) {
                app.input_mode = InputMode::Filter;
                app.filter_input.reset();
                if let Some(current) = &app.active_filter {
                     app.filter_input = TextInput::from(current.clone());
                }
            }
        }
        KeyCode::Esc => {
             app.active_filter = None;
             app.filter_input.reset();
        }
        KeyCode::Right => app.next_tab(),
        KeyCode::Left => app.prev_tab(),

        KeyCode::Char('h') => {
            if app.tab == Tab::Inbox {
                app.prev_source();
            } else if app.tab == Tab::Library {
                app.prev_library_source();
            } else if app.tab == Tab::Dashboard || app.tab == Tab::Snoozed || app.tab == Tab::Settings {
                 app.prev_tab();
            } else {
                app.toggle_focus();
            }
        }
        KeyCode::Char('l') => {
            if app.tab == Tab::Inbox {
                app.next_source();
            } else if app.tab == Tab::Library {
                 app.next_library_source();
            } else if app.tab == Tab::Dashboard || app.tab == Tab::Snoozed || app.tab == Tab::Settings {
                 app.next_tab();
            } else {
                 app.toggle_focus();
            }
        }
        KeyCode::Char('j') | KeyCode::Down => handle_list_move(app, Move::Down),
        KeyCode::Char('k') | KeyCode::Up => handle_list_move(app, Move::Up),
        KeyCode::PageDown => handle_list_move(app, Move::PageDown),
        KeyCode::PageUp => handle_list_move(app, Move::PageUp),
        KeyCode::Home | KeyCode::Char('g') => handle_list_move(app, Move::First),
        KeyCode::End | KeyCode::Char('G') => handle_list_move(app, Move::Last),
        KeyCode::Char('d') => handle_ignore(vault, app)?,
        KeyCode::Char('s') => handle_snooze(vault, app)?,
        KeyCode::Char('u') => handle_unsnooze(vault, app)?,
        KeyCode::Char('a') => {
            if app.tab == Tab::Settings {
                confirm_settings_change(app, ConfirmAction::SwitchVault);
            } else {
                handle_accept(app);
            }
        }
        KeyCode::Char('e') => {
            if app.tab == Tab::Settings {
                open_settings_path_input(app);
            } else {
                handle_edit_rationale(app);
            }
        }
        KeyCode::Char('m') => {
            if app.tab == Tab::Settings {
                confirm_settings_change(app, ConfirmAction::MoveVault);
            }
        }
        KeyCode::Char('r') => handle_refresh(vault, app)?,
        KeyCode::Char('c') => open_manual_capture(app),
        KeyCode::Char('x') => handle_remove(vault, app)?,
        KeyCode::Char(' ') => toggle_selection(app),
        KeyCode::Tab if app.tab != Tab::Dashboard && app.tab != Tab::Settings => app.toggle_focus(),
        KeyCode::BackTab if app.tab != Tab::Dashboard && app.tab != Tab::Settings => app.toggle_focus(),
        KeyCode::Enter => {
            if app.tab != Tab::Dashboard && app.tab != Tab::Settings {
                app.toggle_focus();
            }
        }
        _ => {}
    }

    Ok(false)
}

fn handle_rationale_input(vault: &FsVault, app: &mut App, key: KeyEvent) -> Result<bool> {
    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::None;
            app.input.reset();
        }
        KeyCode::Enter => {
            submit_rationale(vault, app)?;
            app.input_mode = InputMode::None;
            app.input.reset();
        }
        KeyCode::Char(c) => app.input.insert(c),
        KeyCode::Backspace => app.input.delete_back(),
        KeyCode::Left => app.input.move_left(),
        KeyCode::Right => app.input.move_right(),
        KeyCode::Home => app.input.move_home(),
        KeyCode::End => app.input.move_end(),
        _ => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                 match key.code {
                     KeyCode::Char('a') => app.input.move_home(),
                     KeyCode::Char('e') => app.input.move_end(),
                     _ => {}
                 }
            }
        }
    }
    Ok(false)
}



fn handle_filter_input(app: &mut App, key: KeyEvent) -> Result<bool> {
    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::None;
            app.active_filter = None;
            app.filter_input.reset();
        }
        KeyCode::Enter => {
             app.input_mode = InputMode::None;
             if app.filter_input.content.is_empty() {
                 app.active_filter = None;
             } else {
                 app.active_filter = Some(app.filter_input.content.clone());
             }
        }
        KeyCode::Char(c) => {
            app.filter_input.insert(c);
            app.active_filter = Some(app.filter_input.content.clone());
            if app.tab == Tab::Inbox {
                app.inbox_state.select(Some(0));
            } else if app.tab == Tab::Library {
                 app.library_state.select(Some(0));
            } else if app.tab == Tab::Snoozed {
                 app.snoozed_state.select(Some(0));
            }
        }
        KeyCode::Backspace => {
            app.filter_input.delete_back();
            if app.filter_input.content.is_empty() {
                app.active_filter = None;
            } else {
                app.active_filter = Some(app.filter_input.content.clone());
            }
             if app.tab == Tab::Inbox {
                app.inbox_state.select(Some(0));
            } else if app.tab == Tab::Library {
                 app.library_state.select(Some(0));
            } else if app.tab == Tab::Snoozed {
                 app.snoozed_state.select(Some(0));
            }
        }
        KeyCode::Left => app.filter_input.move_left(),
        KeyCode::Right => app.filter_input.move_right(),
        KeyCode::Home => app.filter_input.move_home(),
        KeyCode::End => app.filter_input.move_end(),
        _ => {
             if key.modifiers.contains(KeyModifiers::CONTROL) {
                  match key.code {
                      KeyCode::Char('a') => app.filter_input.move_home(),
                      KeyCode::Char('e') => app.filter_input.move_end(),
                      _ => {}
                  }
             }
        }
    }
    Ok(false)
}

fn handle_settings_path_input(app: &mut App, key: KeyEvent) -> Result<bool> {
    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::None;
            app.input.reset();
        }
        KeyCode::Enter => {
            app.settings_path = app.input.content.clone();
            app.input_mode = InputMode::None;
            app.input.reset();
            app.status = Some("Updated pending vault path".into());
        }
        KeyCode::Char(c) => app.input.insert(c),
        KeyCode::Backspace => app.input.delete_back(),
        KeyCode::Left => app.input.move_left(),
        KeyCode::Right => app.input.move_right(),
        KeyCode::Home => app.input.move_home(),
        KeyCode::End => app.input.move_end(),
        _ => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                 match key.code {
                     KeyCode::Char('a') => app.input.move_home(),
                     KeyCode::Char('e') => app.input.move_end(),
                     _ => {}
                 }
            }
        }
    }
    Ok(false)
}

fn handle_confirm_input(vault: &mut FsVault, app: &mut App, key: KeyEvent) -> Result<bool> {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            if let Some(pending) = app.pending_confirm.clone() {
                apply_settings_change(vault, app, pending)?;
            }
            app.pending_confirm = None;
            app.input_mode = InputMode::None;
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.pending_confirm = None;
            app.input_mode = InputMode::None;
            app.status = Some("Cancelled settings change".into());
        }
        _ => {}
    }
    Ok(false)
}

fn handle_manual_capture_input(vault: &FsVault, app: &mut App, key: KeyEvent) -> Result<bool> {
    let Some(capture) = app.manual_capture.as_mut() else {
        app.input_mode = InputMode::None;
        return Ok(false);
    };

    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::None;
            app.input.reset();
            app.manual_capture = None;
            app.status = Some("Manual capture cancelled".into());
        }
        KeyCode::Enter => {
            match capture.step {
                CaptureStep::Title => {
                    capture.title = app.input.content.trim().to_string();
                    capture.step = CaptureStep::Rationale;
                }
                CaptureStep::Rationale => {
                    capture.rationale = app.input.content.trim().to_string();
                    capture.step = CaptureStep::Command;
                }
                CaptureStep::Command => {
                    capture.cmd = app.input.content.trim().to_string();
                    capture.step = CaptureStep::Tags;
                }
                CaptureStep::Tags => {
                    capture.tags = parse_tag_list(&app.input.content);
                    capture.step = CaptureStep::EntryType;
                }
                CaptureStep::EntryType => {
                    capture.entry_type = parse_entry_type(&app.input.content);
                    capture.step = CaptureStep::Verification;
                }
                CaptureStep::Verification => {
                    let value = app.input.content.trim();
                    capture.verification = if value.is_empty() {
                        None
                    } else {
                        Some(value.to_string())
                    };
                    finalize_manual_capture(vault, app)?;
                    app.input_mode = InputMode::None;
                    app.manual_capture = None;
                }
            }
            app.input.reset();
        }
        KeyCode::Char(c) => app.input.insert(c),
        KeyCode::Backspace => app.input.delete_back(),
        KeyCode::Left => app.input.move_left(),
        KeyCode::Right => app.input.move_right(),
        KeyCode::Home => app.input.move_home(),
        KeyCode::End => app.input.move_end(),
        _ => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                match key.code {
                    KeyCode::Char('a') => app.input.move_home(),
                    KeyCode::Char('e') => app.input.move_end(),
                    _ => {}
                }
            }
        }
    }
    Ok(false)
}

fn handle_palette_input(vault: &FsVault, app: &mut App, key: KeyEvent) -> Result<bool> {
    match key.code {
        KeyCode::Esc => {
            close_palette(app);
        }
        KeyCode::Enter => {
            let commands = filtered_commands(app);
            let action = app.palette_state.selected()
                .and_then(|i| commands.get(i))
                .map(|c| c.action);
            
            close_palette(app);
            
            if let Some(action) = action {
                if matches!(action, CommandAction::Quit) {
                    return Ok(true);
                }
                execute_command(vault, app, action)?;
            }
        }
        KeyCode::Char(c) => {
            app.palette_input.insert(c);
            app.palette_state.select(Some(0));
        }
        KeyCode::Backspace => {
            app.palette_input.delete_back();
            app.palette_state.select(Some(0));
        }
        KeyCode::Left => app.palette_input.move_left(),
        KeyCode::Right => app.palette_input.move_right(),
        KeyCode::Home => app.palette_input.move_home(),
        KeyCode::End => app.palette_input.move_end(),
        KeyCode::Up => {
            let len = filtered_commands(app).len();
            App::select_prev(&mut app.palette_state, len);
        }
        KeyCode::Down => {
            let len = filtered_commands(app).len();
            App::select_next(&mut app.palette_state, len);
        }
        KeyCode::PageUp => App::select_page_up(&mut app.palette_state),
        KeyCode::PageDown => {
            let len = filtered_commands(app).len();
            App::select_page_down(&mut app.palette_state, len);
        }
        _ => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                 match key.code {
                     KeyCode::Char('a') => app.palette_input.move_home(),
                     KeyCode::Char('e') => app.palette_input.move_end(),
                     _ => {}
                 }
            }
        }
    }
    Ok(false)
}

fn handle_list_move(app: &mut App, movement: Move) {
    match app.tab {
        Tab::Inbox => {
            let len = app.filtered_inbox().len();
            move_list(&mut app.inbox_state, len, movement);
        }
        Tab::Library => {
            let len = app.filtered_library().len();
            move_list(&mut app.library_state, len, movement);
        }
        Tab::Dashboard => {}
        Tab::Snoozed => {
            let len = app.filtered_snoozed().len();
            move_list(&mut app.snoozed_state, len, movement);
        }
        Tab::Settings => {}
    }
}

fn handle_accept(app: &mut App) {
    if app.tab == Tab::Inbox {
        app.input_mode = InputMode::Rationale;
        app.input.reset();
    }
}

fn handle_edit_rationale(app: &mut App) {
    match app.tab {
        Tab::Library => {
            if let Some(selected) = app.library_state.selected() {
                let rationale = {
                    let filtered = app.filtered_library();
                    filtered.get(selected).map(|e| e.rationale.as_str().to_string())
                };
                
                if let Some(r) = rationale {
                     app.input_mode = InputMode::Rationale;
                     app.input = TextInput::from(r);
                }
            }
        }
        Tab::Snoozed | Tab::Dashboard | Tab::Inbox => {}
        Tab::Settings => {}
    }
}

fn open_settings_path_input(app: &mut App) {
    app.input_mode = InputMode::SettingsPath;
    app.input = TextInput::from(app.settings_path.clone());
}

fn confirm_settings_change(app: &mut App, action: ConfirmAction) {
    let target = std::path::PathBuf::from(app.settings_path.clone());
    if app.settings_path.trim().is_empty() {
        app.status = Some("Pending path is empty".into());
        return;
    }
    if app.settings_path == app.current_vault_path {
        app.status = Some("Pending path matches current vault path".into());
        return;
    }
    app.pending_confirm = Some(PendingConfirm { action, target });
    app.input_mode = InputMode::Confirm;
}

fn open_manual_capture(app: &mut App) {
    app.manual_capture = Some(ManualCapture {
        step: CaptureStep::Title,
        title: String::new(),
        rationale: String::new(),
        cmd: String::new(),
        tags: Vec::new(),
        entry_type: EntryType::Other,
        verification: None,
    });
    app.input_mode = InputMode::ManualCapture;
    app.input.reset();
}

fn finalize_manual_capture(vault: &FsVault, app: &mut App) -> Result<()> {
    let Some(capture) = app.manual_capture.clone() else {
        return Ok(());
    };

    if capture.title.trim().is_empty() || capture.rationale.trim().is_empty() {
        app.status = Some("Title and rationale are required".into());
        return Ok(());
    }

    let cmd = if capture.cmd.trim().is_empty() {
        "manual entry".to_string()
    } else {
        capture.cmd.trim().to_string()
    };

    let entry = Entry::new(
        uuid::Uuid::new_v4(),
        capture.title,
        capture.entry_type,
        "manual",
        cmd,
        SystemInfo {
            os: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
        },
        chrono::Utc::now(),
        EntryStatus::Active,
        capture
            .tags
            .into_iter()
            .map(Tag::new)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| anyhow::anyhow!(err.to_string()))?,
        Rationale::new(capture.rationale)?,
        capture.verification,
    )?;

    vault.create(&entry)?;
    app.library.push(entry);
    app.status = Some("Manual entry saved".into());
    Ok(())
}

fn parse_tag_list(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(|tag| tag.to_string())
        .collect()
}

fn parse_entry_type(input: &str) -> EntryType {
    match input.trim().to_lowercase().as_str() {
        "package" => EntryType::Package,
        "config" => EntryType::Config,
        "application" => EntryType::Application,
        "script" => EntryType::Script,
        _ => EntryType::Other,
    }
}

fn handle_refresh(vault: &FsVault, app: &mut App) -> Result<()> {
    if app.tab == Tab::Dashboard || app.tab == Tab::Inbox {
        let detectors = default_detectors();

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .context("failed to initialize runtime")?;
        let changes = runtime
            .block_on(run_detectors(detectors))
            .context("detector run failed")?;

        let mut inbox = vault.load_inbox().unwrap_or_default();
        let mut new_changes = Vec::new();
        for (source, group) in group_by_source(&changes) {
            let previous = vault.load_detector_snapshot(&source)?;
            let diff = diff_changes(&previous, &group);
            vault.save_detector_snapshot(&source, &group)?;
            new_changes.extend(diff);
        }
        if !new_changes.is_empty() {
            append_unique(&mut inbox, new_changes);
            vault.save_inbox(&inbox)?;
        }
        app.inbox = inbox;
        if app.inbox_state.selected().is_none() && !app.inbox.is_empty() {
            app.inbox_state.select(Some(0));
        }
    }
    Ok(())
}



fn handle_init_input(vault: &mut FsVault, app: &mut App, key: KeyEvent) -> Result<bool> {
    match key.code {
        KeyCode::Esc => {
            app.input.reset();
        }
        KeyCode::Enter => {
            let path = std::path::PathBuf::from(&app.input.content);
            *vault = FsVault::new(path);
            vault.init().context("failed to initialize vault")?;
            set_config_path(vault.path())?;
            app.input_mode = InputMode::None;
            app.input.reset();
            load_data(vault, app)?;
        }
        KeyCode::Char(c) => app.input.insert(c),
        KeyCode::Backspace => app.input.delete_back(),
        KeyCode::Left => app.input.move_left(),
        KeyCode::Right => app.input.move_right(),
        KeyCode::Home => app.input.move_home(),
        KeyCode::End => app.input.move_end(),
        _ => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                 match key.code {
                     KeyCode::Char('a') => app.input.move_home(),
                     KeyCode::Char('e') => app.input.move_end(),
                     _ => {}
                 }
            }
        }
    }
    Ok(false)
}

fn handle_ignore(vault: &FsVault, app: &mut App) -> Result<()> {
    if app.tab != Tab::Inbox {
        return Ok(());
    }

    let ids_to_ignore: Vec<uuid::Uuid> = if !app.selected_inbox.is_empty() {
        app.selected_inbox.iter().cloned().collect()
    } else {
        current_inbox_id(app).into_iter().collect()
    };

    if ids_to_ignore.is_empty() {
        return Ok(());
    }

    for id in &ids_to_ignore {
        vault.remove_inbox_item(*id)?;
        app.inbox.retain(|item| item.id != *id);
    }

    app.selected_inbox.clear();
    app.status = Some(format!("Ignored {} item(s)", ids_to_ignore.len()));
    Ok(())
}

fn handle_snooze(vault: &FsVault, app: &mut App) -> Result<()> {
    if app.tab != Tab::Inbox {
        return Ok(());
    }

    let ids_to_snooze: Vec<uuid::Uuid> = if !app.selected_inbox.is_empty() {
        app.selected_inbox.iter().cloned().collect()
    } else {
        current_inbox_id(app).into_iter().collect()
    };

    if ids_to_snooze.is_empty() {
        return Ok(());
    }

    for id in &ids_to_snooze {
        vault.snooze_inbox_item(*id)?;
        app.inbox.retain(|item| item.id != *id);
    }

    app.selected_inbox.clear();
    app.status = Some(format!("Snoozed {} item(s)", ids_to_snooze.len()));
    Ok(())
}

fn handle_unsnooze(vault: &FsVault, app: &mut App) -> Result<()> {
    if app.tab != Tab::Snoozed {
        return Ok(());
    }

    let ids_to_unsnooze: Vec<uuid::Uuid> = if !app.selected_snoozed.is_empty() {
        app.selected_snoozed.iter().cloned().collect()
    } else {
        current_snoozed_id(app).into_iter().collect()
    };

    if ids_to_unsnooze.is_empty() {
        return Ok(());
    }

    for id in &ids_to_unsnooze {
        vault.unsnooze_item(*id)?;
        app.snoozed.retain(|item| item.id != *id);
    }

    app.inbox = vault.load_inbox().unwrap_or_default();
    app.selected_snoozed.clear();
    app.status = Some(format!("Restored {} item(s) to inbox", ids_to_unsnooze.len()));
    Ok(())
}

fn submit_rationale(vault: &FsVault, app: &mut App) -> Result<()> {
    match app.tab {
        Tab::Dashboard | Tab::Snoozed | Tab::Settings => {},
        Tab::Inbox => {
            let ids_to_approve: Vec<uuid::Uuid> = if !app.selected_inbox.is_empty() {
                app.selected_inbox.iter().cloned().collect()
            } else {
                current_inbox_id(app).into_iter().collect()
            };

            if ids_to_approve.is_empty() {
                return Ok(());
            }

            let rationale = Rationale::new(app.input.content.clone())?;
            let mut approved_count = 0;

            for id in ids_to_approve {
                if let Some(change) = app.inbox.iter().find(|c| c.id == id).cloned() {
                    if let Some(path) = change.path.as_ref() {
                        if let Ok(contents) = std::fs::read_to_string(path) {
                            if sv_utils::contains_potential_secret(&contents) {
                                app.status = Some(format!("Warning: potential secret in {path}"));
                            }
                        }
                    }

                    let entry = Entry::new(
                        uuid::Uuid::new_v4(),
                        change.title,
                        change.entry_type,
                        change.source,
                        change.cmd,
                        change.system,
                        change.detected_at,
                        EntryStatus::Active,
                        change.tags,
                        rationale.clone(),
                        None,
                    )?;

                    vault.create(&entry)?;
                    vault.remove_inbox_item(change.id)?;
                    app.inbox.retain(|item| item.id != change.id);
                    app.library.push(entry);
                    approved_count += 1;
                }
            }

            app.selected_inbox.clear();
            app.status = Some(format!("Approved {} item(s)", approved_count));
        }
        Tab::Library => {
             if let Some(id) = current_library_id(app) {
                 if let Some(entry) = app.library.iter_mut().find(|e| e.id == id) {
                    entry.rationale = Rationale::new(app.input.content.clone())?;
                    vault.update(entry)?;
                    app.status = Some("Updated rationale".into());
                 }
             }
        }
    }
    Ok(())
}

fn apply_settings_change(
    vault: &mut FsVault,
    app: &mut App,
    pending: PendingConfirm,
) -> Result<()> {
    let target = pending.target;
    let current = vault.path().to_path_buf();

    if target == current {
        app.status = Some("Vault path unchanged".into());
        return Ok(());
    }

    match pending.action {
        ConfirmAction::MoveVault => {
            move_vault(&current, &target)?;
            *vault = FsVault::new(target.clone());
            set_config_path(&target)?;
            app.status = Some("Vault moved to new location".into());
        }
        ConfirmAction::SwitchVault => {
            let new_vault = FsVault::new(target.clone());
            if !new_vault.exists() {
                new_vault.init().context("failed to initialize vault")?;
            }
            *vault = new_vault;
            set_config_path(&target)?;
            app.status = Some("Vault location updated".into());
        }
    }

    app.current_vault_path = vault.path().to_string_lossy().to_string();
    app.settings_path = app.current_vault_path.clone();
    load_data(vault, app)?;
    Ok(())
}

fn move_vault(source: &std::path::Path, target: &std::path::Path) -> Result<()> {
    if !source.exists() {
        return Err(anyhow::anyhow!("source vault path does not exist"));
    }

    if target.exists() {
        if !target.is_dir() {
            return Err(anyhow::anyhow!(
                "target path exists and is not a directory"
            ));
        }
        if !is_dir_empty(target)? {
            return Err(anyhow::anyhow!(
                "target directory is not empty"
            ));
        }
    } else if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).context("failed to create target parent")?;
    }

    if let Err(_) = std::fs::rename(source, target) {
        copy_dir_all(source, target)?;
        std::fs::remove_dir_all(source).context("failed to remove source vault")?;
    }
    Ok(())
}

fn is_dir_empty(path: &std::path::Path) -> Result<bool> {
    let mut entries = std::fs::read_dir(path).context("failed to read target directory")?;
    Ok(entries.next().is_none())
}

fn copy_dir_all(source: &std::path::Path, target: &std::path::Path) -> Result<()> {
    std::fs::create_dir_all(target).context("failed to create target directory")?;
    for entry in std::fs::read_dir(source).context("failed to read source directory")? {
        let entry = entry.context("failed to read source entry")?;
        let path = entry.path();
        let dest = target.join(entry.file_name());
        if path.is_dir() {
            copy_dir_all(&path, &dest)?;
        } else {
            std::fs::copy(&path, &dest).context("failed to copy file")?;
        }
    }
    Ok(())
}

fn render_filter_popup(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 20, frame.size());
    let r = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)].as_ref())
        .split(area);
    
    frame.render_widget(Clear, area); // Clear background

    let input_block = Block::default()
        .borders(Borders::ALL)
        .title("Filter")
        .style(Style::default().fg(Color::Yellow));

    let input = Paragraph::new(app.filter_input.content.as_str())
         .style(Style::default().fg(Color::Yellow))
         .block(input_block);
    
    frame.render_widget(input, r[0]);

    // Visually place cursor
    let cx = r[0].x + 1 + (app.filter_input.cursor as u16).min(r[0].width - 3);
    frame.set_cursor(cx, r[0].y + 1);
}

fn toggle_selection(app: &mut App) {
    match app.tab {
        Tab::Inbox => {
            if let Some(id) = current_inbox_id(app) {
                if !app.selected_inbox.insert(id) {
                    app.selected_inbox.remove(&id);
                }
            }
        }
        Tab::Snoozed => {
            if let Some(id) = current_snoozed_id(app) {
                if !app.selected_snoozed.insert(id) {
                    app.selected_snoozed.remove(&id);
                }
            }
        }
        Tab::Library => {
            if let Some(id) = current_library_id(app) {
                if !app.selected_library.insert(id) {
                    app.selected_library.remove(&id);
                }
            }
        }
        Tab::Dashboard | Tab::Settings => {}
    }
}

fn current_snoozed_id(app: &App) -> Option<uuid::Uuid> {
    let index = app.snoozed_state.selected()?;
    app.filtered_snoozed().get(index).map(|item| item.id)
}

fn current_inbox_id(app: &App) -> Option<uuid::Uuid> {
    let index = app.inbox_state.selected()?;
    app.filtered_inbox().get(index).map(|item| item.id)
}

fn current_library_id(app: &App) -> Option<uuid::Uuid> {
    let index = app.library_state.selected()?;
    app.filtered_library().get(index).map(|item| item.id)
}


#[derive(Debug, Clone, Copy)]
enum Move {
    Up,
    Down,
    PageUp,
    PageDown,
    First,
    Last,
}

fn move_list(state: &mut ListState, len: usize, movement: Move) {
    match movement {
        Move::Up => App::select_prev(state, len),
        Move::Down => App::select_next(state, len),
        Move::PageUp => App::select_page_up(state),
        Move::PageDown => App::select_page_down(state, len),
        Move::First => App::select_first(state),
        Move::Last => App::select_last(state, len),
    }
}

fn render_app(frame: &mut ratatui::Frame, app: &App) {
    let size = frame.size();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(3)].as_ref())
        .split(size);

    let titles = vec!["Dashboard", "Library", "Inbox", "Snoozed", "Settings"]
        .iter()
        .map(|title| Line::from(Span::styled(*title, Style::default())))
        .collect::<Vec<_>>();

    let tabs = Tabs::new(titles)
        .select(match app.tab {
            Tab::Dashboard => 0,
            Tab::Library => 1,
            Tab::Inbox => 2,
            Tab::Snoozed => 3,
            Tab::Settings => 4,
        })
        .block(Block::default().borders(Borders::ALL).title("SetupVault"))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    frame.render_widget(tabs, chunks[0]);

    match app.tab {
        Tab::Dashboard => render_dashboard(frame, chunks[1], app),
        Tab::Library => render_library(frame, chunks[1], app),
        Tab::Inbox => render_inbox(frame, chunks[1], app),
        Tab::Snoozed => render_snoozed(frame, chunks[1], app),
        Tab::Settings => render_settings(frame, chunks[1], app),
    }

    render_guide_bar(frame, chunks[2], app);

    if matches!(app.input_mode, InputMode::Rationale) {
        render_input_popup(frame, size, &app.input);
    }

    if app.show_help {
        render_help_popup(frame, size, &help_text(app));
    }

    if matches!(app.input_mode, InputMode::Palette) {
        render_palette_popup(frame, size, app);
    }

    if matches!(app.input_mode, InputMode::Init) {
        render_init_popup(frame, size, &app.input);
    }

    if matches!(app.input_mode, InputMode::Filter) {
        render_filter_popup(frame, app);
    }

    if matches!(app.input_mode, InputMode::SnoozeQuery) {
        render_snooze_popup(frame, size, &app.input);
    }

    if matches!(app.input_mode, InputMode::SettingsPath) {
        render_settings_path_popup(frame, size, &app.input);
    }

    if matches!(app.input_mode, InputMode::Confirm) {
        render_confirm_popup(frame, size, app);
    }

    if matches!(app.input_mode, InputMode::ManualCapture) {
        render_manual_capture_popup(frame, size, app);
    }
}

fn render_dashboard(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // Summary stats
            Constraint::Min(10),   // Charts
            Constraint::Length(8), // Recent activity
        ])
        .split(area);

    // Row 1: Summary Widgets
    let summary_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(chunks[0]);

    let inbox_count = app.inbox.len();
    let library_count = app.library.len();
    let total_count = inbox_count + library_count;

    let s1 = Paragraph::new(format!("\n{}", inbox_count))
        .alignment(ratatui::layout::Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Inbox Pending"))
        .style(Style::default().fg(if inbox_count > 0 {
            Color::Red
        } else {
            Color::Green
        }));

    let s2 = Paragraph::new(format!("\n{}", library_count))
        .alignment(ratatui::layout::Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Managed Items"))
        .style(Style::default().fg(Color::Cyan));

    let health_pct = if total_count > 0 {
        (library_count as f64 * 100.0) / total_count as f64
    } else {
        100.0
    };
    let s3 = Paragraph::new(format!("\n{:.1}%", health_pct))
        .alignment(ratatui::layout::Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Vault Health"))
        .style(Style::default().fg(Color::Green));

    frame.render_widget(s1, summary_chunks[0]);
    frame.render_widget(s2, summary_chunks[1]);
    frame.render_widget(s3, summary_chunks[2]);

    // Center: Source Breakdown (BarChart)
    let mut source_counts: HashMap<String, u64> = HashMap::new();
    for entry in &app.library {
        *source_counts.entry(entry.source.clone()).or_insert(0) += 1;
    }
    let mut counts_vec: Vec<(String, u64)> = source_counts.into_iter().collect();
    counts_vec.sort_by(|a, b| {
        b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0))
    });

    let bars_data: Vec<(&str, u64)> = counts_vec
        .iter()
        .take(5)
        .map(|(k, v)| (k.as_str(), *v))
        .collect();

    let barchart = BarChart::default()
        .block(Block::default().title("Top Sources").borders(Borders::ALL))
        .data(&bars_data)
        .bar_width(12)
        .bar_gap(2)
        .bar_style(Style::default().fg(Color::Yellow))
        .value_style(Style::default().fg(Color::Black).bg(Color::Yellow));
    frame.render_widget(barchart, chunks[1]);

    // Row 3: Recent Activity
    let recent_items = app
        .library
        .iter()
        .rev()
        .take(5)
        .map(|e| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("[{}] ", e.source), Style::default().fg(Color::Blue)),
                Span::raw(e.title.clone()),
            ]))
        })
        .collect::<Vec<_>>();

    let recent_list = List::new(recent_items).block(
        Block::default()
            .title("Recent Activity")
            .borders(Borders::ALL),
    );

    frame.render_widget(recent_list, chunks[2]);
}

fn render_inbox(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(area);

    // Source Tabs
    let sources = app.available_sources();
    let source_titles: Vec<Line> = sources
        .iter()
        .map(|s| Line::from(s.as_str()))
        .collect();
    
    // Clamp index for safety
    let selected_index = if app.inbox_source_index >= sources.len() { 0 } else { app.inbox_source_index };

    let tabs = Tabs::new(source_titles)
        .select(selected_index)
        .block(Block::default().borders(Borders::BOTTOM))
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    
    frame.render_widget(tabs, chunks[0]);

    let list_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)].as_ref())
        .split(chunks[1]);

    let items = app
        .filtered_inbox()
        .iter()
        .map(|change| {
            let mut title = change.title.clone();
            if app.selected_inbox.contains(&change.id) {
                title = format!("[x] {title}");
            } else {
                title = format!("[ ] {title}");
            }
            ListItem::new(title)
        })
        .collect::<Vec<_>>();
    let list_block = Block::default()
        .borders(Borders::ALL)
        .title(if let Some(filter) = &app.active_filter {
            format!("Inbox (Filtered: {})", filter)
        } else {
            "Inbox".into()
        })
        .border_style(if app.focus == Focus::List {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        });
    let list = List::new(items)
        .block(list_block)
        .highlight_style(Style::default().bg(Color::DarkGray));
    frame.render_stateful_widget(list, list_chunks[0], &mut app.inbox_state.clone());

    let detail = match app.inbox_state.selected().and_then(|i| app.filtered_inbox().get(i).copied()) {
        Some(change) => {
            let mut lines = Vec::new();
            lines.push(Line::from(Span::styled(
                format!("{}", change.title),
                Style::default().add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(format!("Source: {}", change.source)));
            lines.push(Line::from(format!("Type: {:?}", change.entry_type)));
            lines.push(Line::from(format!("Cmd: {}", change.cmd)));
            if let Some(path) = &change.path {
                lines.push(Line::from(format!("Path: {}", path)));
            }
            lines
        }
        None => vec![Line::from("No item selected")],
    };

    let detail_block = Block::default()
        .borders(Borders::ALL)
        .title("Details")
        .border_style(if app.focus == Focus::Detail {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        });
    let detail_p = Paragraph::new(detail)
        .block(detail_block)
        .wrap(Wrap { trim: true });
    
    frame.render_widget(detail_p, list_chunks[1]);
}

fn render_snoozed(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)].as_ref())
        .split(area);

    let items = app.filtered_snoozed()
        .iter()
        .map(|change| {
            let mut title = change.title.clone();
            if app.selected_snoozed.contains(&change.id) {
                title = format!("[x] {title}");
            } else {
                title = format!("[ ] {title}");
            }
            ListItem::new(title)
        })
        .collect::<Vec<_>>();
    let list_block = Block::default()
        .borders(Borders::ALL)
        .title(if let Some(filter) = &app.active_filter {
            format!("Snoozed Items (Filtered: {})", filter)
        } else {
            "Snoozed Items".into()
        })
        .border_style(if app.focus == Focus::List {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        });
    let list = List::new(items)
        .block(list_block)
        .highlight_style(Style::default().bg(Color::DarkGray));
    frame.render_stateful_widget(list, chunks[0], &mut app.snoozed_state.clone());

    let detail = match app.snoozed_state.selected().and_then(|i| app.filtered_snoozed().get(i).copied()) {
        Some(change) => {
            let mut lines = Vec::new();
            lines.push(Line::from(Span::styled(
                format!("{}", change.title),
                Style::default().add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(format!("Source: {}", change.source)));
            lines.push(Line::from(format!("Type: {:?}", change.entry_type)));
            lines.push(Line::from(format!("Cmd: {}", change.cmd)));
            if let Some(path) = &change.path {
                lines.push(Line::from(format!("Path: {}", path)));
            }
            lines
        }
        None => vec![Line::from("No item selected")],
    };

    let detail_block = Block::default()
        .borders(Borders::ALL)
        .title("Details")
        .border_style(if app.focus == Focus::Detail {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        });
    let detail_p = Paragraph::new(detail)
        .block(detail_block)
        .wrap(Wrap { trim: true });
    
    frame.render_widget(detail_p, chunks[1]);
}

fn render_library(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(area);

    // Source Tabs
    let sources = app.available_library_sources();
    let source_titles: Vec<Line> = sources
        .iter()
        .map(|s| Line::from(s.as_str()))
        .collect();
    
    // Clamp index for safety
    let selected_index = if app.library_source_index >= sources.len() { 0 } else { app.library_source_index };

    let tabs = Tabs::new(source_titles)
        .select(selected_index)
        .block(Block::default().borders(Borders::BOTTOM))
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    
    frame.render_widget(tabs, chunks[0]);

    let list_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)].as_ref())
        .split(chunks[1]);

    let items = app
        .filtered_library()
        .iter()
        .map(|entry| {
            let mut title = entry.title.clone();
            if app.selected_library.contains(&entry.id) {
                title = format!("[x] {title}");
            } else {
                title = format!("[ ] {title}");
            }
            ListItem::new(title)
        })
        .collect::<Vec<_>>();
    let list_block = Block::default()
        .borders(Borders::ALL)
        .title(if let Some(filter) = &app.active_filter {
            format!("Library (Filtered: {})", filter)
        } else {
            "Library".into()
        })
        .border_style(if app.focus == Focus::List {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        });
    let list = List::new(items)
        .block(list_block)
        .highlight_style(Style::default().bg(Color::DarkGray));
    frame.render_stateful_widget(list, list_chunks[0], &mut app.library_state.clone());

    let detail = match app.library_state.selected().and_then(|i| app.filtered_library().get(i).copied()) {
        Some(entry) => {
            let mut lines = Vec::new();
            lines.push(Line::from(Span::styled(
                entry.title.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(format!("Source: {}", entry.source)));
            lines.push(Line::from(format!("Type: {:?}", entry.entry_type)));
            lines.push(Line::from(format!("Cmd: {}", entry.cmd)));
            lines.push(Line::from("Rationale:"));
            lines.push(Line::from(entry.rationale.as_str().to_string()));
            Paragraph::new(lines)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Details")
                        .border_style(if app.focus == Focus::Detail {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default()
                        }),
                )
                .wrap(Wrap { trim: true })
        }
        None => Paragraph::new("No entry selected")
            .block(Block::default().borders(Borders::ALL).title("Details")),
    };
    frame.render_widget(detail, list_chunks[1]);
}

fn render_settings(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),
            Constraint::Length(5),
            Constraint::Min(0),
        ])
        .split(area);

    let summary_lines = vec![
        Line::from(Span::styled(
            "Vault Location",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(format!("Current: {}", app.current_vault_path)),
        Line::from(format!("Pending: {}", app.settings_path)),
        Line::from(""),
        Line::from("Edit the pending path and apply to change where SetupVault stores data."),
    ];
    let summary = Paragraph::new(summary_lines)
        .block(Block::default().borders(Borders::ALL).title("Overview"))
        .wrap(Wrap { trim: true });
    frame.render_widget(summary, chunks[0]);

    let actions = vec![
        Line::from("[e] Edit pending path"),
        Line::from("[m] Apply and move data"),
        Line::from("[a] Apply without moving (use existing or create)"),
    ];
    let actions = Paragraph::new(actions)
        .block(Block::default().borders(Borders::ALL).title("Actions"))
        .wrap(Wrap { trim: true });
    frame.render_widget(actions, chunks[1]);

    if let Some(status) = &app.status {
        let status = Paragraph::new(status.as_str())
            .block(Block::default().borders(Borders::ALL).title("Status"))
            .wrap(Wrap { trim: true });
        frame.render_widget(status, chunks[2]);
    } else {
        let hint = Paragraph::new("Changes require confirmation before applying.")
            .block(Block::default().borders(Borders::ALL).title("Status"))
            .wrap(Wrap { trim: true });
        frame.render_widget(hint, chunks[2]);
    }
}

fn render_input_popup(frame: &mut ratatui::Frame, area: Rect, input_data: &TextInput) {
    let popup_area = centered_rect(60, 20, area);
    frame.render_widget(Clear, popup_area);
    let block = Block::default().borders(Borders::ALL).title("Rationale");
    let input_widget = Paragraph::new(input_data.content.as_str())
        .block(block)
        .wrap(Wrap { trim: true });
    frame.render_widget(input_widget, popup_area);
    
    // Simple cursor positioning (approximate for wrapped text, better for single line)
    // For wrap, we would need to calculate line breaks. For now let's assume end of text if flows.
    // A robust impl would use the width.
    let x_offset = (input_data.cursor as u16) % (popup_area.width - 2); 
    let y_offset = (input_data.cursor as u16) / (popup_area.width - 2);
    frame.set_cursor(popup_area.x + 1 + x_offset, popup_area.y + 1 + y_offset);
}

fn render_settings_path_popup(frame: &mut ratatui::Frame, area: Rect, input_data: &TextInput) {
    let popup_area = centered_rect(70, 20, area);
    frame.render_widget(Clear, popup_area);
    let block = Block::default().borders(Borders::ALL).title("Edit Vault Path");

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(2),
                Constraint::Length(3),
                Constraint::Length(2),
            ]
            .as_ref(),
        )
        .margin(1)
        .split(popup_area);

    let text = Paragraph::new("Set the new vault directory path:")
        .wrap(Wrap { trim: true });
    frame.render_widget(text, chunks[0]);

    let input_widget = Paragraph::new(input_data.content.as_str())
        .block(Block::default().borders(Borders::ALL).title("Path"));
    frame.render_widget(input_widget, chunks[1]);

    let cx = chunks[1].x + 1 + (input_data.cursor as u16).min(chunks[1].width - 3);
    frame.set_cursor(cx, chunks[1].y + 1);

    let help = Paragraph::new("Enter: Save | Esc: Cancel")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[2]);

    frame.render_widget(block, popup_area);
}

fn render_confirm_popup(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let popup_area = centered_rect(60, 18, area);
    frame.render_widget(Clear, popup_area);
    let block = Block::default().borders(Borders::ALL).title("Confirm Change");

    let message = if let Some(pending) = &app.pending_confirm {
        match pending.action {
            ConfirmAction::MoveVault => format!(
                "Move vault data from:\n{}\n\nto:\n{}\n\nProceed?",
                app.current_vault_path,
                pending.target.to_string_lossy()
            ),
            ConfirmAction::SwitchVault => format!(
                "Switch vault location to:\n{}\n\nProceed?",
                pending.target.to_string_lossy()
            ),
        }
    } else {
        "No pending action.".to_string()
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(4), Constraint::Length(2)].as_ref())
        .margin(1)
        .split(popup_area);

    let text = Paragraph::new(message)
        .wrap(Wrap { trim: true });
    frame.render_widget(text, chunks[0]);

    let help = Paragraph::new("y: Confirm | n/Esc: Cancel")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[1]);

    frame.render_widget(block, popup_area);
}

fn render_manual_capture_popup(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let popup_area = centered_rect(70, 22, area);
    frame.render_widget(Clear, popup_area);
    let block = Block::default().borders(Borders::ALL).title("Manual Capture");

    let label = match app
        .manual_capture
        .as_ref()
        .map(|capture| capture.step)
        .unwrap_or(CaptureStep::Title)
    {
        CaptureStep::Title => "Title",
        CaptureStep::Rationale => "Rationale",
        CaptureStep::Command => "Reproduction Command",
        CaptureStep::Tags => "Tags (comma separated)",
        CaptureStep::EntryType => "Entry Type (package/config/application/script/other)",
        CaptureStep::Verification => "Verification (optional)",
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(2),
                Constraint::Length(3),
                Constraint::Length(2),
            ]
            .as_ref(),
        )
        .margin(1)
        .split(popup_area);

    let text = Paragraph::new(label).wrap(Wrap { trim: true });
    frame.render_widget(text, chunks[0]);

    let input_widget = Paragraph::new(app.input.content.as_str())
        .block(Block::default().borders(Borders::ALL).title(label));
    frame.render_widget(input_widget, chunks[1]);

    let cx = chunks[1].x + 1 + (app.input.cursor as u16).min(chunks[1].width - 3);
    frame.set_cursor(cx, chunks[1].y + 1);

    let help = Paragraph::new("Enter: Next | Esc: Cancel")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[2]);

    frame.render_widget(block, popup_area);
}



fn render_init_popup(frame: &mut ratatui::Frame, area: Rect, input_data: &TextInput) {
    let popup_area = centered_rect(60, 20, area);
    frame.render_widget(Clear, popup_area);
    let block = Block::default().borders(Borders::ALL).title("Initialize SetupVault");
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(2),
                Constraint::Length(3),
                Constraint::Length(2),
            ]
            .as_ref(),
        )
        .margin(1)
        .split(popup_area);

    let text = Paragraph::new("SetupVault is not initialized. Please confirm the vault location:")
        .wrap(Wrap { trim: true });
    frame.render_widget(text, chunks[0]);

    let input_widget = Paragraph::new(input_data.content.as_str())
        .block(Block::default().borders(Borders::ALL).title("Path"));
    frame.render_widget(input_widget, chunks[1]);

    // Cursor for Init (single line usually)
    let cx = chunks[1].x + 1 + (input_data.cursor as u16).min(chunks[1].width - 3);
    frame.set_cursor(cx, chunks[1].y + 1);
    
    let help = Paragraph::new("Enter: Initialize | Esc: Reset")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[2]);
    
    frame.render_widget(block, popup_area);
}

fn render_guide_bar(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let hints = get_key_hints(app);
    let spans: Vec<Span> = hints
        .iter()
        .flat_map(|(key, desc)| {
            vec![
                Span::styled(format!(" [{}] ", key), Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan)),
                Span::raw(format!("{}  ", desc)),
            ]
        })
        .collect();
    
    let guide = Paragraph::new(Line::from(spans))
        .block(Block::default().borders(Borders::ALL).title("Guide"));
    frame.render_widget(guide, area);
}

fn get_key_hints(app: &App) -> Vec<(&'static str, &'static str)> {
    if matches!(app.input_mode, InputMode::Init) {
        return vec![("Enter", "Initialize"), ("Esc", "Reset")];
    }
    if matches!(app.input_mode, InputMode::Rationale) {
        return vec![("Enter", "Submit"), ("Esc", "Cancel")];
    }
    if matches!(app.input_mode, InputMode::ManualCapture) {
        return vec![("Enter", "Next"), ("Esc", "Cancel")];
    }
    if matches!(app.input_mode, InputMode::SettingsPath) {
        return vec![("Enter", "Save"), ("Esc", "Cancel")];
    }
    if matches!(app.input_mode, InputMode::Confirm) {
        return vec![("y", "Confirm"), ("n", "Cancel")];
    }
    if matches!(app.input_mode, InputMode::Palette) {
        return vec![("Enter", "Run"), ("Esc", "Close")];
    }
    if app.show_help {
        return vec![("?", "Close Help")];
    }

    let mut hints = vec![("q", "Quit"), ("?", "Help"), ("p", "Cmds")];

    match app.tab {
        Tab::Dashboard => {
            hints.extend_from_slice(&[("←/→", "Tabs"), ("r", "Refresh"), ("c", "Capture")]);
        }
        Tab::Inbox => {
            hints.extend_from_slice(&[("←/→", "Tabs"), ("h/l", "Src"), ("↑/↓", "Nav"), ("/", "Filter"), ("Space", "Select"), ("c", "Capture")]);
            if app.focus == Focus::List {
                hints.extend_from_slice(&[("a", "Approve"), ("s", "Snooze"), ("d", "Ignore"), ("Enter", "Detail")]);
            } else {
                hints.extend_from_slice(&[("Tab", "Focus List")]);
            }
        }
        Tab::Snoozed => {
            hints.extend_from_slice(&[("←/→", "Tabs"), ("↑/↓", "Nav"), ("c", "Capture")]);
            if app.focus == Focus::List {
                hints.extend_from_slice(&[("u", "Unsnooze"), ("x", "Remove"), ("Enter", "Detail")]);
            } else {
                hints.extend_from_slice(&[("Tab", "Focus List")]);
            }
        }
        Tab::Library => {
            hints.extend_from_slice(&[("←/→", "Tabs"), ("h/l", "Src"), ("↑/↓", "Nav"), ("/", "Filter"), ("c", "Capture")]);
            if app.focus == Focus::List {
                hints.extend_from_slice(&[("e", "Edit Rationale"), ("x", "Remove"), ("Enter", "Detail")]);
            } else {
                hints.extend_from_slice(&[("Tab", "Focus List")]);
            }
        }
        Tab::Settings => {
            hints.extend_from_slice(&[("←/→", "Tabs"), ("e", "Edit Path"), ("m", "Move"), ("a", "Apply"), ("c", "Capture")]);
        }
    }
    hints
}

fn render_help_popup(frame: &mut ratatui::Frame, area: Rect, content: &str) {
    let popup_area = centered_rect(70, 30, area);
    frame.render_widget(Clear, popup_area);
    let block = Block::default().borders(Borders::ALL).title("Help");
    let help = Paragraph::new(content).block(block).wrap(Wrap { trim: true });
    frame.render_widget(help, popup_area);
}

fn render_palette_popup(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let popup_area = centered_rect(80, 50, area);
    frame.render_widget(Clear, popup_area);
    let block = Block::default().borders(Borders::ALL).title("Command Palette");
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(popup_area);

    let query = Paragraph::new(format!("> {}", app.palette_input.content))
        .block(Block::default().borders(Borders::ALL).title("Search"));
    frame.render_widget(query, chunks[0]);

    let cx = chunks[0].x + 3 + (app.palette_input.cursor as u16).min(chunks[0].width - 5);
    frame.set_cursor(cx, chunks[0].y + 1);

    let items = filtered_commands(app)
        .iter()
        .map(|command| {
            let line = format!("{} — {}", command.name, command.description);
            ListItem::new(line)
        })
        .collect::<Vec<_>>();
    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(Color::DarkGray));
    frame.render_stateful_widget(list, chunks[1], &mut app.palette_state.clone());
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

fn restore_terminal(mut terminal: Terminal<ratatui::backend::CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn help_text(app: &App) -> String {
    match app.tab {
        Tab::Dashboard => {
            "c: manual capture\nr: refresh inbox\nleft/right: switch tabs\np: command palette\nq: quit".into()
        }
        Tab::Inbox => {
            "a: accept\ns: snooze\nd: ignore\nspace: select\nc: manual capture\nr: refresh\np: command palette\n/: filter\nh/l: filter source\ntab: focus list/detail".into()
        }
        Tab::Snoozed => {
             "u: unsnooze\nx: remove\nc: manual capture\n↑/↓: navigate\nleft/right: switch tabs\ntab: focus list/detail".into()
        }
        Tab::Library => {
            "e: edit rationale\nspace: select\nc: manual capture\np: command palette\n/: filter\nh/l: filter source\ntab: focus list/detail\nleft/right: switch tabs".into()
        }
        Tab::Settings => {
            "e: edit path\nm: apply & move\na: apply without move\nc: manual capture\nleft/right: switch tabs\np: command palette\nq: quit".into()
        }
    }
}



fn diff_changes(previous: &[DetectedChange], current: &[DetectedChange]) -> Vec<DetectedChange> {
    let previous_keys: std::collections::HashSet<_> = previous
        .iter()
        .map(|change| (change.source.clone(), change.title.clone()))
        .collect();
    current
        .iter()
        .filter(|change| !previous_keys.contains(&(change.source.clone(), change.title.clone())))
        .cloned()
        .collect()
}

fn append_unique(target: &mut Vec<DetectedChange>, incoming: Vec<DetectedChange>) {
    let mut seen: std::collections::HashSet<_> = target
        .iter()
        .map(|change| (change.source.clone(), change.title.clone()))
        .collect();
    for change in incoming {
        let key = (change.source.clone(), change.title.clone());
        if seen.insert(key) {
            target.push(change);
        }
    }
}

fn group_by_source(
    changes: &[DetectedChange],
) -> std::collections::BTreeMap<String, Vec<DetectedChange>> {
    let mut map = std::collections::BTreeMap::new();
    for change in changes {
        map.entry(change.source.clone())
            .or_insert_with(Vec::new)
            .push(change.clone());
    }
    map
}

#[derive(Debug, Clone, Copy)]
enum CommandAction {
    TabDashboard,
    TabInbox,
    TabSnoozed,
    TabLibrary,
    TabSettings,
    Refresh,
    Accept,
    Snooze,
    Ignore,
    EditRationale,
    EditVaultPath,
    ApplyVaultMove,
    ApplyVaultSwitch,
    ManualCapture,
    ToggleSelection,
    ToggleHelp,
    Quit,
    Remove,
    Filter,
    SnoozeQuery,
    Unsnooze,
    ClearFilter,
    ClearSelection,
    NextSource,
    PrevSource,
    ToggleFocus,
    MoveTop,
    MoveBottom,
}

#[derive(Debug, Clone)]
struct PaletteCommand {
    name: String,
    description: String,
    action: CommandAction,
}

fn build_commands() -> Vec<PaletteCommand> {
    vec![
        PaletteCommand {
            name: "Refresh Inbox".into(),
            description: "Run detectors and update inbox".into(),
            action: CommandAction::Refresh,
        },
        PaletteCommand {
            name: "Go to Dashboard".into(),
            description: "Switch to the dashboard tab".into(),
            action: CommandAction::TabDashboard,
        },
        PaletteCommand {
            name: "Go to Inbox".into(),
            description: "Switch to the inbox tab".into(),
            action: CommandAction::TabInbox,
        },
        PaletteCommand {
            name: "Go to Snoozed".into(),
            description: "Switch to the snoozed tab".into(),
            action: CommandAction::TabSnoozed,
        },
        PaletteCommand {
            name: "Go to Library".into(),
            description: "Switch to the library tab".into(),
            action: CommandAction::TabLibrary,
        },
        PaletteCommand {
            name: "Go to Settings".into(),
            description: "Switch to the settings tab".into(),
            action: CommandAction::TabSettings,
        },
        PaletteCommand {
            name: "Accept Change".into(),
            description: "Approve selected inbox item".into(),
            action: CommandAction::Accept,
        },
        PaletteCommand {
            name: "Snooze Change".into(),
            description: "Snooze selected inbox item".into(),
            action: CommandAction::Snooze,
        },
        PaletteCommand {
            name: "Ignore Change".into(),
            description: "Ignore selected inbox item".into(),
            action: CommandAction::Ignore,
        },
        PaletteCommand {
            name: "Edit Rationale".into(),
            description: "Edit rationale for selected entry".into(),
            action: CommandAction::EditRationale,
        },
        PaletteCommand {
            name: "Edit Vault Path".into(),
            description: "Update the pending vault directory".into(),
            action: CommandAction::EditVaultPath,
        },
        PaletteCommand {
            name: "Apply Vault Move".into(),
            description: "Move vault data to the pending path".into(),
            action: CommandAction::ApplyVaultMove,
        },
        PaletteCommand {
            name: "Apply Vault Switch".into(),
            description: "Switch vault location without moving data".into(),
            action: CommandAction::ApplyVaultSwitch,
        },
        PaletteCommand {
            name: "Manual Capture".into(),
            description: "Create a manual entry".into(),
            action: CommandAction::ManualCapture,
        },
        PaletteCommand {
            name: "Remove".into(),
            description: "Remove selected library entry".into(),
            action: CommandAction::Remove,
        },
        PaletteCommand {
            name: "Toggle Selection".into(),
            description: "Toggle selection checkbox".into(),
            action: CommandAction::ToggleSelection,
        },
        PaletteCommand {
            name: "Toggle Help".into(),
            description: "Show or hide help overlay".into(),
            action: CommandAction::ToggleHelp,
        },
        PaletteCommand {
            name: "Filter".into(),
            description: "Filter list items".into(),
            action: CommandAction::Filter,
        },
        PaletteCommand {
            name: "Snooze by Query".into(),
            description: "Snooze inbox items matching a query".into(),
            action: CommandAction::SnoozeQuery,
        },
        PaletteCommand {
            name: "Quit".into(),
            description: "Exit the application".into(),
            action: CommandAction::Quit,
        },
        PaletteCommand {
            name: "Unsnooze".into(),
            description: "Restore selected snoozed item to inbox".into(),
            action: CommandAction::Unsnooze,
        },
        PaletteCommand {
            name: "Clear Filter".into(),
            description: "Remove the active search filter".into(),
            action: CommandAction::ClearFilter,
        },
        PaletteCommand {
            name: "Clear Selection".into(),
            description: "Deselect all items in the current view".into(),
            action: CommandAction::ClearSelection,
        },
        PaletteCommand {
            name: "Next Source".into(),
            description: "Switch to the next source filter".into(),
            action: CommandAction::NextSource,
        },
        PaletteCommand {
            name: "Previous Source".into(),
            description: "Switch to the previous source filter".into(),
            action: CommandAction::PrevSource,
        },
        PaletteCommand {
            name: "Switch Focus".into(),
            description: "Toggle focus between list and details".into(),
            action: CommandAction::ToggleFocus,
        },
        PaletteCommand {
            name: "Move to Top".into(),
            description: "Go to the first item in the list".into(),
            action: CommandAction::MoveTop,
        },
        PaletteCommand {
            name: "Move to Bottom".into(),
            description: "Go to the last item in the list".into(),
            action: CommandAction::MoveBottom,
        },
    ]
}

fn filtered_commands(app: &App) -> Vec<PaletteCommand> {
    let query = app.palette_input.content.to_lowercase();
    app.commands
        .iter()
        .filter(|command| {
            let available = match command.action {
                CommandAction::SnoozeQuery => {
                    app.tab == Tab::Inbox
                }
                CommandAction::Accept | CommandAction::Snooze | CommandAction::Ignore => {
                    app.tab == Tab::Inbox && app.focus == Focus::List
                }
                CommandAction::Remove => {
                    (app.tab == Tab::Library || app.tab == Tab::Snoozed) && app.focus == Focus::List
                }
                CommandAction::Unsnooze => {
                    app.tab == Tab::Snoozed && app.focus == Focus::List
                }
                CommandAction::EditRationale => {
                    app.tab == Tab::Library && app.focus == Focus::List
                }
                CommandAction::EditVaultPath
                | CommandAction::ApplyVaultMove
                | CommandAction::ApplyVaultSwitch => {
                    app.tab == Tab::Settings
                }
                CommandAction::ManualCapture => true,
                CommandAction::Refresh => {
                    matches!(app.tab, Tab::Dashboard | Tab::Inbox)
                }
                CommandAction::ToggleSelection => {
                    matches!(app.tab, Tab::Inbox | Tab::Library | Tab::Snoozed)
                }
                CommandAction::Filter => {
                     matches!(app.tab, Tab::Inbox | Tab::Library | Tab::Snoozed)
                }
                CommandAction::ClearFilter => {
                    app.active_filter.is_some()
                }
                CommandAction::ClearSelection => {
                    match app.tab {
                        Tab::Inbox => !app.selected_inbox.is_empty(),
                        Tab::Library => !app.selected_library.is_empty(),
                        Tab::Snoozed => !app.selected_snoozed.is_empty(),
                        _ => false,
                    }
                }
                CommandAction::NextSource | CommandAction::PrevSource => {
                    matches!(app.tab, Tab::Inbox | Tab::Library)
                }
                CommandAction::ToggleFocus => {
                    app.tab != Tab::Dashboard && app.tab != Tab::Settings
                }
                CommandAction::MoveTop | CommandAction::MoveBottom => {
                    app.tab != Tab::Dashboard && app.tab != Tab::Settings
                }
                _ => true,
            };

            if !available {
                return false;
            }

            command.name.to_lowercase().contains(&query)
                || command.description.to_lowercase().contains(&query)
        })
        .cloned()
        .collect()
}

fn open_palette(app: &mut App) {
    app.input_mode = InputMode::Palette;
    app.palette_input.reset();
    app.palette_state.select(Some(0));
}

fn close_palette(app: &mut App) {
    app.input_mode = InputMode::None;
    app.palette_input.reset();
    app.palette_state.select(None);
}

fn execute_command(vault: &FsVault, app: &mut App, action: CommandAction) -> Result<()> {
    match action {
        CommandAction::TabDashboard => app.tab = Tab::Dashboard,
        CommandAction::TabInbox => app.tab = Tab::Inbox,
        CommandAction::TabSnoozed => app.tab = Tab::Snoozed,
        CommandAction::TabLibrary => app.tab = Tab::Library,
        CommandAction::TabSettings => app.tab = Tab::Settings,
        CommandAction::Refresh => handle_refresh(vault, app)?,
        CommandAction::Accept => handle_accept(app),
        CommandAction::Snooze => handle_snooze(vault, app)?,
        CommandAction::Ignore => handle_ignore(vault, app)?,
        CommandAction::EditRationale => handle_edit_rationale(app),
        CommandAction::EditVaultPath => {
            if app.tab == Tab::Settings {
                open_settings_path_input(app);
            }
        }
        CommandAction::ApplyVaultMove => {
            if app.tab == Tab::Settings {
                confirm_settings_change(app, ConfirmAction::MoveVault);
            }
        }
        CommandAction::ApplyVaultSwitch => {
            if app.tab == Tab::Settings {
                confirm_settings_change(app, ConfirmAction::SwitchVault);
            }
        }
        CommandAction::ManualCapture => open_manual_capture(app),
        CommandAction::ToggleSelection => toggle_selection(app),
        CommandAction::ToggleHelp => app.show_help = !app.show_help,
        CommandAction::Quit => app.status = Some("Use q to quit".into()),
        CommandAction::Remove => handle_remove(vault, app)?,
        CommandAction::Filter => {
             if matches!(app.tab, Tab::Inbox | Tab::Library | Tab::Snoozed) {
                 app.input_mode = InputMode::Filter;
                 app.filter_input.reset();
                 if let Some(current) = &app.active_filter {
                      app.filter_input = TextInput::from(current.clone());
                 }
             }
        }
        CommandAction::SnoozeQuery => {
             if app.tab == Tab::Inbox {
                  app.input_mode = InputMode::SnoozeQuery;
                  app.input.reset();
             }
        }
        CommandAction::Unsnooze => handle_unsnooze(vault, app)?,
        CommandAction::ClearFilter => {
            app.active_filter = None;
            app.filter_input.reset();
        }
        CommandAction::ClearSelection => {
            match app.tab {
                Tab::Inbox => app.selected_inbox.clear(),
                Tab::Library => app.selected_library.clear(),
                Tab::Snoozed => app.selected_snoozed.clear(),
                _ => {}
            }
        }
        CommandAction::NextSource => {
            if app.tab == Tab::Inbox {
                app.next_source();
            } else if app.tab == Tab::Library {
                app.next_library_source();
            }
        }
        CommandAction::PrevSource => {
            if app.tab == Tab::Inbox {
                app.prev_source();
            } else if app.tab == Tab::Library {
                app.prev_library_source();
            }
        }
        CommandAction::ToggleFocus => {
            if app.tab != Tab::Dashboard && app.tab != Tab::Settings {
                app.toggle_focus();
            }
        }
        CommandAction::MoveTop => handle_list_move(app, Move::First),
        CommandAction::MoveBottom => handle_list_move(app, Move::Last),
    }
    Ok(())
}

fn handle_remove(vault: &FsVault, app: &mut App) -> Result<()> {
    if app.tab == Tab::Library {
        let ids_to_remove: Vec<uuid::Uuid> = if !app.selected_library.is_empty() {
            app.selected_library.iter().cloned().collect()
        } else {
            current_library_id(app).into_iter().collect()
        };

        if ids_to_remove.is_empty() {
            return Ok(());
        }

        for id in &ids_to_remove {
            vault.restore_to_inbox(*id)?;
            if let Some(real_index) = app.library.iter().position(|e| e.id == *id) {
                app.library.remove(real_index);
            }
        }

        app.inbox = vault.load_inbox().unwrap_or_default();
        app.selected_library.clear();
        app.status = Some(format!("Removed {} item(s) and restored to inbox", ids_to_remove.len()));

        // Adjust selection
        let filtered_len = app.filtered_library().len();
        if let Some(selected) = app.library_state.selected() {
             if selected >= filtered_len && filtered_len > 0 {
                app.library_state.select(Some(filtered_len - 1));
            } else if filtered_len == 0 {
                app.library_state.select(None);
            }
        }
    } else if app.tab == Tab::Snoozed {
        let ids_to_remove: Vec<uuid::Uuid> = if !app.selected_snoozed.is_empty() {
            app.selected_snoozed.iter().cloned().collect()
        } else {
            current_snoozed_id(app).into_iter().collect()
        };

        if ids_to_remove.is_empty() {
            return Ok(());
        }

        for id in &ids_to_remove {
            vault.remove_snoozed_item(*id)?;
            if let Some(pos) = app.snoozed.iter().position(|item| item.id == *id) {
                app.snoozed.remove(pos);
            }
        }

        app.selected_snoozed.clear();
        app.status = Some(format!("Removed {} snoozed item(s)", ids_to_remove.len()));

        let len = app.filtered_snoozed().len();
        if let Some(selected) = app.snoozed_state.selected() {
            if selected >= len && len > 0 {
                app.snoozed_state.select(Some(len - 1));
            } else if len == 0 {
                app.snoozed_state.select(None);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use sv_core::{EntryType, SystemInfo, Tag};

    #[test]
    fn render_snapshot() {
        let backend = TestBackend::new(60, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let mut app = App::new();
        app.tab = Tab::Inbox;
        app.inbox = vec![DetectedChange {
            id: uuid::Uuid::new_v4(),
            path: None,
            title: "jq".into(),
            entry_type: EntryType::Package,
            source: "homebrew".into(),
            cmd: "brew install jq".into(),
            system: SystemInfo {
                os: "macos".into(),
                arch: "arm64".into(),
            },
            detected_at: chrono::Utc::now(),
            tags: vec![Tag::new("cli").unwrap()],
        }];
        app.inbox_state.select(Some(0));

        terminal
            .draw(|frame| render_app(frame, &app))
            .expect("render");

        let buffer = terminal.backend().buffer();
        let snapshot = buffer_to_string(buffer);
        insta::assert_snapshot!(snapshot);
    }

    fn buffer_to_string(buffer: &ratatui::buffer::Buffer) -> String {
        let mut lines = Vec::new();
        for y in 0..buffer.area.height {
            let mut line = String::new();
            for x in 0..buffer.area.width {
                let cell = buffer.get(x, y);
                line.push_str(cell.symbol());
            }
            lines.push(line.trim_end().to_string());
        }
        lines.join("\n")
    }
}

fn handle_snooze_query(vault: &mut FsVault, app: &mut App, key: KeyEvent) -> Result<bool> {
    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::None;
            app.input.reset();
        }
        KeyCode::Enter => {
            let query = app.input.content.to_lowercase();
            if !query.is_empty() {
                let to_snooze: Vec<_> = app.inbox.iter()
                    .filter(|item| item.title.to_lowercase().contains(&query) 
                                || item.source.to_lowercase().contains(&query))
                    .map(|item| item.id)
                    .collect();

                let count = to_snooze.len();
                for id in to_snooze {
                    vault.snooze_inbox_item(id)?;
                    app.inbox.retain(|item| item.id != id);
                }
                app.status = Some(format!("Snoozed {} items matching '{}'", count, query));
            }
            app.input_mode = InputMode::None;
            app.input.reset();
        }
        KeyCode::Char(c) => app.input.insert(c),
        KeyCode::Backspace => app.input.delete_back(),
        KeyCode::Left => app.input.move_left(),
        KeyCode::Right => app.input.move_right(),
        KeyCode::Home => app.input.move_home(),
        KeyCode::End => app.input.move_end(),
        _ => {}
    }
    Ok(false)
}

fn render_snooze_popup(frame: &mut ratatui::Frame, area: Rect, input_data: &TextInput) {
    let popup_area = centered_rect(60, 20, area);
    frame.render_widget(Clear, popup_area);
    let block = Block::default().borders(Borders::ALL).title("Snooze Matching Items");
    let input_widget = Paragraph::new(input_data.content.as_str())
        .block(block)
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(Color::Yellow));
    frame.render_widget(input_widget, popup_area);
    
    let cx = popup_area.x + 1 + (input_data.cursor as u16).min(popup_area.width - 2);
    let cy = popup_area.y + 1;
    frame.set_cursor(cx, cy);
}
