use crate::{app::AppError, ui::*};
use chrono::{DateTime, Local};
use std::collections::VecDeque;
use tui::{
    backend::Backend,
    layout::{Constraint, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

#[macro_export]
macro_rules! errplace {
    () => {
        format!("{}:{}:{}", file!(), line!(), column!())
    };
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DebugKey {
    Debug(String),
    Info(String),
    Warning(String),
    Fatal(String),
}

impl ToString for DebugKey {
    fn to_string(&self) -> String {
        match self {
            DebugKey::Debug(msg) => format!("[DEBUG] {}", msg),
            DebugKey::Info(msg) => format!("[INFO] {}", msg),
            DebugKey::Warning(msg) => format!("[WARN] {}", msg),
            DebugKey::Fatal(msg) => format!("[FATAL] {}", msg),
        }
    }
}

#[derive(Default)]
pub struct DebugInfo {
    messages: Vec<DebugMessage>,
    new_messages: VecDeque<DebugMessage>,
}

impl DebugInfo {
    pub fn new() -> Self {
        Self {
            messages: vec![],
            new_messages: VecDeque::new(),
        }
    }

    pub fn handle_error(&mut self, error: AppError) {
        let msg: DebugMessage = error.into();
        self.insert(msg.clone());
        match &msg.key {
            DebugKey::Debug(_) => {}
            DebugKey::Info(_) => {}
            DebugKey::Warning(_) => self.new_messages.push_back(msg),
            DebugKey::Fatal(_) => {
                crate::app::cleanup_terminal_state().unwrap();
                eprintln!("{}", msg.to_string());
                panic!()
            }
        }
    }

    fn insert(&mut self, msg: DebugMessage) {
        match self.get_key_idx(&msg.key) {
            Some(idx) => self.messages[idx] = msg,
            None => self.messages.push(msg),
        }
        self.messages
            .sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap())
    }

    fn get_key_idx(&self, key: &DebugKey) -> Option<usize> {
        return self.messages.iter().rposition(|msg| &msg.key == key);
    }

    fn next_message(&mut self) -> Option<DebugMessage> {
        self.new_messages.pop_front()
    }

    pub fn add_debug_message(&mut self, key: impl Into<String>, message: impl Into<String>) {
        let key = DebugKey::Debug(key.into());
        self.insert(DebugMessage::new(key, message));
    }

    fn to_table(&self, is_colored: bool) -> Table {
        let rows = self.messages.iter().map(|msg| msg.to_row(is_colored));
        return Table::new(rows)
            .widths(&[
                Constraint::Length(8),
                Constraint::Length(7),
                Constraint::Percentage(80),
            ])
            .column_spacing(2);
    }
}

impl ToString for DebugInfo {
    fn to_string(&self) -> String {
        return self
            .messages
            .iter()
            .map(|msg| msg.to_string())
            .collect::<String>();
    }
}

#[derive(Debug, Clone)]
struct DebugMessage {
    key: DebugKey,
    message: String,
    time: DateTime<Local>,
}

impl DebugMessage {
    fn new(level: DebugKey, message: impl Into<String>) -> Self {
        return Self {
            key: level,
            message: message.into(),
            time: Local::now(),
        };
    }

    fn set_time(&mut self, time: DateTime<Local>) {
        self.time = time
    }

    fn to_row(&self, is_colored: bool) -> Row {
        let cells: [Cell; 3] = match is_colored {
            true => match self.key {
                DebugKey::Debug(_) => [
                    Cell::from(instant_to_string(self.time)),
                    Cell::from(self.key.to_string()).style(Style::default().fg(Color::Yellow)),
                    Cell::from(self.message.to_string()),
                ],
                DebugKey::Info(_) => [
                    Cell::from(instant_to_string(self.time)),
                    Cell::from(self.key.to_string()).style(Style::default().fg(Color::White)),
                    Cell::from(self.message.to_string()),
                ],
                DebugKey::Warning(_) => [
                    Cell::from(instant_to_string(self.time)),
                    Cell::from(self.key.to_string()).style(Style::default().fg(ORANGE)),
                    Cell::from(self.message.to_string()),
                ],
                DebugKey::Fatal(_) => [
                    Cell::from(instant_to_string(self.time)),
                    Cell::from(self.key.to_string()).style(Style::default().fg(Color::Red)),
                    Cell::from(self.message.to_string()),
                ],
            },
            false => [
                instant_to_string(self.time).into(),
                self.key.to_string().into(),
                self.message.to_string().into(),
            ],
        };
        return Row::new(cells);
    }
}

impl From<AppError> for DebugMessage {
    fn from(error: AppError) -> Self {
        match &error {
            AppError::GetCounterError(message) => Self {
                key: DebugKey::Fatal(error.to_string()),
                message: message.to_string(),
                time: Local::now(),
            },
            AppError::GetPhaseError => todo!(),
            AppError::DevIoError(msg) => Self {
                key: DebugKey::Warning(error.to_string()),
                message: msg.to_string(),
                time: Local::now(),
            },
            AppError::IoError(_) => todo!(),
            AppError::SettingNotFound => todo!(),
            AppError::InputThread => todo!(),
            AppError::ThreadError(msg) => Self {
                key: DebugKey::Fatal(error.to_string()),
                message: msg.to_string(),
                time: Local::now(),
            },
            AppError::ImpossibleState(_) => todo!(),
            AppError::ScreenSize(msg) => Self {
                key: DebugKey::Info(error.to_string()),
                message: msg.to_string(),
                time: Local::now(),
            },
            AppError::DialogAlreadyOpen(_) => todo!(),
            AppError::EventEmpty(_) => todo!(),
            AppError::SettingsType(_) => todo!(),
            AppError::Platform(msg) => Self {
                key: DebugKey::Warning(error.to_string()),
                message: msg.to_string(),
                time: Local::now(),
            },
        }
    }
}

impl ToString for DebugMessage {
    fn to_string(&self) -> String {
        format!("{}: {}", self.key.to_string(), self.message)
    }
}

fn instant_to_string(instant: DateTime<Local>) -> String {
    format!("{}", instant.format("%H:%M:%S"))
}

#[derive(Default)]
pub struct DebugWindow {
    pub debug_info: DebugInfo,
    style: Style,
    border_style: Style,
    is_colored: bool,
}

impl DebugWindow {
    pub fn toggle_color(mut self) -> Self {
        self.is_colored = !self.is_colored;
        self
    }
    pub fn draw<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.border_style);
        let widget = self
            .debug_info
            .to_table(self.is_colored)
            .style(self.style)
            .block(block);
        f.render_widget(widget, area)
    }
}

#[cfg(test)]
mod test_debugging {
    use tui::{backend::TestBackend, buffer::Buffer, Terminal};

    use super::*;
    use crate::app::AppError;

    #[test]
    fn test_error_stringify() {
        let mut debugger = DebugInfo::new();
        let error = AppError::DevIoError("src/debugging:180:20 `error`".to_string());
        debugger.handle_error(error);
        assert_eq!(
            "[WARN] DevIoError: src/debugging:180:20 `error`".to_string(),
            debugger.next_message().unwrap().to_string()
        );
    }
    #[test]
    #[should_panic]
    fn test_error_fatal() {
        let mut debugger = DebugInfo::new();
        let error = AppError::GetCounterError(errplace!());
        debugger.handle_error(error);
    }
    #[test]
    fn test_multiple_errors() {
        let mut debugger = DebugInfo::new();
        let error = AppError::DevIoError(errplace!());
        debugger.handle_error(error);
        assert!(debugger.next_message().is_some());
        assert!(debugger.next_message().is_none());
        assert_eq!(1, debugger.messages.len());

        let error = AppError::DevIoError(errplace!());
        debugger.handle_error(error);
        let error = AppError::DevIoError(errplace!());
        debugger.handle_error(error);

        assert_eq!(1, debugger.messages.len());
        assert!(debugger.next_message().is_some());
        assert!(debugger.next_message().is_some());
    }
    #[test]
    fn test_draw_ui() {
        let test_case = |window: &DebugWindow, expected| {
            let backend = TestBackend::new(60, 10);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal.draw(|f| window.draw(f, f.size())).unwrap();
            terminal.backend().assert_buffer(&expected);
        };
        let mut debug_window = DebugWindow::default();

        debug_window.debug_info.handle_error(AppError::DevIoError(
            "src/debugging:180:20 `error`".to_string(),
        ));
        test_case(
            &debug_window,
            Buffer::with_lines(vec![
                format!("┌──────────────────────────────────────────────────────────┐"),
                format!(
                    "│{      }  [WARN]   src/debugging:180:20 `error`           │",
                    instant_to_string(Local::now())
                ),
                format!("│                                                          │"),
                format!("│                                                          │"),
                format!("│                                                          │"),
                format!("│                                                          │"),
                format!("│                                                          │"),
                format!("│                                                          │"),
                format!("│                                                          │"),
                format!("└──────────────────────────────────────────────────────────┘"),
            ]),
        );

        debug_window.debug_info.handle_error(AppError::ScreenSize(
            "src/ui.rs:140:34 `screensize too small`".to_string(),
        ));
        test_case(
            &debug_window,
            Buffer::with_lines(vec![
                format!("┌──────────────────────────────────────────────────────────┐"),
                format!(
                    "│{      }  [WARN]   src/debugging:180:20 `error`           │",
                    instant_to_string(Local::now())
                ),
                format!(
                    "│{      }  [INFO]   src/ui.rs:140:34 `screensize too small`│",
                    instant_to_string(Local::now())
                ),
                format!("│                                                          │"),
                format!("│                                                          │"),
                format!("│                                                          │"),
                format!("│                                                          │"),
                format!("│                                                          │"),
                format!("│                                                          │"),
                format!("└──────────────────────────────────────────────────────────┘"),
            ]),
        );

        debug_window.debug_info.add_debug_message(
            "testing",
            "testing too long debug message this should not wrap",
        );
        test_case(
            &debug_window,
            Buffer::with_lines(vec![
                format!("┌──────────────────────────────────────────────────────────┐"),
                format!(
                    "│{      }  [WARN]   src/debugging:180:20 `error`           │",
                    instant_to_string(Local::now())
                ),
                format!(
                    "│{      }  [INFO]   src/ui.rs:140:34 `screensize too small`│",
                    instant_to_string(Local::now())
                ),
                format!(
                    "│{      }  [DEBUG]  testing too long debug message this sho│",
                    instant_to_string(Local::now())
                ),
                format!("│                                                          │"),
                format!("│                                                          │"),
                format!("│                                                          │"),
                format!("│                                                          │"),
                format!("│                                                          │"),
                format!("└──────────────────────────────────────────────────────────┘"),
            ]),
        );

        debug_window.debug_info.handle_error(AppError::DevIoError(
            "src/debugging:180:20 `error`".to_string(),
        ));
        test_case(
            &debug_window,
            Buffer::with_lines(vec![
                format!("┌──────────────────────────────────────────────────────────┐"),
                format!(
                    "│{      }  [INFO]   src/ui.rs:140:34 `screensize too small`│",
                    instant_to_string(Local::now())
                ),
                format!(
                    "│{      }  [DEBUG]  testing too long debug message this sho│",
                    instant_to_string(Local::now())
                ),
                format!(
                    "│{      }  [WARN]   src/debugging:180:20 `error`           │",
                    instant_to_string(Local::now())
                ),
                format!("│                                                          │"),
                format!("│                                                          │"),
                format!("│                                                          │"),
                format!("│                                                          │"),
                format!("│                                                          │"),
                format!("└──────────────────────────────────────────────────────────┘"),
            ]),
        );
    }
}
