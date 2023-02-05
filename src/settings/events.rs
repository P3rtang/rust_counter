use crate::input::Event;
use crate::settings::Settings;
use crate::{
    app::{AppError, AppMode, AppState},
    input::{EventHandler, EventType, Key},
};
use crossterm::event::KeyModifiers;
use super::ui::WindowState;

pub enum WindowEvent {
    NoEvent,
    ExitWindow,
}

pub fn handle_event(
    settings: &mut Settings,
    app_state: &AppState,
    event_handler: &EventHandler,
) -> Result<(), AppError> {
    // get event and key from the event handler module
    let event = if let Some(event) = event_handler.poll() {
        event
    } else {
        return Err(AppError::EventEmpty("".to_string()));
    };
    let cur_state = settings.window.get_state();
    let window_event = match cur_state {
        WindowState::Default => handle_default(settings, app_state, event)?,
        WindowState::SubMenu(key) => settings
            .setting_items
            .get_mut_setting(key)
            .ok_or(AppError::SettingNotFound)?
            .handle_event(event),
    };
    match window_event {
        WindowEvent::NoEvent => {}
        WindowEvent::ExitWindow => settings.window.set_state(WindowState::Default),
    }
    Ok(())
}

fn handle_default(
    settings: &mut Settings,
    app_state: &AppState,
    event: Event,
) -> Result<WindowEvent, AppError> {
    // match the event modifiers keys and key character pressed
    let key = if let EventType::KeyEvent(key) = event.clone().type_ {
        key
    } else {
        return Ok(WindowEvent::NoEvent);
    };
    if event.modifiers == KeyModifiers::NONE {
        match key {
            Key::Up | Key::Char('k') => settings.window.select_prev(settings.setting_items.contents.len()),
            Key::Down | Key::Char('j') => settings.window.select_next(settings.setting_items.contents.len()),
            Key::Enter => {
                let state = settings.window.get_selected_key()?;
                settings.window.set_state(WindowState::SubMenu(state));
            }
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
            _ => {}
        };
    };
    Ok(WindowEvent::NoEvent)
}
