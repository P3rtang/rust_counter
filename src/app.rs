use std::{io, time::{Instant, Duration}};
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
use crate::counter::{Counter, CounterStore};
use crate::ui;
use crate::entry::EntryState;

#[derive(Clone, PartialEq)]
pub enum AppState {
    Selection,
    Counting,
    AddingNew,
    Rename,
    ChangeCount,
}

pub struct App {
    tick_rate: Duration,
    c_store: CounterStore,
    c_state: ListState,
    entry_state: EntryState,
    app_state: AppState,
    cursor_pos: Option<(u16, u16)>,
}

impl App {
    pub fn new(tick_rate: u64, counter_store: CounterStore) -> Self {
        return App { 
            tick_rate: Duration::from_millis(tick_rate),
            c_store: counter_store,
            c_state: ListState::default(),
            entry_state: EntryState::default(),
            app_state: AppState::Selection,
            cursor_pos: None,
        }
    }
    pub fn start(&mut self) -> io::Result<()> {
        // setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).unwrap();

        let last_tick = Instant::now();
        let timeout = self.tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

        self.c_state.select(Some(0));

        loop {
            terminal.draw(|f| {
                ui::draw(f, &self.c_store, &mut self.c_state, self.app_state.clone(), &mut self.entry_state);
            })?;

            if let Some(pos) = self.cursor_pos {
                terminal.set_cursor(pos.0, pos.1).unwrap();
            }

            let len = self.c_store.len();
            
            if crossterm::event::poll(timeout).unwrap() {
                if let Event::Key(key) = event::read().unwrap() {
                    match self.app_state {
                        AppState::Selection if self.c_store.len() > 0 => {
                            match key.code {
                                KeyCode::Char('q') => return Ok(()),
                                KeyCode::Char('n') => { self.app_state = AppState::AddingNew }
                                KeyCode::Char('d') => {
                                    self.c_store.remove(self.c_state.selected().unwrap_or(0));
                                    if self.c_state.selected().unwrap_or(0) >= self.c_store.len() && self.c_store.len() > 0 {
                                        self.c_state.select(Some(self.c_store.len() - 1));
                                    }
                                }
                                KeyCode::Char('r') => { 
                                    self.app_state = AppState::Rename 
                                }
                                KeyCode::Char('s') => { 
                                    self.app_state = AppState::ChangeCount 
                                }
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
                                KeyCode::Enter => { 
                                    self.app_state = AppState::Counting 
                                }
                                _ => {}
                            }
                        }
                        AppState::Selection => {
                            match key.code {
                                KeyCode::Char('q') => return Ok(()),
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
                                KeyCode::Char(charr) if charr.is_alphanumeric() => { 
                                    self.entry_state.push(charr) 
                                }
                                KeyCode::Backspace => { self.entry_state.pop() }
                                _ => {}
                            }
                        }
                        AppState::Rename => {
                            match key.code {
                                KeyCode::Char(charr) if charr.is_alphanumeric() => {
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
                        AppState::ChangeCount => {
                            match key.code {
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
                    }
                }
            }
        }
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
}
