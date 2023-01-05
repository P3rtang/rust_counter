use crate::entry::EntryState;
use crate::ui::{self, UiWidth};
use crate::SAVE_FILE;
use crate::{
    counter::{Counter, CounterStore},
    FRAME_RATE, TICK_SLOWDOWN, TIME_OUT,
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::cell::{Ref, RefMut};
use std::io;
use std::time::{Duration, Instant};
use tui::{backend::CrosstermBackend, widgets::ListState, Terminal};
use DialogState as DS;
use EditingState as ES;

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
    is_super_user:        bool,
    cursor_pos:           Option<(u16, u16)>,
    pub debug_info:       Vec<String>,
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
            is_super_user:    false,
            cursor_pos:       None,
            debug_info:       vec![],
        }
    }
    pub fn set_super_user(mut self, is_super: bool) -> Self {
        self.is_super_user = is_super;
        self
    }
    pub fn start(mut self) -> io::Result<Self> {
        // setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).unwrap();

        self.get_mut_list_state(0).select(Some(0));

        let mut previous_time = Instant::now();
        let mut now_time: Instant;

        while self.running {
            // timing the execution time of the loop and add it to the counter time
            now_time = Instant::now();
            if let AppMode::Counting(_) = self.get_mode() {
                self.get_unsafe_c_mut()
                    .increase_time(now_time - previous_time);
            }
            previous_time = Instant::now();

            // draw all ui elements
            terminal.draw(|f| {
                ui::draw(f, &mut self);
            })?;

            // if a widget alters the cursor position it will report to App
            // we set the terminal cursor position itself here
            if let Some(pos) = self.cursor_pos {
                terminal.set_cursor(pos.0, pos.1).unwrap();
            }

            // handle input events
            // if timeout time has been reached since the last interaction we call the blocking
            // handle_event function by doing so pausing the app until a new input is given
            // otherwise check if there is an input event and only call the blocking fn when there
            // is one
            // if the TICK_SLOWDOWN time has been reached put the program in a slower poll rate
            if Instant::now() - self.last_interaction > Duration::from_secs(TIME_OUT) {
                self.handle_event();
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
                self.handle_event();
                self.last_interaction = Instant::now();
                self.time_show_millis = true;
                self.tick_rate = Duration::from_millis(1000 / FRAME_RATE)
            }
        }
        Ok(self)
    }

    pub fn get_active_counter(&self) -> Option<Ref<Counter>> {
        self.c_store.get(self.get_list_state(0).selected().unwrap_or(0))
    }

    pub fn get_unsafe_counter(&self) -> Ref<Counter> {
        self.get_active_counter().unwrap()
    }

    pub fn get_active_c_mut(&self) -> Option<RefMut<Counter>> {
        self.c_store.get_mut(self.get_list_state(0).selected().unwrap_or(0))
    }

    pub fn get_unsafe_c_mut(&self) -> RefMut<Counter> {
        self.get_active_c_mut().unwrap()
    }

    pub fn end(self) -> io::Result<CounterStore> {
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
        Ok(self.c_store)
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

    pub fn list_select(&mut self, index: usize, select_index: usize) {
        return self.state.list_states[index].select(Some(select_index))
    }

    pub fn get_entry_state(&mut self, index: usize) -> &mut EntryState {
        return self.state.entry_state.get_mut(index).unwrap()
    }

    pub fn reset_entry_state(&mut self, index: usize) {
        self.state.entry_state[index] = EntryState::default();
    }

    fn handle_event(&mut self) {
        if let Event::Key(key) = event::read().unwrap() {
            match self.get_mode() {
                AppMode::Selection(DS::None) if self.c_store.len() > 0 => {
                    self.selection_key_event(key.code)
                }
                AppMode::Selection(DS::None) => match key.code {
                    KeyCode::Char('q') => self.running = false,
                    KeyCode::Char('n') => self.set_mode(AppMode::Selection(DS::AddNew)),
                    _ => {}
                },
                AppMode::Counting(n) => match key.code {
                    KeyCode::Char(charr) if (charr == '=') | (charr == '+') => {
                        self.get_unsafe_c_mut().increase_by(1);
                        self.c_store.to_json(SAVE_FILE)
                    }
                    KeyCode::Char('-') => {
                        self.get_unsafe_c_mut().increase_by(-1);
                        self.c_store.to_json(SAVE_FILE)
                    }
                    KeyCode::Esc => self.set_mode(AppMode::Selection(DS::None)),
                    KeyCode::Char('q') if n == &0 => {
                        self.list_select(1, 0);
                        self.set_mode(AppMode::Selection(DS::None))
                    }
                    KeyCode::Char('q') if n == &1 => {
                        self.set_mode(AppMode::PhaseSelect(DS::None)) 
                    }
                    _ => {}
                },
                AppMode::Selection(DS::AddNew) => match key.code {
                    KeyCode::Esc => {
                        self.set_mode(AppMode::Selection(DS::None));
                        self.reset_entry_state(0);
                    }
                    KeyCode::Enter => {
                        let name = self.get_entry_state(0).get_active_field().clone();
                        self.c_store.push(Counter::new(name));
                        self.reset_entry_state(0);
                        self.set_mode(AppMode::Selection(DS::None));
                    }
                    KeyCode::Char(charr) if charr.is_ascii() => self.get_entry_state(0).push(charr),
                    KeyCode::Backspace => {
                        self.get_entry_state(0).pop();
                    }
                    _ => {}
                },
                AppMode::Selection(DS::Editing(ES::Rename(mode))) => 
                    self.rename_key_event(key.code, *mode),
                AppMode::Selection(DS::Editing(ES::ChCount(mode))) => 
                    self.change_count_key_event(key.code, *mode),
                AppMode::Selection(DS::Editing(ES::ChTime(mode))) => {
                    self.change_time_key_event(key.code, *mode)
                }
                AppMode::Selection(DS::Delete) => match key.code {
                    KeyCode::Enter => {
                        self.c_store.remove(self.get_list_state(0).selected().unwrap_or(0));
                        if self.get_list_state(0).selected().unwrap_or(0) >= self.c_store.len()
                            && self.c_store.len() > 0
                        {
                            self.list_select(0, self.c_store.len() - 1);
                        }
                        self.set_mode(AppMode::Selection(DS::None))
                    }
                    KeyCode::Esc => self.set_mode(AppMode::Selection(DS::None)),
                    _ => {}
                },
                AppMode::PhaseSelect(DS::Editing(_)) => {
                    self.rename_phase_key_event(key.code)
                }
                AppMode::PhaseSelect(DS::Delete) => match key.code {
                    KeyCode::Enter => {
                        self.get_unsafe_c_mut()
                            .remove_phase(self.get_list_state(0).selected().unwrap_or(0) + 1);
                        self.set_mode(AppMode::Selection(DS::None));

                        let selection = self.get_list_state(1).selected().unwrap_or(0);
                        if selection >= self.get_unsafe_counter().get_phase_len() {
                            self.list_select(1, selection - 1)
                        }
                    }
                    KeyCode::Esc => self.set_mode(AppMode::Selection(DS::None)),
                    _ => {}
                },
                AppMode::PhaseSelect(_) => self.phase_select_key_event(key.code),
            }
        }
    }

    fn selection_key_event(&mut self, key: KeyCode) {
        let len = self.c_store.len();
        match key {
            KeyCode::Char('q') => self.running = false,
            KeyCode::Char('n') => self.set_mode(AppMode::Selection(DS::AddNew)),
            KeyCode::Char('d') => self.set_mode(AppMode::Selection(DS::Delete)),
            KeyCode::Char('r') => 
                self.set_mode(AppMode::Selection(DS::Editing(ES::Rename(false)))),
            KeyCode::Char('e') => 
                self.set_mode(AppMode::Selection(DS::Editing(ES::Rename(true)))),
            KeyCode::Char('f') => {
                self.list_select(1, self.get_list_state(1).selected().unwrap_or(0));
                self.set_mode(AppMode::PhaseSelect(DS::None)) 
            }
            KeyCode::Enter => {
                if self.get_active_counter().unwrap().get_phase_len() > 1 {
                    self.list_select(1, self.get_list_state(1).selected().unwrap_or(0));
                    self.set_mode(AppMode::PhaseSelect(DS::None))
                } else {
                    self.list_select(1, 0);
                    self.set_mode(AppMode::Counting(0))
                }
            }
            KeyCode::Up => {
                let mut selected = self.get_list_state(0).selected().unwrap();
                selected += len - 1;
                selected %= len;
                self.list_select(0, selected);
            }
            KeyCode::Down => {
                let mut selected = self.get_list_state(0).selected().unwrap();
                selected += 1;
                selected %= len;
                self.list_select(0, selected);
            }
            _ => {}
        }
    }

    fn rename_key_event(&mut self, key: KeyCode, in_editing_mode: bool) {
        match key {
            KeyCode::Char(charr) if charr.is_ascii() => self.get_entry_state(0).push(charr),
            KeyCode::Backspace => {
                self.get_entry_state(0).pop();
            }
            KeyCode::Enter => {
                let name = self.get_entry_state(0).get_active_field().clone();
                self.get_unsafe_c_mut().set_name(&name);
                self.reset_entry_state(0);
                if in_editing_mode { 
                    self.set_mode(AppMode::Selection(DS::Editing(ES::ChCount(true))))
                } else {
                    self.set_mode(AppMode::Selection(DS::None))
                }
            }
            KeyCode::Esc => {
                self.set_mode(AppMode::Selection(DS::None));
                self.reset_entry_state(0);
            }
            _ => {}
        }
    }

    fn change_count_key_event(&mut self, key: KeyCode, in_editing_mode: bool) {
        match key {
            KeyCode::Char(charr) if charr.is_numeric() => self.get_entry_state(0).push(charr),
            KeyCode::Backspace => {
                self.get_entry_state(0).pop();
            }
            KeyCode::Enter => {
                let count = self.get_entry_state(0)
                    .get_active_field()
                    .parse()
                    .unwrap_or_else(|_| self.get_active_counter().unwrap().get_count());
                self.get_unsafe_c_mut().set_count(count);
                self.reset_entry_state(0);
                if in_editing_mode { 
                    self.set_mode(AppMode::Selection(DS::Editing(ES::ChTime(true))))
                } else {
                    self.set_mode(AppMode::Selection(DS::None))
                }
            }
            KeyCode::Esc => {
                self.set_mode(AppMode::Selection(DS::None));
                self.reset_entry_state(0);
            }
            _ => {}
        }
    }

    fn change_time_key_event(&mut self, key: KeyCode, _in_editing_mode: bool) {
        match key {
            KeyCode::Char(charr) if charr.is_numeric() => self.get_entry_state(0).push(charr),
            KeyCode::Backspace => {
                self.get_entry_state(0).pop();
            }
            KeyCode::Enter => {
                let time = self.get_entry_state(0)
                    .get_active_field()
                    .parse()
                    .unwrap_or(self.get_unsafe_counter().get_time().as_secs() / 60);
                self.get_unsafe_c_mut().set_time(time);
                self.reset_entry_state(0);
                self.set_mode(AppMode::Selection(DS::None))
            }
            KeyCode::Esc => {
                self.reset_entry_state(0);
                self.set_mode(AppMode::Selection(DS::None))
            }
            _ => {}
        }
    }
    fn rename_phase_key_event(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char(charr) if charr.is_ascii() => self.get_entry_state(0).push(charr),
            KeyCode::Backspace => self.get_entry_state(0).pop(),
            KeyCode::Enter => {
                let phase = self.get_list_state(1).selected().unwrap_or(0);
                let name  = self.get_entry_state(0).get_active_field().clone();
                self.get_unsafe_c_mut().set_phase_name(phase, name);
                self.reset_entry_state(0);
                self.set_mode(AppMode::PhaseSelect(DS::None))
            }
            KeyCode::Esc => {
                self.reset_entry_state(0);
                self.set_mode(AppMode::PhaseSelect(DS::None))
            }
            _ => {}
        }
    }
    fn phase_select_key_event(&mut self, key: KeyCode) {
        let len = self.get_active_counter().unwrap().get_phase_len();
        match key {
            KeyCode::Char('d') if self.get_unsafe_counter().get_phase_len() == 1 => {
                self.set_mode(AppMode::Selection(DS::None))
            }
            KeyCode::Char('d') => {
                self.set_mode(AppMode::Selection(DS::Delete))
            }
            KeyCode::Char('n') => self.get_unsafe_c_mut().new_phase(),
            KeyCode::Char('r') => 
                self.set_mode(AppMode::PhaseSelect(DS::Editing(ES::Rename(false)))),
            KeyCode::Up => {
                let mut selected = self.get_list_state(1).selected().unwrap();
                selected += len - 1;
                selected %= len;
                self.get_mut_list_state(1).select(Some(selected as usize));
            }
            KeyCode::Down => {
                let mut selected = self.get_list_state(1).selected().unwrap();
                selected += 1;
                selected %= len;
                self.get_mut_list_state(1).select(Some(selected as usize));
            }
            KeyCode::Enter => {
                self.get_mut_list_state(1).select(Some(0));
                self.set_mode(AppMode::Counting(1));
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                self.get_mut_list_state(1).select(None);
                self.set_mode(AppMode::Selection(DS::None))
            }
            _ => {}
        }
    }
}

pub struct AppState<const T:usize, const U:usize> {
    mode: AppMode,
    list_states: Vec<ListState>,
    pub entry_state: Vec<EntryState>,
}

impl<const T:usize, const U:usize> AppState<T, U> {
    fn new() -> Self {
        Self { 
            mode:             AppMode::default(),
            list_states:      vec![ListState::default(); T],
            entry_state:      vec![EntryState::default(); U],
        }
    }
}

impl<const T:usize, const U:usize> Default for AppState<T, U> {
    fn default() -> Self {
        Self::new()
    }
}
