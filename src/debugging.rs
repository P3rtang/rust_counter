use std::{collections::HashMap, process::exit};

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
            DebugKey::Debug(msg)   => format!("[DEBUG] {}", msg),
            DebugKey::Info(msg)    => format!("[INFO] {}" , msg),
            DebugKey::Warning(msg) => format!("[WARN] {}" , msg),
            DebugKey::Fatal(msg)   => format!("[FATAL] {}", msg),
        }
    }
}

#[derive(Default)]
pub struct DebugInfo {
    info: HashMap<DebugKey, DebugMessage>,
    window: DebugWindow,
}

impl DebugInfo {
    pub fn new() -> Self {
        Self { info: HashMap::new(), window: DebugWindow::default() }
    }

    pub fn handle_error(&mut self, error: AppError) {
        let msg: DebugMessage = error.into();
        self.info.insert(msg.clone().level, msg.clone());
        if let DebugKey::Fatal(_) = &msg.level {
            crate::app::cleanup_terminal_state().unwrap();
            println!("{}", msg.to_string());
            exit(2)
        }
    }

    pub fn next_message(&self) -> String {
        todo!()
    }

    pub fn add_debug_message(&mut self, key: impl Into<String>, message: impl Into<String>) {
        let key = DebugKey::Debug(key.into());
        self.info.insert(key.clone(), DebugMessage::new(key, message));
    }
}

impl ToString for DebugInfo {
    fn to_string(&self) -> String {
        return self.info.iter().map(|(_, msg)| msg.to_string()).collect::<String>()
    }
}

#[derive(Clone)]
struct DebugMessage {
    level: DebugKey,
    message: String,
}

impl DebugMessage {
    fn new(level: DebugKey, message: impl Into<String>) -> Self {
        return Self { level, message: message.into() }
    }
}

impl From<AppError> for DebugMessage {
    fn from(error: AppError) -> Self {
        match &error {
            AppError::GetCounterError(message) => Self {
                level: DebugKey::Fatal(error.to_string()),
                message: message.to_string(),
            },
            AppError::GetPhaseError => todo!(),
            AppError::DevIoError(_) => todo!(),
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
    use crate::app::AppError;
    use super::*;

    #[test]
    fn test_error_into() {
        let mut debugger = DebugInfo::new();
        let error = AppError::GetCounterError(errplace!());
        debugger.handle_error(error);
        assert_eq!(format!(""), debugger.next_message())
    }
}
