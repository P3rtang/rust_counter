#![allow(dead_code)]
#![feature(iterator_try_collect)]

use crate::app::DebugKey;
use app::App;
use nix::fcntl::{open, OFlag};
use std::time::SystemTime;
use dirs;

mod app;
mod counter;
mod tests;
mod ui;
mod widgets;
mod input;
mod settings;

// you can freely change the name of this save file it will create an empty file if none with this
// name exist
const SAVE_FILE: &str = ".local/share/counter-tui/data.json";

fn main() {
    let home_path = dirs::home_dir().unwrap();
    let home_dir = home_path.to_str().unwrap();
    let save_path = format!("{}/{}", home_dir, SAVE_FILE);
    let store = counter::CounterStore::from_json(&save_path)
        .expect("Could not create Counters from save file");

    let mut app = app::App::new(store);

    let fd = match open(
        "/dev/input/event5",
        OFlag::O_RDONLY | OFlag::O_NONBLOCK,
        nix::sys::stat::Mode::empty(),
    ) {
        Ok(f) => f,
        Err(e) => {
            app.debug_info
                .borrow_mut()
                .insert(DebugKey::Warning(e.to_string()), "".to_string());
            0
        }
    };
    app = app.set_super_user(fd);

    match app.start() {
        Ok(app) => {
            let store = app.end().unwrap();
            store.to_json(&save_path);
        },
        Err(e) => {
            App::default().end().unwrap();
            println!("{}", e);
            panic!()
        }
    };
}

fn timeit<F: FnMut() -> T, T>(mut f: F) -> T {
    let start = SystemTime::now();
    let result = f();
    let end = SystemTime::now();
    let duration = end.duration_since(start).unwrap();
    println!("took {} microseconds", duration.as_micros());
    result
}
