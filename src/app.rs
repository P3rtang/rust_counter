use crate::widgets::entry::EntryState;
use crate::ui::{self, UiWidth};
use crate::SAVE_FILE;
use crate::{
    counter::{Counter, CounterStore},
    FRAME_RATE, TICK_SLOWDOWN, TIME_OUT,
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::cell::{Ref, RefMut};
use std::io;
use std::time::{Duration, Instant};
use tui::{backend::CrosstermBackend, widgets::ListState, Terminal};
use DialogState as DS;
use EditingState as ES;
use crate::input::{InputEvent, Key};

#[derive(Debug)]
pub enum AppError {
    GetCounterError,
    GetPhaseError,
    IoError,
}

impl From<io::Error> for AppError {
    fn from(_: io::Error) -> Self {
        Self::IoError
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum AppMode {
    Selection(DialogState),
    PhaseSelect(DialogState),
    Counting(u8),
}

impl Default for AppMode {
    fn default() -> Self {
        Self::Selection(DS::None)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum DialogState {
    AddNew,
    Editing(EditingState),
    Delete,
    None,
}

#[derive(Clone, PartialEq, Eq)]
pub enum EditingState {
    Rename(bool),
    ChCount(bool),
    ChTime(bool),
}

pub struct App {
    pub state:            AppState<2, 1>,
    pub c_store:          CounterStore,
    pub ui_size:          UiWidth,
    pub time_show_millis: bool,
    tick_rate:            Duration,
    last_interaction:     Instant,
    running:              bool,
    cursor_pos:           Option<(u16, u16)>,
    pub debug_info:       Vec<String>,
    dev_input_fd:         i32,
}

impl App {
    pub fn new(tick_rate: u64, counter_store: CounterStore) -> Self {
        App {
            state:            AppState::default(),
            tick_rate:        Duration::from_millis(tick_rate),
            last_interaction: Instant::now(),
            c_store:          counter_store,
            ui_size:          UiWidth::Big,
            running:          true,
            time_show_millis: true,
            cursor_pos:       None,
            debug_info:       vec![],
            dev_input_fd:     0,
        }
    }
    pub fn set_super_user(mut self, input_fd: i32) -> Self {
        self.dev_input_fd = input_fd;
        self
    }
    pub fn start(mut self) -> Result<App, AppError> {
        // setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

            self.list_select(0, Some(0));

        let mut previous_time = Instant::now();
        let mut now_time: Instant;

        while self.running {
            // timing the execution time of the loop and add it to the counter time
            now_time = Instant::now();
            if let AppMode::Counting(_) = self.get_mode() {
                self.get_mut_act_counter().unwrap()
                    .increase_time(now_time - previous_time);
            }
            previous_time = Instant::now();

            // draw all ui elements
            terminal.draw(|f| {
                ui::draw(f, &mut self).unwrap();
            })?;

            // if a widget alters the cursor position it will report to App
            // we set the terminal cursor position itself here
            if let Some(pos) = self.cursor_pos {
                terminal.set_cursor(pos.0, pos.1)?;
            }

            // handle input events
            // if timeout time has been reached since the last interaction we call the blocking
            // handle_event function by doing so pausing the app until a new input is given
            // otherwise check if there is an input event and only call the blocking fn when there
            // is one
            // if the TICK_SLOWDOWN time has been reached put the program in a slower poll rate
            if Instant::now() - self.last_interaction > Duration::from_secs(TIME_OUT) {
                if let Err(e) = self.handle_event() { self.debug_info.push(format!("{:?}", e))}
                self.last_interaction = Instant::now();
                // set previous time to `Now` so the pause time doesn't get added to the counter
                previous_time = Instant::now();
                self.time_show_millis = true;
            } else if Instant::now() - self.last_interaction > Duration::from_secs(TICK_SLOWDOWN) {
                self.time_show_millis = false;
                self.tick_rate = Duration::from_millis(500);
            }
            if crossterm::event::poll(self.tick_rate.saturating_sub(Instant::now() - now_time))
                .unwrap()
            {
                if let Err(e) = self.handle_event() { self.debug_info.push(format!("{:?}", e))}
                self.last_interaction = Instant::now();
                self.time_show_millis = true;
                self.tick_rate = Duration::from_millis(1000 / FRAME_RATE)
            }
        }
        Ok(self)
    }

    pub fn get_act_counter(&self) -> Result<Ref<Counter>, AppError> {
        let selection = self.get_list_state(0).selected().unwrap_or(0);
        if let Some(counter) = self.c_store.get(selection) {
            return Ok(counter)
        } else {
            return Err(AppError::GetCounterError)
        }
    }

    pub fn get_mut_act_counter(&self) -> Result<RefMut<Counter>, AppError> {
        let selection = self.get_list_state(0).selected().unwrap_or(0);
        if let Some(counter) = self.c_store.get_mut(selection) {
            return Ok(counter)
        } else {
            return Err(AppError::GetCounterError)
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

    pub fn get_mode(&self) -> &AppMode {
        return &self.state.mode
    }

    pub fn set_mode(&mut self, mode: AppMode) {
        self.state.mode = mode;
    }

    pub fn get_list_state(&self, index: usize) -> &ListState {
        return self.state.list_states.get(index).unwrap()
    }

    pub fn get_mut_list_state(&mut self, index: usize) -> &mut ListState {
        return self.state.list_states.get_mut(index).unwrap()
    }

    pub fn list_select(&mut self, index: usize, select_index: Option<usize>) {
        self.state.list_states[index].select(select_index)
    }

    pub fn list_deselect(&mut self, index: usize) {
        self.state.list_states[index].select(None)
    }

    pub fn get_entry_state(&mut self, index: usize) -> &mut EntryState {
        return self.state.entry_states.get_mut(index).unwrap()
    }

    pub fn reset_entry_state(&mut self, index: usize) {
        self.state.entry_states[index] = EntryState::default();
    }

    fn handle_event(&mut self) -> Result<(), AppError> {
        let key: Key = if self.dev_input_fd != 0 {
            if let Some(key) = InputEvent::poll( self.tick_rate, self.dev_input_fd) {
                self.debug_info.push(key.clone().to_string());
                key.code.into() 
            }
            else { return Ok(()) }
        } else {
            if let Event::Key(key) = event::read().unwrap() { key.into() } else { return Ok(()) }
        };
        match self.get_mode() {
            AppMode::Selection(DS::None) if self.c_store.len() > 0 => {
                self.selection_key_event(key)
            }
            AppMode::Selection(DS::None) => match key {
                Key::Char('q') => self.running = false,
                Key::Char('n') => self.set_mode(AppMode::Selection(DS::AddNew)),
                _ => {}
            },
            AppMode::Counting(n) => match key {
                Key::Char(charr) if (charr == '=') | (charr == '+') => {
                    self.get_mut_act_counter()?.increase_by(1);
                    self.c_store.to_json(SAVE_FILE)
                }
                Key::Char('-') => {
                    self.get_mut_act_counter()?.increase_by(-1);
                    self.c_store.to_json(SAVE_FILE)
                }
                Key::Esc => self.set_mode(AppMode::Selection(DS::None)),
                Key::Char('q') if n == &0 => {
                    self.list_deselect(1);
                    self.set_mode(AppMode::Selection(DS::None))
                }
                Key::Char('q') if n == &1 => {
                    self.set_mode(AppMode::PhaseSelect(DS::None)) 
                }
                _ => {}
            },
            AppMode::Selection(DS::AddNew) => match key {
                Key::Esc => {
                    self.set_mode(AppMode::Selection(DS::None));
                    self.reset_entry_state(0);
                }
                Key::Enter => {
                    let name = self.get_entry_state(0).get_active_field().clone();
                    self.c_store.push(Counter::new(name));
                    self.reset_entry_state(0);
                    self.set_mode(AppMode::Selection(DS::None));
                }
                Key::Char(charr) if charr.is_ascii() => self.get_entry_state(0).push(charr),
                Key::Backspace => {
                    self.get_entry_state(0).pop();
                }
                _ => {}
            },
            AppMode::Selection(DS::Editing(ES::Rename(mode))) => 
                self.rename_key_event(key, *mode)?,
            AppMode::Selection(DS::Editing(ES::ChCount(mode))) => 
                self.change_count_key_event(key, *mode)?,
            AppMode::Selection(DS::Editing(ES::ChTime(mode))) => {
                self.change_time_key_event(key, *mode)?;
            }
            AppMode::Selection(DS::Delete) => match key {
                Key::Enter => {
                    self.set_mode(AppMode::Selection(DS::None));
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
                Key::Esc => self.set_mode(AppMode::Selection(DS::None)),
                _ => {}
            },
            AppMode::PhaseSelect(DS::Editing(_)) => {
                self.rename_phase_key_event(key)?
            }
            AppMode::PhaseSelect(DS::Delete) => match key {
                Key::Enter => {
                    todo!();
                }
                Key::Esc => self.set_mode(AppMode::Selection(DS::None)),
                _ => {}
            },
            AppMode::PhaseSelect(_) => self.phase_select_key_event(key)?,
        }
        Ok(())
    }

    fn selection_key_event(&mut self, key: Key) {
        let len = self.c_store.len();
        match key {
            Key::Char('q') | Key::Esc => self.running = false,
            Key::Char('n') => self.set_mode(AppMode::Selection(DS::AddNew)),
            Key::Char('d') => self.set_mode(AppMode::Selection(DS::Delete)),
            Key::Char('r') => 
                self.set_mode(AppMode::Selection(DS::Editing(ES::Rename(false)))),
            Key::Char('e') => 
                self.set_mode(AppMode::Selection(DS::Editing(ES::Rename(true)))),
            Key::Char('f') => {
                let selected = self.get_list_state(1).selected().unwrap_or(0);
                self.list_select(1, Some(selected));
                self.set_mode(AppMode::PhaseSelect(DS::None)) 
            }
            Key::Enter => {
                if self.get_act_counter().unwrap().get_phase_len() > 1 {
                    let selected = self.get_list_state(1).selected().unwrap_or(0);
                    self.list_select(1, Some(selected));
                    self.set_mode(AppMode::PhaseSelect(DS::None))
                } else {
                    self.list_select(1, Some(0));
                    self.set_mode(AppMode::Counting(0))
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
    }

    fn rename_key_event(&mut self, key: Key, in_editing_mode: bool) -> Result<(), AppError> {
        match key {
            Key::Char(charr) if charr.is_ascii() => self.get_entry_state(0).push(charr),
            Key::Backspace => {
                self.get_entry_state(0).pop();
            }
            Key::Enter => {
                let name = self.get_entry_state(0).get_active_field().clone();
                self.get_mut_act_counter()?.set_name(&name);
                self.reset_entry_state(0);
                if in_editing_mode { 
                    self.set_mode(AppMode::Selection(DS::Editing(ES::ChCount(true))))
                } else {
                    self.set_mode(AppMode::Selection(DS::None))
                }
            }
            Key::Esc => {
                self.set_mode(AppMode::Selection(DS::None));
                self.reset_entry_state(0);
            }
            _ => {}
        }
        Ok(())
    }

    fn change_count_key_event(&mut self, key: Key, in_editing_mode: bool)
        -> Result<(), AppError> 
    {
        match key {
            Key::Char(charr) if charr.is_numeric() => self.get_entry_state(0).push(charr),
            Key::Backspace => {
                self.get_entry_state(0).pop();
            }
            Key::Enter => {
                let count = self.get_entry_state(0)
                    .get_active_field()
                    .parse()
                    .unwrap_or_else(|_| self.get_act_counter().unwrap().get_count());
                self.get_mut_act_counter()?.set_count(count);
                self.reset_entry_state(0);
                if in_editing_mode { 
                    self.set_mode(AppMode::Selection(DS::Editing(ES::ChTime(true))))
                } else {
                    self.set_mode(AppMode::Selection(DS::None))
                }
            }
            Key::Esc => {
                self.set_mode(AppMode::Selection(DS::None));
                self.reset_entry_state(0);
            }
            _ => {}
        }
        Ok(())
    }

    fn change_time_key_event(&mut self, key: Key, _in_editing_mode: bool)
        -> Result<(), AppError> 
    {
        match key {
            Key::Char(charr) if charr.is_numeric() => self.get_entry_state(0).push(charr),
            Key::Backspace => {
                self.get_entry_state(0).pop();
            }
            Key::Enter => {
                let time = self.get_entry_state(0)
                    .get_active_field()
                    .parse()
                    .unwrap_or(self.get_act_counter()?.get_time().as_secs() / 60);
                self.get_mut_act_counter()?.set_time(Duration::from_secs(time * 60));
                self.reset_entry_state(0);
                self.set_mode(AppMode::Selection(DS::None))
            }
            Key::Esc => {
                self.reset_entry_state(0);
                self.set_mode(AppMode::Selection(DS::None))
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
                let name  = self.get_entry_state(0).get_active_field().clone();
                self.get_mut_act_counter()?.set_phase_name(phase, name);
                self.reset_entry_state(0);
                self.set_mode(AppMode::PhaseSelect(DS::None))
            }
            Key::Esc => {
                self.reset_entry_state(0);
                self.set_mode(AppMode::PhaseSelect(DS::None))
            }
            _ => {}
        }
        Ok(())
    }
    fn phase_select_key_event(&mut self, key: Key) -> Result<(), AppError> {
        let len = self.get_act_counter().unwrap().get_phase_len();
        match key {
            Key::Char('d') if self.get_act_counter()?.get_phase_len() == 1 => {
                self.set_mode(AppMode::Selection(DS::None))
            }
            Key::Char('d') => {
                self.set_mode(AppMode::Selection(DS::Delete))
            }
            Key::Char('n') => self.get_mut_act_counter()?.new_phase(),
            Key::Char('r') => 
                self.set_mode(AppMode::PhaseSelect(DS::Editing(ES::Rename(false)))),
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
                self.set_mode(AppMode::Counting(1));
            }
            Key::Esc | Key::Char('q') => {
                self.list_deselect(1);
                self.set_mode(AppMode::Selection(DS::None))
            }
            _ => {}
        }
        Ok(())
    }
}

pub struct AppState<const T:usize, const U:usize> {
    mode:         AppMode,
    list_states:  Vec<ListState>,
    entry_states: Vec<EntryState>,
}

impl<const T:usize, const U:usize> AppState<T, U> {
    fn new() -> Self {
        Self { 
            mode:         AppMode::default(),
            list_states:  vec![ ListState::default(); T],
            entry_states: vec![EntryState::default(); U],
        }
    }
}

impl<const T:usize, const U:usize> Default for AppState<T, U> {
    fn default() -> Self {
        Self::new()
    }
}
