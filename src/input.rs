use crossterm::event::{DisableMouseCapture, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, LeaveAlternateScreen};
use std::char::CharTryFromError;
use std::collections::VecDeque;
use std::fmt::Display;
use std::io;
use std::process::exit;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU8, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, TryLockError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tui::backend::CrosstermBackend;
use tui::Terminal;

use crate::app::AppError;

const REPEAT_DELAY: Duration = Duration::from_millis(500);
const REPEAT_RATE: Duration = Duration::from_millis(50);
const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;

type EventStream = Arc<Mutex<VecDeque<Event>>>;
type HandlerModeThread = Arc<Mutex<AtomicU8>>;
type ThreadRunning = Arc<Mutex<AtomicBool>>;

// TODO: this file should maybe be its own module or even github project
pub trait InputEventHandler {
    fn init(&mut self) -> Result<(), ThreadError>;
    fn next_event(&self) -> Option<Event>;
    fn has_event(&self) -> bool;
    fn get_buffer(&self) -> VecDeque<Event>;
    fn simulate_key(&self, key: Key) -> Result<(), ThreadError>;
}

#[derive(Debug)]
pub enum ThreadError {
    ThreadStateLock,
    EventStreamLock,
}

impl From<PoisonError<MutexGuard<'_, AtomicBool>>> for ThreadError {
    fn from(_: PoisonError<MutexGuard<'_, AtomicBool>>) -> Self {
        Self::ThreadStateLock
    }
}

impl From<PoisonError<MutexGuard<'_, VecDeque<Event>>>> for ThreadError {
    fn from(_: PoisonError<MutexGuard<'_, VecDeque<Event>>>) -> Self {
        Self::EventStreamLock
    }
}

impl From<TryLockError<MutexGuard<'_, VecDeque<Event>>>> for ThreadError {
    fn from(_: TryLockError<MutexGuard<'_, VecDeque<Event>>>) -> Self {
        Self::EventStreamLock
    }
}

pub struct DevInput {
    fd: DevInputFileDescriptor,
    stream: EventStream,
    is_running: ThreadRunning,
}

impl DevInput {
    fn new(fd: i32) -> Self {
        return Self {
            fd: DevInputFileDescriptor::new(fd),
            stream: Arc::new(Mutex::new(VecDeque::new())),
            is_running: Arc::new(Mutex::new(AtomicBool::new(true))),
        };
    }
}

impl InputEventHandler for DevInput {
    fn init(&mut self) -> Result<(), ThreadError> {
        let fd = self.fd.clone();
        let stream = self.stream.clone();
        let is_running = self.is_running.clone();

        thread::spawn(move || {
            while is_running.lock().unwrap().load(Ordering::SeqCst) {
                if let Some(event) =
                    DevInputEvent::poll(-1, fd.0.lock().unwrap().load(Ordering::SeqCst) as i32)
                {
                    stream.lock().unwrap().push_back(event.into());
                }
            }
        });

        Ok(())
    }

    fn next_event(&self) -> Option<Event> {
        return self.stream.try_lock().ok()?.pop_front();
    }

    fn has_event(&self) -> bool {
        return self
            .stream
            .try_lock()
            .map(|l| l.len() != 0)
            .unwrap_or(false);
    }

    fn get_buffer(&self) -> VecDeque<Event> {
        return self
            .stream
            .try_lock()
            .map(|l| l.clone())
            .unwrap_or_default();
    }

    fn simulate_key(&self, key: Key) -> Result<(), ThreadError> {
        let event = Event {
            type_: EventType::KeyEvent(key),
            modifiers: KeyModifiers::NONE,
            time: Instant::now(),
        };

        self.stream.try_lock()?.push_back(event);
        Ok(())
    }
}

impl Drop for DevInput {
    fn drop(&mut self) {
        self.is_running.lock().unwrap().store(false, Ordering::SeqCst)
    }
}

pub struct CrossTermInput {
    stream: EventStream,
    is_running: ThreadRunning
}

impl CrossTermInput {
    fn new() -> Self {
        Self {
            stream: Arc::new(Mutex::new(VecDeque::new())),
            is_running: Arc::new(Mutex::new(AtomicBool::new(true)))
        }
    }
}

impl InputEventHandler for CrossTermInput {
    fn init(&mut self) -> Result<(), ThreadError> {
        let stream = self.stream.clone();
        let is_running = self.is_running.clone();

        thread::spawn(move || {
            while is_running.lock().unwrap().load(Ordering::SeqCst) {
                match crossterm::event::read() {
                    Ok(crossterm::event::Event::Key(key)) => {
                        let event = Event {
                            type_: EventType::KeyEvent(key.clone().into()),
                            modifiers: key.modifiers,
                            time: Instant::now(),
                        };
                        if key.code == KeyCode::Char('c')
                            && event.modifiers.intersects(KeyModifiers::CONTROL)
                        {
                            end().unwrap();
                            exit(2)
                        }
                        stream.lock().unwrap().push_back(event);
                    }
                    // TODO: integrate mouse events
                    _ => {}
                }
            }
        });

        Ok(())
    }

    fn next_event(&self) -> Option<Event> {
        return self.stream.try_lock().ok()?.pop_front();
    }

    fn has_event(&self) -> bool {
        return self
            .stream
            .try_lock()
            .map(|l| l.len() != 0)
            .unwrap_or(false);
    }

    fn get_buffer(&self) -> VecDeque<Event> {
        return self
            .stream
            .try_lock()
            .map(|l| l.clone())
            .unwrap_or_default();
    }

    fn simulate_key(&self, key: Key) -> Result<(), ThreadError> {
        let event = Event {
            type_: EventType::KeyEvent(key),
            modifiers: KeyModifiers::NONE,
            time: Instant::now(),
        };

        self.stream.try_lock()?.push_back(event);
        Ok(())
    }
}

impl Drop for CrossTermInput {
    fn drop(&mut self) {
        self.is_running.lock().unwrap().store(false, Ordering::SeqCst)
    }
}

pub struct Input {}

impl Input {
    pub fn crossterm() -> CrossTermInput {
        return CrossTermInput::new();
    }

    pub fn dev_input(fd: i32) -> DevInput {
        return DevInput::new(fd);
    }
}

#[derive(Debug, Clone)]
pub enum EventType {
    KeyEvent(Key),
    MouseEvent((u16, u16)),
}

#[derive(Debug, Default)]
pub struct DevInputFileDescriptor(Arc<Mutex<AtomicI32>>);

impl DevInputFileDescriptor {
    fn new(fd: i32) -> Self {
        Self(Arc::new(Mutex::new(AtomicI32::new(fd))))
    }

    #[cfg(target_os = "linux")]
    pub fn set_input(&mut self, file: &str) -> Result<(), AppError> {
        use nix::fcntl::{open, OFlag};

        let fd = open(
            file,
            OFlag::O_RDONLY | OFlag::O_NONBLOCK,
            nix::sys::stat::Mode::empty(),
        )?;
        self.0.lock()?.store(fd, Ordering::SeqCst);
        Ok(())
    }
}

impl Clone for DevInputFileDescriptor {
    fn clone(&self) -> Self {
        return Self(self.0.clone());
    }
}

#[cfg(target_os = "linux")]
pub fn get_kbd_inputs() -> Result<Vec<String>, AppError> {
    use std::fs;
    let input_files = fs::read_dir("/dev/input/by-id").unwrap();
    let files = input_files
        .into_iter()
        .map(|file| file.unwrap().file_name().to_str().unwrap_or("").to_string())
        .collect::<Vec<String>>()
        .into_iter()
        .filter(|f| f.contains("-event-kbd"))
        .filter(|f| !f.contains("-if01"))
        .collect();

    Ok(files)
}

#[cfg(target_os = "windows")]
pub fn get_kbd_inputs() -> Result<Vec<String>, AppError> {
    return Ok(vec![]);
}

#[cfg(not(target_os = "linux"))]
pub fn set_kbd(&self, _: &str) -> Result<(), AppError> {
    return Err(AppError::Platform(format!(
        "Not available on {}",
        std::env::consts::OS
    )));
}
fn end() -> io::Result<()> {
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
    Ok(())
}

#[derive(Debug, Clone)]
pub struct Event {
    pub type_: EventType,
    pub modifiers: KeyModifiers,
    pub time: Instant,
}

impl From<Key> for Event {
    fn from(value: Key) -> Self {
        return Self {
            type_: EventType::KeyEvent(value),
            modifiers: KeyModifiers::NONE,
            time: Instant::now(),
        };
    }
}

impl From<DevInputEvent> for Event {
    fn from(value: DevInputEvent) -> Self {
        Self {
            type_: EventType::KeyEvent(value.code.into()),
            modifiers: KeyModifiers::NONE,
            time: Instant::now(),
        }
    }
}

impl std::fmt::Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct DevInputEvent {
    pub time: Instant,
    pub type_: u16,
    pub code: u16,
    pub value: i32,
}

impl DevInputEvent {
    #[cfg(target_os = "linux")]
    pub fn poll(duration: i32, fd: i32) -> Option<Self> {
        use nix::{
            poll::{poll, PollFd, PollFlags},
            unistd::read,
        };

        let mut poll_fds = [PollFd::new(fd, PollFlags::POLLIN)];

        match poll(&mut poll_fds, duration) {
            Ok(n) => {
                if n > 0 {
                    let mut buf = [0u8; 24];
                    let _bytes_read = read(fd, &mut buf).unwrap();
                    let event: DevInputEvent = unsafe { std::mem::transmute(buf) };
                    if event.type_ == EV_KEY && event.value == 0 {
                        return Some(event);
                    } else {
                        return None;
                    }
                } else {
                    return None;
                }
            }
            Err(_e) => return None,
        }
    }

    #[cfg(not(target_os = "linux"))]
    pub fn poll(_duration: i32, _fd: i32) -> Option<Self> {
        return Some(Self::default());
    }
}

impl Display for DevInputEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Default for DevInputEvent {
    fn default() -> Self {
        return Self {
            time: Instant::now(),
            type_: 0,
            code: 0,
            value: 0,
        };
    }
}

pub fn get_fd(file: &str) -> i32 {
    use nix::fcntl::{open, OFlag};

    let path = format!("/dev/input/by-id/{}", file);

    let fd = open(
        path.as_str(),
        OFlag::O_RDONLY | OFlag::O_NONBLOCK,
        nix::sys::stat::Mode::empty(),
    )
    .unwrap_or(0);
    return fd;
}

#[derive(Debug, Clone, PartialEq)]
pub enum Key {
    Esc,
    Enter,
    Space,
    Backspace,
    Home,
    End,
    PageUp,
    PageDown,
    Tab,
    BackTab,
    Delete,
    Insert,
    F(u8),
    Char(char),
    Null,
    CapsLock,
    ScrollLock,
    NumLock,
    PrintScreen,
    Pause,
    Menu,
    KeypadBegin,
    Down,
    Up,
    Left,
    Right,
}

impl From<crossterm::event::KeyEvent> for Key {
    fn from(value: crossterm::event::KeyEvent) -> Self {
        match value.code {
            KeyCode::Backspace => Key::Backspace,
            KeyCode::Enter => Key::Enter,
            KeyCode::Left => Key::Left,
            KeyCode::Right => Key::Right,
            KeyCode::Up => Key::Up,
            KeyCode::Down => Key::Down,
            KeyCode::Home => Key::Home,
            KeyCode::End => Key::End,
            KeyCode::PageUp => Key::PageUp,
            KeyCode::PageDown => Key::PageDown,
            KeyCode::Tab => Key::Tab,
            KeyCode::BackTab => Key::BackTab,
            KeyCode::Delete => Key::Delete,
            KeyCode::Insert => Key::Insert,
            KeyCode::F(num) => Key::F(num),
            KeyCode::Char(char_) => Key::Char(char_),
            KeyCode::Null => Key::Null,
            KeyCode::Esc => Key::Esc,
            KeyCode::CapsLock => Key::CapsLock,
            KeyCode::ScrollLock => Key::ScrollLock,
            KeyCode::NumLock => Key::NumLock,
            KeyCode::PrintScreen => Key::PrintScreen,
            KeyCode::Pause => Key::Pause,
            KeyCode::Menu => Key::Menu,
            KeyCode::KeypadBegin => Key::KeypadBegin,

            _ => Key::Null,
            #[allow(unreachable_patterns)]
            KeyCode::Media(_) => unimplemented!(),
            #[allow(unreachable_patterns)]
            KeyCode::Modifier(_) => unimplemented!(),
        }
    }
}

impl From<u16> for Key {
    fn from(value: u16) -> Self {
        match value {
            1 => Key::Esc,
            12 => Key::Char('-'),
            13 => Key::Char('='),
            16 => Key::Char('q'),
            28 => Key::Enter,
            74 => Key::Char('-'),
            78 => Key::Char('+'),
            96 => Key::Enter,
            _ => Key::Null,
            // TODO: add more keys
        }
    }
}

impl From<Event> for Key {
    fn from(value: Event) -> Self {
        if let EventType::KeyEvent(key) = value.type_ {
            key
        } else {
            Key::Null
        }
    }
}

impl TryInto<char> for Key {
    type Error = CharTryFromError;
    fn try_into(self) -> Result<char, Self::Error> {
        match self {
            Key::Char(char_) => return Ok(char_),
            _ => panic!(),
        }
    }
}

impl Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[cfg(test)]
mod input_test {
    use super::*;

    #[test]
    fn test_simulate_key() {
        let event_handler = CrossTermInput::new();
        event_handler.simulate_key(Key::Char('h'));
        let key: Key = event_handler.next_event().unwrap().into();
        assert_eq!(key, Key::Char('h'));
        assert_ne!(key, Key::Enter);
        event_handler.simulate_key(Key::Char('f'));
        event_handler.simulate_key(Key::Char('o'));
        event_handler.simulate_key(Key::Char('o'));
        let mut word = String::new();
        while event_handler.has_event() {
            let key: Key = event_handler.next_event().unwrap().into();
            word.push(key.try_into().unwrap())
        }
        assert_eq!(word, "foo".to_string())
    }
}
