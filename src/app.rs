use crate::counter::{Counter, CounterStore};
use crate::debugging::DebugInfo;
use crate::input::{self, EventHandler, EventType, HandlerMode, Key, ThreadError};
use crate::settings::{KeyMap, Settings};
use crate::ui::{self, UiWidth};
use crate::widgets::entry::EntryState;
use crate::{errplace, settings, SAVE_FILE};
use bitflags::bitflags;
use core::sync::atomic::AtomicI32;
use crossterm::event::KeyModifiers;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use nix::errno::Errno;
use std::cell::{Ref, RefCell, RefMut};
use std::error::Error;
use std::io;
use std::sync::{MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};
use tui::{backend::CrosstermBackend, widgets::ListState, Terminal};
use Dialog as DS;
use EditingState as ES;

#[derive(Debug)]
pub enum AppError {
    GetCounterError(String),
    GetPhaseError,
    DevIoError(String),
    IoError(String),
    SettingNotFound,
    InputThread,
    ThreadError(ThreadError),
    ImpossibleState(String),
    ScreenSize(String),
    DialogAlreadyOpen(String),
    EventEmpty(String),
    SettingsType(String),
}

impl Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str_ = match self {
            AppError::GetCounterError(_) => "GetCounterError".to_string(),
            AppError::GetPhaseError => "GetPhaseError".to_string(),
            AppError::DevIoError(_) => "DevIoError".to_string(),
            AppError::IoError(_) => "IoError".to_string(),
            AppError::SettingNotFound => "SettingNotFound".to_string(),
            AppError::InputThread => "InputThread".to_string(),
            AppError::ThreadError(_) => "ThreadError".to_string(),
            AppError::ImpossibleState(_) => "ImpossibleState".to_string(),
            AppError::ScreenSize(_) => "ScreenSize".to_string(),
            AppError::DialogAlreadyOpen(_) => "DialogAlreadyOpen".to_string(),
            AppError::EventEmpty(_) => "EventEmpty".to_string(),
            AppError::SettingsType(_) => "SettingsType".to_string(),
        };
        write!(f, "{}", str_)
    }
}

impl From<io::Error> for AppError {
    fn from(e: io::Error) -> Self {
        Self::IoError(format!("{}, {}", errplace!(), e))
    }
}

impl From<ThreadError> for AppError {
    fn from(value: ThreadError) -> Self {
        Self::ThreadError(value)
    }
}

impl From<Errno> for AppError {
    fn from(e: Errno) -> Self {
        Self::DevIoError(e.to_string())
    }
}

impl From<PoisonError<MutexGuard<'_, AtomicI32>>> for AppError {
    fn from(_: PoisonError<MutexGuard<'_, AtomicI32>>) -> Self {
        Self::InputThread
    }
}

bitflags! {
    pub struct AppMode: u16 {
        const SELECTION      = 0b0000_0000_0001;
        const PHASE_SELECT   = 0b0000_0000_0010;
        const COUNTING       = 0b0000_0000_0100;
        const KEYLOGGING     = 0b0000_0000_1000;

        const DIALOG_OPEN    = 0b0000_0001_0000;
        const SETTINGS_OPEN  = 0b0000_0010_0000;

        const DEBUGGING      = 0b1000_0000_0000;
    }
}

impl Default for AppMode {
    fn default() -> Self {
        Self::SELECTION
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dialog {
    AddNew,
    Editing(EditingState),
    Delete,
    None,
}

impl Default for Dialog {
    fn default() -> Self {
        return Self::None;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditingState {
    Rename,
    ChCount,
    ChTime,
}

pub struct App {
    pub state: AppState,
    pub c_store: CounterStore,
    pub ui_size: UiWidth,
    last_interaction: Instant,
    running: bool,
    cursor_pos: Option<(u16, u16)>,
    pub event_handler: EventHandler,
    pub debugging: DebugInfo,
    pub settings: Settings,
    pub key_map: KeyMap,
}

impl App {
    pub fn new(counter_store: CounterStore) -> Self {
        App {
            state: AppState::new(2),
            last_interaction: Instant::now(),
            c_store: counter_store,
            ui_size: UiWidth::Big,
            running: true,
            cursor_pos: None,
            event_handler: EventHandler::default(),
            debugging: DebugInfo::default(),
            settings: Settings::new(),
            key_map: KeyMap::default(),
        }
    }
    pub fn set_super_user(self, input_fd: i32) -> Self {
        self.event_handler.set_fd(input_fd).unwrap();
        self
    }
    pub fn start(mut self) -> Result<App, AppError> {
        // setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        self.event_handler.start()?;
        // update the settings menu with the correct infomation
        self.settings.load_keyboards()?;

        self.list_select(0, Some(0));

        let mut previous_time = Instant::now();
        let mut now_time: Instant;

        self.debugging.add_debug_message(
            "dev_input_files",
            input::get_kbd_inputs()?
                .into_iter()
                .map(|value| value + ", ")
                .collect::<String>(),
        );

        while self.running {
            match self.handle_events() {
                Ok(_) => {}
                Err(e) => self.debugging.handle_error(e),
            };
            // timing the execution time of the loop and add it to the counter time
            // only do this in counting mode
            now_time = Instant::now();
            if self.get_mode().intersects(AppMode::COUNTING) {
                self.get_mut_act_counter()?
                    .increase_time(now_time - previous_time);
            }
            previous_time = Instant::now();

            let terminal_start_time = Instant::now();

            // draw all ui elements
            terminal.draw(|f| {
                // TODO: factor out these unwraps make them fatal errors but clean up screen first
                match ui::draw(f, &mut self) {
                    Ok(_) => {}
                    Err(e) => self.debugging.handle_error(e),
                }
                // if settings are open draw on top
                if self.get_mode().intersects(AppMode::SETTINGS_OPEN) {
                    match settings::draw_as_overlay(f, &self.settings) {
                        Ok(_) => {}
                        Err(e) => self.debugging.handle_error(e),
                    }
                }
            })?;

            self.debugging.add_debug_message(
                "draw_time",
                format!("{:?}", Instant::now() - terminal_start_time),
            );
            self.debugging.add_debug_message(
                "key_event",
                format!("{:?}", self.event_handler.get_buffer()),
            );

            if self.settings.get_tick_time()? > (Instant::now() - now_time) {
                thread::sleep(self.settings.get_tick_time()? - (Instant::now() - now_time));
            }
        }
        Ok(self)
    }

    pub fn get_act_counter(&self) -> Result<Ref<Counter>, AppError> {
        let selection = self.get_list_state(1).selected().unwrap_or(0);
        if let Some(counter) = self.c_store.get(selection) {
            Ok(counter)
        } else {
            Err(AppError::GetCounterError(errplace!()))
        }
    }

    pub fn get_mut_act_counter(&self) -> Result<RefMut<Counter>, AppError> {
        let selection = self.get_list_state(0).selected().unwrap_or(0);
        if let Some(counter) = self.c_store.get_mut(selection) {
            Ok(counter)
        } else {
            Err(AppError::GetCounterError(errplace!()))
        }
    }

    pub fn get_act_phase_name(&self) -> Result<String, AppError> {
        let selected = self.get_list_state(1).selected().unwrap_or(0);
        self.get_act_counter()?
            .get_phase(selected)
            .map(|p| p.get_name())
            .ok_or(AppError::GetPhaseError)
    }

    pub fn get_act_phase_count(&self) -> Result<i32, AppError> {
        let selected = self.get_list_state(1).selected().unwrap_or(0);
        self.get_act_counter()?
            .get_phase(selected)
            .map(|p| p.get_count())
            .ok_or(AppError::GetPhaseError)
    }

    pub fn get_act_phase_time(&self) -> Result<Duration, AppError> {
        let selected = self.get_list_state(1).selected().unwrap_or(0);
        self.get_act_counter()?
            .get_phase(selected)
            .map(|p| p.get_time())
            .ok_or(AppError::GetPhaseError)
    }

    pub fn end(&self) -> io::Result<CounterStore> {
        enable_raw_mode()?;
        let backend = CrosstermBackend::new(io::stdout());
        let mut terminal = Terminal::new(backend).unwrap();
        // restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;
        Ok(self.c_store.clone())
    }

    /// Returns the [mode](App::state::mode) of [`App`].
    pub fn get_mode(&self) -> AppMode {
        self.state.mode.clone().into_inner()
    }

    /// Sets the [mode](App::state::mode) of [`App`].
    ///
    /// WARNING this function overrides all other app modes
    /// try using [`toggle_mode`] instead
    pub fn set_mode(&self, mode: AppMode) {
        self.state.set_mode(mode)
    }

    /// Toggles the [mode](App::state::mode) of [`App`].
    ///
    /// This function preserves all other selected app modes
    /// and only flips the passed appmode on or off
    pub fn toggle_mode(&self, mode: AppMode) {
        self.state.toggle_mode(mode)
    }

    /// Sets the [mode](App::state::mode) of [`App`]
    /// back to the default mode
    /// this is [AppMode::SELECTION]
    pub fn reset_mode(&self) {
        self.state.set_mode(AppMode::SELECTION)
    }

    /// Opens a `dialog`: [`DialogState`]
    /// Also set the `mode` of `App` to `AppMode::DIALOG_OPEN`
    ///
    /// Opening multiple dialogs at once will result in an error
    pub fn open_dialog(&mut self, dialog: Dialog) -> Result<(), AppError> {
        if self.get_mode().intersects(AppMode::DIALOG_OPEN) {
            return Err(AppError::DialogAlreadyOpen(format!(
                "{:?}",
                self.get_opened_dialog()
            )));
        }
        self.state.new_entry("");
        self.toggle_mode(AppMode::DIALOG_OPEN);
        self.state.dialog = dialog;
        Ok(())
    }

    /// Close any opened dialog
    /// If no dialog was open when running this function nothing will happen
    pub fn close_dialog(&mut self) {
        self.state.dialog = Dialog::None;
        self.state.clear_entry();
        self.toggle_mode(AppMode::DIALOG_OPEN);
    }

    /// returns a borrow of the dialog currently opened
    pub fn get_opened_dialog(&self) -> &Dialog {
        return &self.state.dialog;
    }

    pub fn get_list_state(&self, index: usize) -> &ListState {
        return self.state.list_states.get(index).unwrap();
    }

    pub fn get_mut_list_state(&mut self, index: usize) -> &mut ListState {
        return self.state.list_states.get_mut(index).unwrap();
    }

    pub fn list_select(&mut self, index: usize, select_index: Option<usize>) {
        self.state.list_states[index].select(select_index)
    }

    pub fn list_deselect(&mut self, index: usize) {
        self.state.list_states[index].select(None)
    }

    pub fn get_entry_state(&mut self) -> &mut EntryState {
        return &mut self.state.entry_state;
    }

    pub fn reset_entry_state(&mut self) {
        self.state.entry_state = EntryState::default();
    }

    pub fn handle_events(&mut self) -> Result<(), AppError> {
        while self.event_handler.has_event() {
            self.debugging.add_debug_message("last_key", format!("{:?}", self.event_handler.get_buffer()[0]));

            if self.get_mode().intersects(AppMode::SETTINGS_OPEN) {
                self.settings
                    .handle_event(&self.state, &self.event_handler)?;
            } else {
                self.handle_event()?;
            }
        }
        Ok(())
    }

    fn handle_event(&mut self) -> Result<(), AppError> {
        let event = if let Some(event) = self.event_handler.poll() {
            event
        } else {
            return Ok(());
        };
        let key = if let EventType::KeyEvent(key) = event.clone().type_ {
            key
        } else {
            return Ok(());
        };

        if key == Key::Char('`') {
            self.toggle_mode(AppMode::DEBUGGING)
        } else if event.modifiers.intersects(KeyModifiers::CONTROL) && key == Key::Char('s') {
            self.toggle_mode(AppMode::SETTINGS_OPEN)
        }

        // parsing the state the app is in return an error when in an impossible list_states
        // otherwise directing the key to the correct input parser
        if self.get_mode().intersects(AppMode::DIALOG_OPEN) {
            if self.get_mode().intersects(AppMode::PHASE_SELECT) {
                match self.state.dialog {
                    Dialog::Delete => self.delete_phase_key_event(key)?,
                    Dialog::Editing(ES::Rename) => self.rename_phase_key_event(key)?,
                    _ => return Err(AppError::ImpossibleState(format!("{:?}", self.get_mode()))),
                }
            } else if self.get_mode().intersects(AppMode::SELECTION) {
                match self.state.dialog {
                    Dialog::AddNew => self.add_counter_key_event(key)?,
                    Dialog::Delete => self.delete_counter_key_event(key)?,
                    Dialog::Editing(_) => self.rename_key_event(key)?,
                    Dialog::None => {
                        return Err(AppError::ImpossibleState(format!("{:?}", self.get_mode())))
                    }
                }
            } else {
                return Err(AppError::ImpossibleState(format!("{:?}", self.get_mode())));
            }
        } else if self.get_mode().intersects(AppMode::COUNTING) {
            self.counter_key_event(key)?
        } else if self.get_mode().intersects(AppMode::PHASE_SELECT) {
            self.phase_select_key_event(key)?
        } else if self.get_mode().intersects(AppMode::SELECTION) {
            if self.c_store.len() > 0 {
                self.selection_key_event(key)?
            } else {
                match key {
                    Key::Char('q') => self.running = false,
                    Key::Char('n') => self.open_dialog(DS::AddNew)?,
                    _ => {}
                }
            }
        } else {
            return Err(AppError::ImpossibleState(format!("{:?}", self.get_mode())));
        }
        Ok(())
    }

    fn selection_key_event(&mut self, key: Key) -> Result<(), AppError> {
        let len = self.c_store.len();
        match key {
            Key::Char('q') | Key::Esc => self.running = false,
            Key::Char('n') => self.open_dialog(DS::AddNew)?,
            Key::Char('d') => self.open_dialog(DS::Delete)?,
            Key::Char('e') => self.open_dialog(DS::Editing(ES::Rename))?,
            Key::Char('f') => {
                let selected = self.get_list_state(1).selected().unwrap_or(0);
                self.list_select(1, Some(selected));
                self.toggle_mode(AppMode::PHASE_SELECT)
            }
            Key::Enter => {
                if self.get_act_counter()?.get_phase_len() > 1 {
                    let selected = self.get_list_state(1).selected().unwrap_or(0);
                    self.list_select(1, Some(selected));
                    self.toggle_mode(AppMode::PHASE_SELECT)
                } else {
                    self.list_select(1, Some(0));
                    self.toggle_mode(AppMode::COUNTING)
                }
            }
            Key::Up => {
                let mut selected = self.get_list_state(0).selected().unwrap_or(0);
                selected += len - 1;
                selected %= len;
                self.list_select(0, Some(selected));
            }
            Key::Down => {
                let mut selected = self.get_list_state(0).selected().unwrap_or(0);
                selected += 1;
                selected %= len;
                self.list_select(0, Some(selected));
            }
            _ => {}
        }
        Ok(())
    }

    fn counter_key_event(&mut self, key: Key) -> Result<(), AppError> {
        match key {
            key if self.key_map.key_increase_counter.contains(&key) => {
                self.get_mut_act_counter()?.increase_by(1);
                self.c_store.to_json(SAVE_FILE)
            }
            key if self.key_map.key_decrease_counter.contains(&key) => {
                self.get_mut_act_counter()?.increase_by(-1);
                self.c_store.to_json(SAVE_FILE)
            }
            key if self.key_map.key_toggle_keylogger.contains(&key) => {
                match self.event_handler.set_kbd(&self.settings.get_kbd_input()?) {
                    Ok(_) => {
                        self.event_handler.toggle_mode();
                        self.toggle_mode(AppMode::KEYLOGGING)
                    }
                    Err(e) => return Err(e),
                };
            }
            Key::Char('q') | Key::Esc => {
                self.event_handler.set_mode(HandlerMode::Terminal);
                if self.get_mode().intersects(AppMode::KEYLOGGING) {
                    self.toggle_mode(AppMode::KEYLOGGING)
                }

                if !self.get_mode().intersects(AppMode::PHASE_SELECT) {
                    self.list_deselect(1)
                }

                self.toggle_mode(AppMode::COUNTING);
            }
            _ => {}
        }
        Ok(())
    }

    fn add_counter_key_event(&mut self, key: Key) -> Result<(), AppError> {
        match key {
            Key::Esc => {
                self.close_dialog();
            }
            Key::Enter => {
                let name = self.get_entry_state().get_active_field().clone();
                self.c_store.push(Counter::new(name));
                self.close_dialog();
            }
            Key::Char(charr) if charr.is_ascii() => self.get_entry_state().push(charr),
            Key::Backspace => {
                self.get_entry_state().pop();
            }
            _ => {}
        }
        Ok(())
    }

    fn delete_counter_key_event(&mut self, key: Key) -> Result<(), AppError> {
        match key {
            Key::Enter => {
                if self.c_store.len() < 1 {
                    return Err(AppError::GetCounterError(errplace!()));
                }
                self.c_store
                    .remove(self.get_list_state(0).selected().unwrap_or(0));
                self.close_dialog()
            }
            Key::Esc => self.close_dialog(),
            _ => {}
        }
        Ok(())
    }

    fn delete_phase_key_event(&mut self, _key: Key) -> Result<(), AppError> {
        todo!()
    }

    fn rename_key_event(&mut self, key: Key) -> Result<(), AppError> {
        match key {
            Key::Char(charr) if charr.is_ascii() => self.get_entry_state().push(charr),
            Key::Backspace => {
                self.get_entry_state().pop();
            }
            Key::Enter => {
                let name = self.get_entry_state().get_active_field().clone();
                self.get_mut_act_counter()?.set_name(&name);
                self.open_dialog(DS::Editing(ES::ChCount))?;
            }
            Key::Esc => {
                self.close_dialog();
            }
            _ => {}
        }
        Ok(())
    }

    fn change_count_key_event(&mut self, key: Key) -> Result<(), AppError> {
        match key {
            Key::Char(charr) if charr.is_numeric() => self.get_entry_state().push(charr),
            Key::Backspace => {
                self.get_entry_state().pop();
            }
            Key::Enter => {
                let count = self
                    .get_entry_state()
                    .get_active_field()
                    .parse()
                    .unwrap_or_else(|_| self.get_act_counter().unwrap().get_count());
                self.get_mut_act_counter()?.set_count(count);
                self.open_dialog(DS::Editing(ES::ChTime))?;
            }
            Key::Esc => {
                self.close_dialog();
            }
            _ => {}
        }
        Ok(())
    }

    fn change_time_key_event(&mut self, key: Key) -> Result<(), AppError> {
        match key {
            Key::Char(charr) if charr.is_numeric() => self.get_entry_state().push(charr),
            Key::Backspace => {
                self.get_entry_state().pop();
            }
            Key::Enter => {
                let time = self
                    .get_entry_state()
                    .get_active_field()
                    .parse()
                    .unwrap_or(self.get_act_counter()?.get_time().as_secs() / 60);
                self.get_mut_act_counter()?
                    .set_time(Duration::from_secs(time * 60));
                self.close_dialog()
            }
            Key::Esc => self.close_dialog(),
            _ => {}
        }
        Ok(())
    }
    fn rename_phase_key_event(&mut self, key: Key) -> Result<(), AppError> {
        match key {
            Key::Char(charr) if charr.is_ascii() => self.get_entry_state().push(charr),
            Key::Backspace => self.get_entry_state().pop(),
            Key::Enter => {
                let phase = self.get_list_state(1).selected().unwrap_or(0);
                let name = self.get_entry_state().get_active_field().clone();
                self.get_mut_act_counter()?.set_phase_name(phase, name);
                self.close_dialog()
            }
            Key::Esc => self.close_dialog(),
            _ => {}
        }
        Ok(())
    }
    fn phase_select_key_event(&mut self, key: Key) -> Result<(), AppError> {
        let len = self.get_act_counter().unwrap().get_phase_len();
        match key {
            Key::Char('d') if self.get_act_counter()?.get_phase_len() == 1 => {
                self.set_mode(AppMode::SELECTION)
            }
            Key::Char('d') => self.open_dialog(DS::Delete)?,
            Key::Char('n') => self.get_mut_act_counter()?.new_phase(),
            Key::Char('r') => self.open_dialog(DS::Editing(ES::Rename))?,
            Key::Up => {
                let mut selected = self.get_list_state(1).selected().unwrap_or(0);
                selected += len - 1;
                selected %= len;
                self.list_select(1, Some(selected));
            }
            Key::Down => {
                let mut selected = self.get_list_state(1).selected().unwrap_or(0);
                selected += 1;
                selected %= len;
                self.list_select(1, Some(selected));
            }
            Key::Enter => {
                self.list_select(1, Some(0));
                self.toggle_mode(AppMode::COUNTING);
            }
            Key::Esc | Key::Char('q') => {
                self.list_deselect(1);
                self.toggle_mode(AppMode::PHASE_SELECT)
            }
            _ => {}
        }
        Ok(())
    }
}

impl Default for App {
    fn default() -> Self {
        Self {
            state: AppState::default(),
            c_store: CounterStore::default(),
            ui_size: UiWidth::Medium,
            last_interaction: Instant::now(),
            running: true,
            cursor_pos: None,
            event_handler: EventHandler::default(),
            debugging: DebugInfo::default(),
            settings: Settings::new(),
            key_map: KeyMap::default(),
        }
    }
}

#[derive(Default)]
pub struct AppState {
    mode: RefCell<AppMode>,
    dialog: Dialog,
    list_states: Vec<ListState>,
    entry_state: EntryState,
}

impl AppState {
    fn new(lists: usize) -> Self {
        Self {
            mode: RefCell::new(AppMode::default()),
            dialog: Dialog::None,
            list_states: vec![ListState::default(); lists],
            entry_state: EntryState::default(),
        }
    }

    fn get_mode(&self) -> AppMode {
        self.mode.clone().into_inner()
    }

    fn set_mode(&self, mode: AppMode) {
        self.mode.swap(&RefCell::new(mode))
    }

    pub fn toggle_mode(&self, mode: AppMode) {
        self.mode
            .swap(&RefCell::new(self.mode.clone().borrow().clone() ^ mode))
    }

    pub fn exit_mode(&self, mode: AppMode) {
        self.mode.swap(&RefCell::new(
            self.mode.clone().borrow().clone() & AppMode::complement(mode),
        ))
    }

    fn new_entry(&mut self, default_value: impl Into<String>) {
        self.entry_state.set_field(default_value)
    }

    fn clear_entry(&mut self) {
        self.entry_state = EntryState::default();
    }
}

pub fn cleanup_terminal_state() -> Result<(), AppError> {
    enable_raw_mode()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend).unwrap();
    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

#[cfg(test)]
mod test_app {
    use super::*;
    #[test]
    fn test_input_handling() {
        let mut app = App::default();
        assert!(app.handle_event().is_ok());
        app.event_handler.simulate_key(Key::Char('n'));
        app.handle_event().unwrap();
        assert_eq!(
            app.get_mode(),
            AppMode::from_bits(0b0000_0001_0001).unwrap()
        );
        app.event_handler.simulate_key(Key::Char('n'));
        app.event_handler.simulate_key(Key::Char('e'));
        app.event_handler.simulate_key(Key::Char('w'));
        app.event_handler.simulate_key(Key::Enter);
        app.handle_events().unwrap();
        assert_eq!(
            app.get_mode(),
            AppMode::from_bits(0b0000_0000_0001).unwrap()
        );
        assert_eq!(
            app.c_store.get_counters(),
            vec![RefCell::new(Counter::new("new"))]
        )
    }
    #[test]
    fn test_new_counter_dialog() {
        let mut app = App::default();
        app.event_handler.simulate_key(Key::Char('n'));
        app.handle_event().unwrap();
        app.event_handler.simulate_key(Key::Char('f'));
        app.handle_event().unwrap();
        assert_eq!(app.state.entry_state.get_active_field(), "f");
        app.event_handler.simulate_key(Key::Char('o'));
        app.event_handler.simulate_key(Key::Char('o'));
        assert_eq!(app.state.entry_state.get_active_field(), "f");
        app.handle_events().unwrap();
        assert_eq!(app.state.entry_state.get_active_field(), "foo");
        app.event_handler.simulate_key(Key::Enter);
        app.handle_events().unwrap();
        assert_eq!(app.state.entry_state.get_active_field(), "");
        assert_eq!(
            app.c_store.get_counters(),
            vec![RefCell::new(Counter::new("foo"))]
        );
    }
    #[test]
    fn test_delete_dialog() {
        let mut app = App::default();
        app.event_handler.simulate_key(Key::Char('d'));
        assert_eq!(
            app.get_mode(),
            AppMode::from_bits(0b0000_0000_0001).unwrap()
        );
        app.c_store.push(Counter::new("foo"));
        app.event_handler.simulate_key(Key::Char('d'));
        assert_eq!(
            app.get_mode(),
            AppMode::from_bits(0b0000_0001_0001).unwrap()
        );
        app.c_store.push(Counter::new("foo"));
        app.c_store.push(Counter::new("baz"));
        app.c_store.push(Counter::new("bar"));
    }
}
