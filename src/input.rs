use crossterm::event::{DisableMouseCapture, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, LeaveAlternateScreen};
use nix::fcntl::{open, OFlag};
use nix::poll::{poll, PollFd, PollFlags};
use nix::unistd::read;
use std::char::CharTryFromError;
use std::fmt::Display;
use std::fs::DirEntry;
use std::process::exit;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU8, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use std::{fs, io};
use tui::backend::CrosstermBackend;
use tui::Terminal;

use crate::app::AppError;

const REPEAT_DELAY: Duration = Duration::from_millis(500);
const REPEAT_RATE: Duration = Duration::from_millis(50);
const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;

type EventStream = Arc<Mutex<Vec<Event>>>;
type HandlerModeThread = Arc<Mutex<AtomicU8>>;
type ThreadRunning = Arc<Mutex<AtomicBool>>;

// TODO: this file should maybe be its own module or even github project

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

impl From<PoisonError<MutexGuard<'_, Vec<Event>>>> for ThreadError {
    fn from(_: PoisonError<MutexGuard<'_, Vec<Event>>>) -> Self {
        Self::EventStreamLock
    }
}

#[derive(Debug, Clone)]
pub enum EventType {
    KeyEvent(Key),
    MouseEvent((u16, u16)),
}

#[derive(Debug, Clone, PartialEq)]
pub enum HandlerMode {
    DevInput,
    Terminal,
}

impl From<Arc<Mutex<AtomicU8>>> for HandlerMode {
    fn from(value: Arc<Mutex<AtomicU8>>) -> Self {
        match value.lock().unwrap().load(Ordering::SeqCst) {
            0 => HandlerMode::DevInput,
            1 => HandlerMode::Terminal,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Default)]
pub struct DevInputFileDescriptor {
    file_descriptor: Arc<Mutex<AtomicI32>>,
    active_file: (i32, i32),
}

impl DevInputFileDescriptor {
    fn new(fd: i32) -> Self {
        Self {
            file_descriptor: Arc::new(Mutex::new(AtomicI32::new(fd))),
            active_file: (0, 0),
        }
    }

    pub fn set_input(&mut self, file: &str) -> Result<(), AppError> {
        let fd = open(
            file,
            OFlag::O_RDONLY | OFlag::O_NONBLOCK,
            nix::sys::stat::Mode::empty(),
        )?;
        self.file_descriptor.lock()?.store(fd, Ordering::SeqCst);
        Ok(())
    }
}

pub fn get_kbd_inputs() -> Result<Vec<String>, AppError> {
    let input_files = fs::read_dir("/dev/input/by-id").unwrap();
    let process_file = |file: Result<DirEntry, io::Error>| -> Result<String, AppError> {
        let rtn_file = file?
            .file_name()
            .to_str()
            .ok_or(AppError::DevIoError(
                "cannot read from /dev/input/".to_string(),
            ))?
            .to_string();
        Ok(rtn_file)
    };
    let files = input_files
        .into_iter()
        .map(process_file)
        .try_collect::<Vec<String>>()?
        .into_iter()
        .filter(|f| f.contains("-event-kbd"))
        .filter(|f| !f.contains("-if01"))
        .collect();

    Ok(files)
}

pub struct EventHandler {
    mode: HandlerModeThread,
    file_descriptor: DevInputFileDescriptor,
    event_stream: EventStream,
    thread_terminal: JoinHandle<()>,
    thread_running_state: ThreadRunning,
}

impl EventHandler {
    pub fn new() -> Self {
        let mode = Arc::new(Mutex::new(AtomicU8::new(HandlerMode::Terminal as u8)));
        let file_descriptor = DevInputFileDescriptor::default();
        let event_stream: EventStream = Arc::new(Mutex::new(vec![]));
        let thread_running_state = Arc::new(Mutex::new(AtomicBool::new(false)));
        let thread_terminal = EventHandler::spawn_thread(
            event_stream.clone(),
            mode.clone(),
            file_descriptor.file_descriptor.clone(),
            thread_running_state.clone(),
        );
        Self {
            mode,
            file_descriptor,
            event_stream,
            thread_terminal,
            thread_running_state,
        }
    }
    fn spawn_thread(
        event_stream: EventStream,
        mode: HandlerModeThread,
        fd: Arc<Mutex<AtomicI32>>,
        thread_running_state: ThreadRunning,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            while thread_running_state.lock().unwrap().load(Ordering::SeqCst) {
                match mode.clone().into() {
                    HandlerMode::Terminal => {
                        match crossterm::event::read().unwrap() {
                            crossterm::event::Event::Key(key) => {
                                let event = Event {
                                    type_: EventType::KeyEvent(key.clone().into()),
                                    modifiers: key.modifiers,
                                    time: Instant::now(),
                                    mode: HandlerMode::Terminal,
                                };
                                if key.code == KeyCode::Char('c')
                                    && event.modifiers.intersects(KeyModifiers::CONTROL)
                                {
                                    end().unwrap();
                                    exit(2)
                                }
                                event_stream.lock().unwrap().insert(0, event);
                            }
                            // TODO: integrate mouse events
                            crossterm::event::Event::Mouse(_) => {}
                            _ => {}
                        }
                    }
                    HandlerMode::DevInput => {
                        if crossterm::event::poll(Duration::from_millis(0)).unwrap() {
                            match crossterm::event::read().unwrap() {
                                crossterm::event::Event::Key(key) => {
                                    if key.code == KeyCode::Char('c')
                                        && key.modifiers.intersects(KeyModifiers::CONTROL)
                                    {
                                        end().unwrap();
                                        exit(2)
                                    }
                                }
                                _ => {}
                            }
                        }
                        if let Some(event) = DevInputEvent::poll(
                            -1,
                            fd.lock().unwrap().load(Ordering::SeqCst) as i32,
                        ) {
                            event_stream.lock().unwrap().insert(0, event.into())
                        }
                        // TODO: use crossterm mouse events in this context
                    }
                }
            }
        })
    }
    pub fn toggle_mode(&self) {
        if self
            .file_descriptor
            .file_descriptor
            .lock()
            .unwrap()
            .load(Ordering::SeqCst)
            == 0
        {
            return;
        }
        self.clear();
        match self.mode.clone().into() {
            HandlerMode::Terminal => self.mode.lock().unwrap().store(0, Ordering::SeqCst),
            HandlerMode::DevInput => self.mode.lock().unwrap().store(1, Ordering::SeqCst),
        }
    }
    pub fn set_mode(&self, mode: HandlerMode) {
        match mode {
            HandlerMode::DevInput => self.mode.lock().unwrap().store(0, Ordering::SeqCst),
            HandlerMode::Terminal => self.mode.lock().unwrap().store(1, Ordering::SeqCst),
        }
    }

    pub fn get_buffer(&self) -> Vec<Event> {
        return self.event_stream.lock().unwrap().clone();
    }
    pub fn poll(&self) -> Option<Event> {
        if self.event_stream.lock().unwrap().len() == 0 {
            return None;
        } else if self.event_stream.lock().unwrap().last().unwrap().mode != self.mode.clone().into()
        {
            let _ = self.event_stream.lock().unwrap().pop();
            return None;
        }
        return Some(self.event_stream.lock().unwrap().pop().unwrap());
    }

    pub fn has_event(&self) -> bool {
        match self.event_stream.lock() {
            Ok(stream) => stream.len() != 0,
            Err(_) => false,
        }
    }

    pub fn set_kbd(&self, file: &str) -> Result<(), AppError> {
        let fd = open(
            format!("/dev/input/by-id/{}", file).as_str(),
            OFlag::O_RDONLY | OFlag::O_NONBLOCK,
            nix::sys::stat::Mode::empty(),
        )?;
        self.set_fd(fd)
    }

    pub fn set_fd(&self, fd: i32) -> Result<(), AppError> {
        self.file_descriptor
            .file_descriptor
            .lock()?
            .store(fd, Ordering::SeqCst);
        Ok(())
    }
    pub fn start(&mut self) -> Result<(), ThreadError> {
        self.thread_running_state
            .lock()?
            .store(true, Ordering::SeqCst);
        self.thread_terminal = Self::spawn_thread(
            self.event_stream.clone(),
            self.mode.clone(),
            self.file_descriptor.file_descriptor.clone(),
            self.thread_running_state.clone(),
        );
        Ok(())
    }
    pub fn stop(&mut self) {
        self.thread_running_state
            .lock()
            .unwrap()
            .store(false, Ordering::SeqCst)
    }
    fn clear(&self) {
        self.event_stream.lock().unwrap().clear();
    }

    pub fn simulate_key(&self, key: Key) {
        self.event_stream.lock().unwrap().insert(0, Event {
            type_: EventType::KeyEvent(key),
            modifiers: KeyModifiers::NONE,
            time: Instant::now(),
            mode: HandlerMode::Terminal,
        })
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for EventHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "event: {:?}", self.event_stream.lock().unwrap())
    }
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
    mode: HandlerMode,
}

impl From<Key> for Event {
    fn from(value: Key) -> Self {
        return Self {
            type_: EventType::KeyEvent(value),
            modifiers: KeyModifiers::NONE,
            time: Instant::now(),
            mode: HandlerMode::Terminal,
        };
    }
}

impl From<DevInputEvent> for Event {
    fn from(value: DevInputEvent) -> Self {
        Self {
            type_: EventType::KeyEvent(value.code.into()),
            modifiers: KeyModifiers::NONE,
            time: Instant::now(),
            mode: HandlerMode::DevInput,
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
    pub fn poll(duration: i32, fd: i32) -> Option<Self> {
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
}

impl Display for DevInputEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
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
            _ => panic!()
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
        let event_handler = EventHandler::default();
        event_handler.simulate_key(Key::Char('h'));
        let key: Key = event_handler.poll().unwrap().into();
        assert_eq!(key, Key::Char('h'));
        assert_ne!(key, Key::Enter);
        event_handler.simulate_key(Key::Char('f'));
        event_handler.simulate_key(Key::Char('o'));
        event_handler.simulate_key(Key::Char('o'));
        let mut word = String::new();
        while event_handler.has_event() {
            let key: Key = event_handler.poll().unwrap().into();
            word.push(key.try_into().unwrap())
        }
        assert_eq!(word, "foo".to_string())
    }
}
