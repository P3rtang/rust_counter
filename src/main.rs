#![allow(dead_code)]
use std::time::SystemTime;
use nix::fcntl::{open, OFlag};
use crate::app::DebugKey;

mod counter;
mod app;
mod ui;
mod widgets;
mod input;
<<<<<<< Updated upstream
=======
mod settings;
mod tests;
>>>>>>> Stashed changes

// you can freely change the name of this save file it will create an empty file if none with this
// name exist
const SAVE_FILE:     &str = "data.json";
const TIME_OUT:      u64  = 180;
const TICK_SLOWDOWN: u64  = 60;
const FRAME_RATE:    u64  = 25;

fn main() {
    let store = counter::CounterStore::from_json(SAVE_FILE)
        .expect("Could not create Counters from save file");

    let mut app = app::App::new(1000 / FRAME_RATE, store);

    let fd = match open(
        "/dev/input/event5",
        OFlag::O_RDONLY | OFlag::O_NONBLOCK, nix::sys::stat::Mode::empty()
    ) {
        Ok(f) => f,
        Err(e) => { app.debug_info.borrow_mut().insert(DebugKey::Warning(e.to_string()), "".to_string()); 0}
    };
    app = app.set_super_user(fd);

    app = app.start().unwrap();
    let store = app.end().unwrap();
    store.to_json(SAVE_FILE);
}

fn timeit<F: FnMut() -> T, T>(mut f: F) -> T {
    let start = SystemTime::now();
    let result = f();
    let end = SystemTime::now();
    let duration = end.duration_since(start).unwrap();
    println!("took {} microseconds", duration.as_micros());
    result
}
