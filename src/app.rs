use crate::counter::{Counter, CounterStore};
use crate::input::{DevInputFileDescriptor, EventHandler, EventType, Key, ThreadError};
use crate::ui::{self, UiWidth};
use crate::widgets::entry::EntryState;
use crate::SAVE_FILE;
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
use std::collections::HashMap;
use std::io;
use std::sync::{MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};
use tui::{backend::CrosstermBackend, widgets::ListState, Terminal};
use Dialog as DS;
use EditingState as ES;

#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
pub enum AppError {
    GetCounterError,
    GetPhaseError,
    DevIoError,
    IoError,
    ThreadError(ThreadError),
    ImpossibleState(String),
    ScreenSize(String),
}

impl From<io::Error> for AppError {
    fn from(_: io::Error) -> Self {
        Self::IoError
    }
}

impl From<ThreadError> for AppError {
    fn from(value: ThreadError) -> Self {
        Self::ThreadError(value)
    }
}

impl From<Errno> for AppError {
    fn from(_: Errno) -> Self {
        Self::DevIoError
    }
}

impl From<PoisonError<MutexGuard<'_, AtomicI32>>> for AppError {
    fn from(_: PoisonError<MutexGuard<'_, AtomicI32>>) -> Self {
        Self::DevIoError
    }
}

#[derive(Eq, Hash, PartialEq)]
pub enum DebugKey {
    Debug(String),
    Info(String),
    Warning(String),
    Fatal(String),
}

impl std::fmt::Display for DebugKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DebugKey::Debug(name) => write!(f, "[DEBUG] {}", name),
            DebugKey::Info(name) => write!(f, "[INFO] {}", name),
            DebugKey::Warning(name) => write!(f, "[WARN] {}", name),
            DebugKey::Fatal(name) => write!(f, "[FATAL] {}", name),
        }
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

        const DIALOG_CLOSE   = 0b1111_1110_1111;
        const SETTINGS_CLOSE = 0b1111_1101_1111;

        const DEBUGGING      = 0b1000_0000_0000;
    }
}

impl Default for AppMode {
    fn default() -> Self {
        Self::SELECTION
    }
}

#[derive(Clone, PartialEq, Eq)]
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

#[derive(Clone, PartialEq, Eq)]
pub enum EditingState {
    Rename,
    ChCount,
    ChTime,
}

pub struct App {
    pub state: AppState<2, 1>,
    pub c_store: CounterStore,
    pub ui_size: UiWidth,
    tick_rate: Duration,
    last_interaction: Instant,
    running: bool,
    pub time_show_millis: bool,
    cursor_pos: Option<(u16, u16)>,
    pub event_handler: EventHandler,
    pub debug_info: RefCell<HashMap<DebugKey, String>>,
}

impl App {
    pub fn new(tick_rate: u64, counter_store: CounterStore) -> Self {
        App {
            state: AppState::new(),
            tick_rate: Duration::from_millis(tick_rate),
            last_interaction: Instant::now(),
            c_store: counter_store,
            ui_size: UiWidth::Big,
            running: true,
            time_show_millis: true,
            cursor_pos: None,
            event_handler: EventHandler::new(0),
            debug_info: RefCell::new(HashMap::new()),
        }
    }
    pub fn set_super_user(self, input_fd: i32) -> Self {
        self.event_handler.set_fd(input_fd);
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

        self.list_select(0, Some(0));

        let mut previous_time = Instant::now();
        let mut now_time: Instant;

        self.debug_info.borrow_mut().insert(
            DebugKey::Debug("dev_input_files".to_string()),
            DevInputFileDescriptor::get_kbd_inputs()
                .into_iter()
                .map(|(_key, value)| value + ", ")
                .collect::<String>(),
        );

        while self.running {
            while self.event_handler.has_event()? {
                self.debug_info.borrow_mut().insert(
                    DebugKey::Debug("Last Key".to_string()),
                    format!("{:?}", self.event_handler.get_buffer()[0]),
                );
                self.handle_event()?;
            }

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
                ui::draw(f, &mut self).unwrap();
                // if settings are open draw on top
            })?;

            self.debug_info.borrow_mut().insert(
                DebugKey::Debug("draw time".to_string()),
                format!("{:?}", Instant::now() - terminal_start_time),
            );

            // if a widget alters the cursor position it will report to App
            // we set the terminal cursor position itself here
            if let Some(pos) = self.cursor_pos {
                terminal.set_cursor(pos.0, pos.1)?;
            }

            thread::sleep(self.tick_rate - (Instant::now() - now_time));
            self.debug_info.borrow_mut().insert(
                DebugKey::Debug("key event".to_string()),
                format!("{:?}", self.event_handler.get_buffer()),
            );
        }
        Ok(self)
    }

    pub fn get_act_counter(&self) -> Result<Ref<Counter>, AppError> {
        let selection = self.get_list_state(0).selected().unwrap_or(0);
        if let Some(counter) = self.c_store.get(selection) {
            Ok(counter)
        } else {
            Err(AppError::GetCounterError)
        }
    }

    pub fn get_mut_act_counter(&self) -> Result<RefMut<Counter>, AppError> {
        let selection = self.get_list_state(0).selected().unwrap_or(0);
        if let Some(counter) = self.c_store.get_mut(selection) {
            Ok(counter)
        } else {
            Err(AppError::GetCounterError)
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
    fn open_dialog(&mut self, dialog: Dialog) {
        self.toggle_mode(AppMode::DIALOG_OPEN);
        self.state.dialog = dialog
    }

    fn close_dialog(&mut self) {
        self.state.dialog = Dialog::None;
        self.state
            .set_mode(self.state.get_mode() & AppMode::DIALOG_CLOSE);
    }

    pub fn get_dialog_state(&self) -> Dialog {
        return self.state.dialog.clone();
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

    pub fn get_entry_state(&mut self, index: usize) -> &mut EntryState {
        return self.state.entry_states.get_mut(index).unwrap();
    }

    pub fn reset_entry_state(&mut self, index: usize) {
        self.state.entry_states[index] = EntryState::default();
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
                    Key::Char('n') => self.open_dialog(DS::AddNew),
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
            Key::Char('n') => self.open_dialog(DS::AddNew),
            Key::Char('d') => self.open_dialog(DS::Delete),
            Key::Char('e') => self.open_dialog(DS::Editing(ES::Rename)),
            Key::Char('f') => {
                let selected = self.get_list_state(1).selected().unwrap_or(0);
                self.list_select(1, Some(selected));
                self.toggle_mode(AppMode::PHASE_SELECT)
            }
            Key::Enter => {
                if self.get_act_counter().unwrap().get_phase_len() > 1 {
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
            Key::Char('+') | Key::Char('=') => {
                self.get_mut_act_counter()?.increase_by(1);
                self.c_store.to_json(SAVE_FILE)
            }
            Key::Char('-') => {
                self.get_mut_act_counter()?.increase_by(-1);
                self.c_store.to_json(SAVE_FILE)
            }
            Key::Char('*') => {
                self.event_handler.toggle_mode();
                self.toggle_mode(AppMode::KEYLOGGING)
            }
            Key::Char('q') | Key::Esc => {
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
                self.reset_entry_state(0);
            }
            Key::Enter => {
                let name = self.get_entry_state(0).get_active_field().clone();
                self.c_store.push(Counter::new(name));
                self.reset_entry_state(0);
                self.close_dialog();
            }
            Key::Char(charr) if charr.is_ascii() => self.get_entry_state(0).push(charr),
            Key::Backspace => {
                self.get_entry_state(0).pop();
            }
            _ => {}
        }
        Ok(())
    }

    fn delete_counter_key_event(&mut self, key: Key) -> Result<(), AppError> {
        match key {
            Key::Enter => {
                self.set_mode(AppMode::SELECTION);
                if let Some(selected) = self.get_list_state(0).selected() {
                    if self.c_store.len() == 1 {
                        self.c_store.remove(0);
                    }

                    self.c_store.remove(selected);
                    if selected == self.c_store.len() {
                        self.list_select(0, Some(selected - 1))
                    }
                }
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
            Key::Char(charr) if charr.is_ascii() => self.get_entry_state(0).push(charr),
            Key::Backspace => {
                self.get_entry_state(0).pop();
            }
            Key::Enter => {
                let name = self.get_entry_state(0).get_active_field().clone();
                self.get_mut_act_counter()?.set_name(&name);
                self.reset_entry_state(0);
                self.open_dialog(DS::Editing(ES::ChCount));
            }
            Key::Esc => {
                self.close_dialog();
                self.reset_entry_state(0);
            }
            _ => {}
        }
        Ok(())
    }

    fn change_count_key_event(&mut self, key: Key) -> Result<(), AppError> {
        match key {
            Key::Char(charr) if charr.is_numeric() => self.get_entry_state(0).push(charr),
            Key::Backspace => {
                self.get_entry_state(0).pop();
            }
            Key::Enter => {
                let count = self
                    .get_entry_state(0)
                    .get_active_field()
                    .parse()
                    .unwrap_or_else(|_| self.get_act_counter().unwrap().get_count());
                self.get_mut_act_counter()?.set_count(count);
                self.reset_entry_state(0);
                self.open_dialog(DS::Editing(ES::ChTime));
            }
            Key::Esc => {
                self.close_dialog();
                self.reset_entry_state(0);
            }
            _ => {}
        }
        Ok(())
    }

    fn change_time_key_event(&mut self, key: Key) -> Result<(), AppError> {
        match key {
            Key::Char(charr) if charr.is_numeric() => self.get_entry_state(0).push(charr),
            Key::Backspace => {
                self.get_entry_state(0).pop();
            }
            Key::Enter => {
                let time = self
                    .get_entry_state(0)
                    .get_active_field()
                    .parse()
                    .unwrap_or(self.get_act_counter()?.get_time().as_secs() / 60);
                self.get_mut_act_counter()?
                    .set_time(Duration::from_secs(time * 60));
                self.reset_entry_state(0);
                self.close_dialog()
            }
            Key::Esc => {
                self.reset_entry_state(0);
                self.close_dialog()
            }
            _ => {}
        }
        Ok(())
    }
    fn rename_phase_key_event(&mut self, key: Key) -> Result<(), AppError> {
        match key {
            Key::Char(charr) if charr.is_ascii() => self.get_entry_state(0).push(charr),
            Key::Backspace => self.get_entry_state(0).pop(),
            Key::Enter => {
                let phase = self.get_list_state(1).selected().unwrap_or(0);
                let name = self.get_entry_state(0).get_active_field().clone();
                self.get_mut_act_counter()?.set_phase_name(phase, name);
                self.reset_entry_state(0);
                self.close_dialog()
            }
            Key::Esc => {
                self.reset_entry_state(0);
                self.close_dialog()
            }
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
            Key::Char('d') => self.open_dialog(DS::Delete),
            Key::Char('n') => self.get_mut_act_counter()?.new_phase(),
            Key::Char('r') => self.open_dialog(DS::Editing(ES::Rename)),
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
            tick_rate: Duration::from_millis(40),
            last_interaction: Instant::now(),
            running: true,
            time_show_millis: true,
            cursor_pos: None,
            event_handler: EventHandler::new(0),
            debug_info: RefCell::new(HashMap::default()),
        }
    }
}

#[derive(Default)]
pub struct AppState<const T: usize, const U: usize> {
    mode: RefCell<AppMode>,
    dialog: Dialog,
    list_states: Vec<ListState>,
    entry_states: Vec<EntryState>,
}

impl<const T: usize, const U: usize> AppState<T, U> {
    fn new() -> Self {
        Self {
            mode: RefCell::new(AppMode::default()),
            dialog: Dialog::None,
            list_states: vec![ListState::default(); T],
            entry_states: vec![EntryState::default(); U],
        }
    }

    fn get_mode(&self) -> AppMode {
        self.mode.clone().into_inner()
    }

    fn set_mode(&self, mode: AppMode) {
        self.mode.swap(&RefCell::new(mode))
    }

    fn toggle_mode(&self, mode: AppMode) {
        self.mode
            .swap(&RefCell::new(self.mode.clone().borrow().clone() ^ mode))
    }
}
