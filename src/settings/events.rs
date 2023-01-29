use crossterm::event::KeyModifiers;

use super::ui::SettingsWindow;
use crate::{
    app::{AppError, AppMode, AppState},
    input::{EventHandler, EventType, Key},
};

pub fn handle_event(
    window: &mut SettingsWindow,
    app_state: &AppState,
    event_handler: &EventHandler,
) -> Result<(), AppError> {
    let event = if let Some(event) = event_handler.poll() {
        event
    } else {
        return Err(AppError::EventEmpty("".to_string()));
    };
    let key = if let EventType::KeyEvent(key) = event.clone().type_ {
        key
    } else {
        return Ok(());
    };
    if event.modifiers == KeyModifiers::NONE {
        match key {
            Key::Up => window.select_prev(),
            Key::Down => window.select_next(),
            Key::Char('q') | Key::Esc => {
                app_state.toggle_mode(AppMode::SETTINGS_OPEN);
            }
            _ => {}
        }
    } else if event.modifiers.contains(KeyModifiers::CONTROL) {
        match key {
            Key::Char('s') => {
                app_state.toggle_mode(AppMode::SETTINGS_OPEN);
            }
            _ => {},
        };
    }; 
    Ok(())
}
