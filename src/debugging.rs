use std::{collections::HashMap, time::Instant};

use crate::app::AppError;

#[macro_export]
macro_rules! errplace {
    () => {
        format!("{}:{}:{}", file!(), line!(), column!())
    };
}

#[derive(Clone, PartialEq, Eq, Hash)]
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
    info: HashMap<DebugKey, DebugMessage>,
    warning: Vec<DebugMessage>,
    window: DebugWindow,
}

impl DebugInfo {
    pub fn new() -> Self {
        Self {
            info: HashMap::new(),
            warning: vec![],
            window: DebugWindow::default(),
        }
    }

    pub fn handle_error(&mut self, error: AppError) {
        let msg: DebugMessage = error.into();
        self.info.insert(msg.clone().level, msg.clone());
        match &msg.level {
            DebugKey::Debug(_) => {},
            DebugKey::Info(_) => {},
            DebugKey::Warning(_) => {
                self.warning.push(msg)
            },
            DebugKey::Fatal(_) => {
                crate::app::cleanup_terminal_state().unwrap();
                eprintln!("{}", msg.to_string());
                panic!()
            }
        }
    }

    pub fn next_message(&self) -> String {
        todo!()
    }

    pub fn add_debug_message(&mut self, key: impl Into<String>, message: impl Into<String>) {
        let key = DebugKey::Debug(key.into());
        self.info
            .insert(key.clone(), DebugMessage::new(key, message));
    }
}

impl ToString for DebugInfo {
    fn to_string(&self) -> String {
        return self
            .info
            .iter()
            .map(|(_, msg)| msg.to_string())
            .collect::<String>();
    }
}

#[derive(Clone)]
struct DebugMessage {
    level: DebugKey,
    message: String,
    time: Instant,
}

impl DebugMessage {
    fn new(level: DebugKey, message: impl Into<String>) -> Self {
        return Self {
            level,
            message: message.into(),
            time: Instant::now(),
        };
    }
}

impl From<AppError> for DebugMessage {
    fn from(error: AppError) -> Self {
        match &error {
            AppError::GetCounterError(message) => Self {
                level: DebugKey::Fatal(error.to_string()),
                message: message.to_string(),
                time: Instant::now(),
            },
            AppError::GetPhaseError => todo!(),
            AppError::DevIoError(msg) => Self {
                level: DebugKey::Warning(error.to_string()),
                message: msg.to_string(),
                time: Instant::now(),
            },
            AppError::IoError(_) => todo!(),
            AppError::SettingNotFound => todo!(),
            AppError::InputThread => todo!(),
            AppError::ThreadError(_) => todo!(),
            AppError::ImpossibleState(_) => todo!(),
            AppError::ScreenSize(_) => todo!(),
            AppError::DialogAlreadyOpen(_) => todo!(),
            AppError::EventEmpty(_) => todo!(),
            AppError::SettingsType(_) => todo!(),
        }
    }
}

impl ToString for DebugMessage {
    fn to_string(&self) -> String {
        format!("{}: {}\n", self.level.to_string(), self.message)
    }
}

#[derive(Default)]
struct DebugWindow {}

#[cfg(test)]
mod test_debugging {
    use super::*;
    use crate::app::AppError;

    #[test]
    #[should_panic]
    fn test_error_into() {
        let mut debugger = DebugInfo::new();
        let error = AppError::GetCounterError(errplace!());
        debugger.handle_error(error);
    }
}
