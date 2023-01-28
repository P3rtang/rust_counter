use std::time::Duration;
use crate::{input::Key, app::{AppError, App}};
use std::io::Stdout;
use tui::{backend::CrosstermBackend, Frame, layout::{Layout, Rect, Constraint}};

mod ui;

const TICK_RATE: u64 = 25;

pub struct Settings {
    pub tick_rate: Duration,
    pub time_show_millis: bool,
    pub act_kbd_path: String,
    keybinds: KeyMap,
    window: ui::SettingsWindow,
}

impl Settings {
    fn new(tick_rate: Duration, act_kbd_path: String) -> Self {
        Self { 
            tick_rate,
            time_show_millis: true,
            act_kbd_path,
            keybinds: KeyMap::default(),
            window: ui::SettingsWindow::default(),
        }
    }

    fn set_tick_rate(&mut self, rate: u64) {
        self.tick_rate = Duration::from_millis(1000 / rate);
    }

    pub fn get_key_map(&self) -> KeyMap {
        return self.keybinds.clone()
    }

    pub fn handle_event(&self, _app: &App) -> Result<(), AppError> {
        todo!()
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self { 
            tick_rate: Duration::from_millis(1000 / TICK_RATE),
            time_show_millis: true,
            act_kbd_path: String::default(),
            keybinds: KeyMap::default(),
            window: ui::SettingsWindow::default(),
        }
    }
}

#[derive(Clone)]
pub struct KeyMap {
    pub key_increase_counter: Vec<Key>,
    pub key_decrease_counter: Vec<Key>,
    pub key_toggle_keylogger: Vec<Key>,
}

impl Default for KeyMap {
    fn default() -> Self {
        Self {
            key_increase_counter: vec![Key::Char('+'), Key::Char('=')],
            key_decrease_counter: vec![Key::Char('-')],
            key_toggle_keylogger: vec![Key::Char('*')],
        }
    }
}

pub fn draw(f: &mut Frame<CrosstermBackend<Stdout>>, settings: &Settings) -> Result<(), AppError> {
    settings.window.draw(f, f.size())
}

pub fn draw_as_overlay(f: &mut Frame<CrosstermBackend<Stdout>>, settings: &Settings) -> Result<(), AppError> {
    let area = Layout::default().vertical_margin(5).horizontal_margin(20).constraints(vec![Constraint::Min(20)]).split(f.size());
    settings.window.draw(f, area[0])
}

pub fn draw_sized(_f: &mut Frame<CrosstermBackend<Stdout>>, _settings: &Settings, _area: Rect, _is_overlay: bool) -> Result<(), AppError> {
    todo!()
}
