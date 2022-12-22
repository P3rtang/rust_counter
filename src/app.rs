use std::{io::{self, stdout}, time::{Instant, Duration}};
use tui::{
    backend::CrosstermBackend,
    widgets::ListState,
    Terminal
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use crate::{counter::{Counter, CounterStore}, TIME_OUT, FRAME_RATE, TICK_SLOWDOWN};
use crate::ui;
use crate::entry::EntryState;

#[derive(Clone, PartialEq)]
pub enum AppState {
    Selection,
    Counting,
    AddingNew,
    Rename,
    ChangeCount,
    Delete,
    Editing(u8),
}

pub struct App {
    tick_rate:        Duration,
    last_interaction: Instant,
    c_store:          CounterStore,
    c_state:          ListState,
    entry_state:      EntryState,
    app_state:        AppState,
    running:          bool,
    time_show_millis: bool,
    cursor_pos:       Option<(u16, u16)>,
}

impl App {
    pub fn new(tick_rate: u64, counter_store: CounterStore) -> Self {
        return App { 
            tick_rate:        Duration::from_millis(tick_rate),
            last_interaction: Instant::now(),
            c_store:          counter_store,
            c_state:          ListState::default(),
            entry_state:      EntryState::default(),
            app_state:        AppState::Selection,
            running:          true,
            time_show_millis: true,
            cursor_pos:       None,
        }
    }
    pub fn start(&mut self) -> io::Result<()> {
        // setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).unwrap();

        self.c_state.select(Some(0));

        let mut previous_time = Instant::now();
        let mut now_time      : Instant;

        while self.running {
            // timing the execution time of the loop and add it to the counter time
            now_time = Instant::now();
            if self.app_state == AppState::Counting {
                self.c_store.get_mut(self.c_state.selected().unwrap()).unwrap().increase_time(now_time - previous_time);
            }
            previous_time = Instant::now();

            // draw all ui elements
            terminal.draw(|f| {
                ui::draw(f, &self.c_store, &mut self.c_state, self.app_state.clone(), &mut self.entry_state, self.time_show_millis);
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
            if crossterm::event::poll(self.tick_rate.saturating_sub(Instant::now() - now_time)).unwrap() {
                self.handle_event();
                self.last_interaction = Instant::now();
                self.time_show_millis = true;
                self.tick_rate = Duration::from_millis(1000 / FRAME_RATE)
            }
        }
        Ok(())
    }

    pub fn get_counter(&mut self) -> &mut Counter {
        return self.c_store.get_mut(self.c_state.selected().unwrap_or(0)).unwrap()
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

    fn handle_event(&mut self) {
        if let Event::Key(key) = event::read().unwrap() {
            match self.app_state {
                AppState::Selection if self.c_store.len() > 0 => {
                    self.selection_key_event(key.code)
                }
                AppState::Selection => {
                    match key.code {
                        KeyCode::Char('q') => self.running = false,
                        KeyCode::Char('n') => { self.app_state = AppState::AddingNew }
                        _ => {}
                    }
                }
                AppState::Counting => {
                    match key.code {
                        KeyCode::Char(charr) if (charr == '=') | (charr == '+') => {
                            self.get_counter().increase_by(1);
                        }
                        KeyCode::Char('-') => {
                            self.get_counter().increase_by(-1);
                        }
                        KeyCode::Esc       => { self.app_state = AppState::Selection }
                        KeyCode::Char('q') => { self.app_state = AppState::Selection }
                        _ => {}
                    }
                }
                AppState::AddingNew => {
                    match key.code {
                        KeyCode::Esc => { 
                            self.app_state = AppState::Selection;
                            self.entry_state = EntryState::default();
                        }
                        KeyCode::Enter => { 
                            self.c_store.push(Counter::new(&self.entry_state.get_field()));
                            self.entry_state = EntryState::default();
                            self.app_state = AppState::Selection;
                        }
                        KeyCode::Char(charr) if charr.is_ascii() => { 
                            self.entry_state.push(charr) 
                        }
                        KeyCode::Backspace => { self.entry_state.pop() }
                        _ => {}
                    }
                }
                AppState::Rename => {
                    self.rename_key_event(key.code)
                }
                AppState::ChangeCount => {
                    self.change_count_key_event(key.code)
                }
                AppState::Delete => {
                    match key.code {
                        KeyCode::Enter => {
                            self.c_store.remove(self.c_state.selected().unwrap_or(0));
                            if self.c_state.selected().unwrap_or(0) >= self.c_store.len() && self.c_store.len() > 0 {
                                self.c_state.select(Some(self.c_store.len() - 1));
                            }
                            self.app_state = AppState::Selection
                        }
                        KeyCode::Esc   => { self.app_state = AppState::Selection }
                        _ => {}
                    }
                }
                AppState::Editing(stage) => {
                    self.editing_key_event(key.code, stage)
                }
            }
        }
    }

    fn selection_key_event(&mut self, key: KeyCode) {
        let len = self.c_store.len();
        match key {
            KeyCode::Char('q') => self.running = false,
            KeyCode::Char('n') => { self.app_state = AppState::AddingNew   }
            KeyCode::Char('d') => { self.app_state = AppState::Delete      }
            KeyCode::Char('r') => { self.app_state = AppState::Rename      }
            KeyCode::Char('s') => { self.app_state = AppState::ChangeCount }
            KeyCode::Char('e') => { self.app_state = AppState::Editing(0)  }
            KeyCode::Enter     => { self.app_state = AppState::Counting    }
            KeyCode::Up => {
                let mut selected = self.c_state.selected().unwrap();
                selected += len - 1;
                selected %= len;
                self.c_state.select(Some(selected as usize));
            }
            KeyCode::Down => {
                let mut selected = self.c_state.selected().unwrap();
                selected += 1;
                selected %= len;
                self.c_state.select(Some(selected as usize));
            }
            _ => {}
        }
    }

    fn editing_key_event(&mut self, key: KeyCode, stage: u8) {
        match stage {
            0 => {
                match key {
                    KeyCode::Enter => { 
                        let counter = self.c_store.get_mut(self.c_state.selected().unwrap_or(0)).unwrap();
                        counter.set_name(&self.entry_state.get_field());
                        self.entry_state = EntryState::default();
                        self.app_state = AppState::Editing(1) 
                    }
                    _ => self.rename_key_event(key)
                }
            }
            1 => {
                match key {
                    KeyCode::Enter => { 
                        let counter = self.c_store.get_mut(self.c_state.selected().unwrap_or(0)).unwrap();
                        counter.set_count(self.entry_state.get_field().parse().unwrap_or(counter.get_count()));
                        self.entry_state = EntryState::default();
                        self.app_state = AppState::Editing(2) 
                    }
                    _ => self.rename_key_event(key)
                }
            }
            2 => {
                match key {
                    KeyCode::Enter => { 
                        let counter = self.c_store.get_mut(self.c_state.selected().unwrap_or(0)).unwrap();
                        counter.set_time(self.entry_state.get_field().parse().unwrap_or(counter.get_time().as_secs() / 60));
                        self.entry_state = EntryState::default();
                        self.app_state = AppState::Selection 
                    }
                    _ => self.rename_key_event(key)
                }
            }
            3.. => unreachable!()
        }
    }

    fn rename_key_event(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char(charr) if charr.is_ascii() => {
                self.entry_state.push(charr)
            }
            KeyCode::Backspace => {
                self.entry_state.pop()
            }
            KeyCode::Enter => {
                let counter = self.c_store.get_mut(self.c_state.selected().unwrap_or(0)).unwrap();
                counter.set_name(&self.entry_state.get_field());
                self.entry_state = EntryState::default();
                self.app_state = AppState::Selection;
            }
            KeyCode::Esc => { 
                self.app_state = AppState::Selection;
                self.entry_state = EntryState::default();
            }
            _ => {}
        }
    }

    fn change_count_key_event(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char(charr) if charr.is_numeric() => {
                self.entry_state.push(charr)
            }
            KeyCode::Backspace => {
                self.entry_state.pop()
            }
            KeyCode::Enter => {
                let counter = self.c_store.get_mut(self.c_state.selected().unwrap_or(0)).unwrap();
                counter.set_count(self.entry_state.get_field().parse().unwrap_or(counter.get_count()));
                self.entry_state = EntryState::default();
                self.app_state = AppState::Selection;
            }
            KeyCode::Esc => { 
                self.app_state = AppState::Selection;
                self.entry_state = EntryState::default();
            }
            _ => {}
        }
    }

    fn change_time_key_event(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char(charr) if charr.is_numeric() => {
                self.entry_state.push(charr)
            }
            KeyCode::Backspace => {
                self.entry_state.pop()
            }
            KeyCode::Enter => {
                let counter = self.c_store.get_mut(self.c_state.selected().unwrap_or(0)).unwrap();
                counter.set_time(self.entry_state.get_field().parse().unwrap_or(counter.get_time().as_secs() / 60));
                self.entry_state = EntryState::default();
                self.app_state = AppState::Selection;
            }
            KeyCode::Esc => { 
                self.app_state = AppState::Selection;
                self.entry_state = EntryState::default();
            }
            _ => {}
        }
    }
}
