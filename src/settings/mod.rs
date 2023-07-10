use crate::{
    app::{AppError, AppState},
    input::{self, InputEventHandler, Key},
};
pub use item::ContentKey;
use item::MainContents;
use std::{io::Stdout, time::Duration};
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    Frame,
};

use self::item::ContentItem;

mod events;
mod item;
mod ui;

const TICK_RATE: u64 = 25;

pub struct Settings {
    keybinds: KeyMap,
    setting_items: MainContents,
    window: ui::SettingsWindow,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            keybinds: KeyMap::default(),
            setting_items: MainContents::new(),
            window: ui::SettingsWindow::new(),
        }
    }

    pub fn get_key_map(&self) -> KeyMap {
        return self.keybinds.clone();
    }

    pub fn get_settings(&self) -> &MainContents {
        return &self.setting_items;
    }

    pub fn get_tick_time(&self) -> Result<Duration, AppError> {
        Ok(Duration::from_millis(1000 / self.get_tick_rate()?))
    }

    pub fn get_tick_rate(&self) -> Result<u64, AppError> {
            Ok(self.setting_items
                .get_setting(ContentKey::TickRate)
                .ok_or(AppError::SettingNotFound)?
                .to_value()? as u64)
    }

    pub fn get_show_millis(&self) -> Result<bool, AppError> {
        self.setting_items
            .get_setting(ContentKey::ShowMillis)
            .ok_or(AppError::SettingNotFound)?
            .to_bool()
    }

    pub fn get_kbd_input(&self) -> Result<String, AppError> {
        Ok(self
            .setting_items
            .get_setting(ContentKey::ActKeyboard)
            .ok_or(AppError::SettingNotFound)?
            .to_string())
    }

    pub fn load_keyboards(&mut self) -> Result<(), AppError> {
        let setting = ContentItem::<String>::new(ContentKey::ActKeyboard, input::get_kbd_inputs()?);
        self.setting_items
            .set_setting(ContentKey::ActKeyboard, Box::new(setting));
        Ok(())
    }

    pub fn handle_event(
        &mut self,
        app_state: &AppState,
        event_handler: &Box<dyn InputEventHandler>,
    ) -> Result<(), AppError> {
        events::handle_event(self, app_state, event_handler)
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
    settings.window.draw(f, f.size(), &settings.setting_items)
}

pub fn draw_as_overlay(
    f: &mut Frame<CrosstermBackend<Stdout>>,
    settings: &Settings,
) -> Result<(), AppError> {
    let area = Layout::default()
        .vertical_margin(5)
        .horizontal_margin(20)
        .constraints(vec![Constraint::Min(20)])
        .split(f.size());
    settings.window.draw(f, area[0], &settings.setting_items)
}

pub trait ContentItemType: ToString + Clone + Default {}

impl ContentItemType for u32 {}
impl ContentItemType for bool {}
impl ContentItemType for String {}
