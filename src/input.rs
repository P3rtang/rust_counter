use crossterm::event::KeyCode;
use nix::unistd::read;
use nix::poll::{poll, PollFd, PollFlags};
use std::time::{Instant, Duration};

const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;
const KEY_ENTER: u16 = 28;

#[repr(C)]
#[derive(Debug)]
pub struct InputEvent {
    pub time: Instant,
    pub type_: u16,
    pub code: u16,
    pub value: i32,
}

impl InputEvent {
    pub fn poll(duration: Duration, fd: i32) -> Option<Self> {
        let mut poll_fds = [PollFd::new(fd, PollFlags::POLLIN)];

        loop {
            match poll(&mut poll_fds, duration.as_millis() as i32) {
                Ok(n) => {
                    if n > 0 {
                        let mut buf = [0u8; 24];
                        let _bytes_read = read(fd, &mut buf).unwrap();
                        // here you could parse the input event struct
                        let event: InputEvent = unsafe { std::mem::transmute(buf) };
                        if event.type_ == EV_KEY && event.value == 0 {
                            return Some(event)
                        }
                    }
                    else { return None }
                },
                Err(_e) => return None,
            }
        }
    }
}

#[derive(Debug)]
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
