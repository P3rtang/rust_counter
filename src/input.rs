use crossterm::event::KeyCode;
use nix::unistd::read;
use nix::poll::{poll, PollFd, PollFlags};
use std::fmt::Display;
use std::sync::atomic::{AtomicU8, Ordering, AtomicI8};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle, sleep};
use std::time::{Instant, Duration};

use crate::app::App;

const REPEAT_DELAY: Duration = Duration::from_millis(500);
const REPEAT_RATE : Duration = Duration::from_millis(50);
const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;

enum Modifier {
    Shift,
    Esc,
    Control,
}

#[derive(Debug, Clone)]
pub enum EventType {
    KeyEvent(Key),
    MouseEvent((u16, u16)),
}

#[derive(Debug, Clone, PartialEq)]
enum HandlerMode {
    DevInput,
    Terminal,
}

impl From<Arc<Mutex<AtomicU8>>> for HandlerMode {
    fn from(value: Arc<Mutex<AtomicU8>>) -> Self {
        match value.lock().unwrap().load(Ordering::SeqCst) {
            0 => HandlerMode::DevInput,
            1 => HandlerMode::Terminal,
            _ => unreachable!()
        }
    }
}

type EventStream = Arc<Mutex<Vec<Event>>>;

pub struct EventHandler {
    mode: Arc<Mutex<AtomicU8>>,
    file_descriptor: Arc<Mutex<AtomicI8>>,
    event_stream: EventStream,
    thread_terminal: JoinHandle<()>,
}

impl EventHandler {
    pub fn new(fd: i32) -> Self {
        let mode = Arc::new(Mutex::new(AtomicU8::new(HandlerMode::Terminal as u8)));
        let file_descriptor = Arc::new(Mutex::new(AtomicI8::new(fd as i8)));
        let event_stream: EventStream = Arc::new(Mutex::new(vec![]));
        let thread_terminal = EventHandler::spawn_thread(event_stream.clone(), mode.clone(), file_descriptor.clone());
        Self { 
            mode,
            file_descriptor,
            event_stream,
            thread_terminal,
        }
    }
    fn spawn_thread(event_stream: EventStream, mode: Arc<Mutex<AtomicU8>> , fd: Arc<Mutex<AtomicI8>>) -> JoinHandle<()> {
        thread::spawn(move || {
            loop {
                match mode.clone().into() {
                    HandlerMode::Terminal => {
                        match crossterm::event::read().unwrap() {
                            crossterm::event::Event::FocusGained => {}
                            crossterm::event::Event::FocusLost => {}
                            crossterm::event::Event::Key(key) => {
                                let event = Event { 
                                    type_: EventType::KeyEvent(key.into()),
                                    time: Instant::now(),
                                    mode: HandlerMode::Terminal,
                                };
                                event_stream.lock().unwrap().insert(0, event)
                            }
                            crossterm::event::Event::Mouse(_) => {}
                            crossterm::event::Event::Paste(_) => {}
                            crossterm::event::Event::Resize(_, _) => {}
                        }
                    }
                    HandlerMode::DevInput => {
                        if let Some(event) = InputEvent::poll(0, fd.lock().unwrap().load(Ordering::SeqCst) as i32) {
                            event_stream.lock().unwrap().insert(0, event.into())
                        }
                    }
                }
            }
        })
    }
    pub fn toggle_mode(&mut self) {
        if self.file_descriptor.lock().unwrap().load(Ordering::SeqCst) == 0 { return }
        match self.mode.clone().into() {
            HandlerMode::DevInput => self.mode.lock().unwrap().store(1, Ordering::SeqCst),
            HandlerMode::Terminal => self.mode.lock().unwrap().store(0, Ordering::SeqCst),
        }
    }
    pub fn poll(&self) -> Option<Event> {
        if self.event_stream.lock().unwrap().len() == 0 {
            panic!()
        } else if self.event_stream.lock().unwrap().last().unwrap().mode != self.mode.clone().into() {
            let _ = self.event_stream.lock().unwrap().pop();
            return None
        }
        return Some(self.event_stream.lock().unwrap().pop().unwrap())
    }
    pub fn has_event(&self) -> bool {
        return self.event_stream.lock().unwrap().len() != 0
    }
    pub fn set_fd(&self, fd: i32) {
        self.file_descriptor.lock().unwrap().store(fd as i8, Ordering::SeqCst)
    }
}

impl std::fmt::Display for EventHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "event: {:?}", self.event_stream.lock().unwrap())
    }
}

pub struct Das {
    last_time: Instant,
    last_key:  Key,
    delay: Duration,
}

impl Das {
    fn eat_input(&mut self, event: Event) -> bool {
        match event.type_ {
            EventType::MouseEvent(_) => return false,
            EventType::KeyEvent(key) => {
                if key == self.last_key && event.time - self.last_time > self.delay {
                    self.delay = REPEAT_RATE;
                    return false
                } else { 
                    self.last_key = key;
                    return true 
                }
            }
        }
    }
}

impl Default for Das {
    fn default() -> Self {
        return Self { last_time: Instant::now(), last_key: Key::Null, delay: REPEAT_DELAY }
    }
}

#[derive(Debug, Clone)]
pub struct Event {
    pub type_: EventType,
    pub time: Instant,
    mode: HandlerMode,
}

impl From<InputEvent> for Event {
    fn from(value: InputEvent) -> Self {
        Self {
            type_: EventType::KeyEvent(value.code.into()),
            time: Instant::now(),
            mode: HandlerMode::DevInput
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
pub struct InputEvent {
    pub time: Instant,
    pub type_: u16,
    pub code: u16,
    pub value: i32,
}

impl InputEvent {
    pub fn poll(duration: i32, fd: i32) -> Option<Self> {
        let mut poll_fds = [PollFd::new(fd, PollFlags::POLLIN)];

        match poll(&mut poll_fds, duration) {
            Ok(n) => {
                if n > 0 {
                    let mut buf = [0u8; 24];
                    let _bytes_read = read(fd, &mut buf).unwrap();
                    let event: InputEvent = unsafe { std::mem::transmute(buf) };
                    if event.type_ == EV_KEY && event.value == 0 {
                        return Some(event)
                    } else { return None }
                }
                else { return None }
            },
            Err(_e) => return None,
        }
    }
}

impl Display for InputEvent {
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
            1  => Key::Esc,
            13 => Key::Char('='),
            16 => Key::Char('q'),
            28 => Key::Enter,
            74 => Key::Char('-'),
            78 => Key::Char('+'),
            96 => Key::Enter,
            _  => Key::Null,
        }
    }
}

impl Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
