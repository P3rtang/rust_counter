#![allow(dead_code)]

use app::App;
use dirs;
use std::time::SystemTime;

mod app;
mod counter;
mod debugging;
mod input;
mod settings;
mod tests;
mod ui;
mod widgets;

// you can freely change the name of this save file it will create an empty file if none with this
// name exist
#[cfg(target_os = "linux")]
const SAVE_FILE: &str = ".local/share/counter-tui/data.json";

#[cfg(not(target_os = "linux"))]
const SAVE_FILE: &str = "data.json";

fn main() {
    #[cfg(target_os = "linux")]
    let home_path = dirs::home_dir().unwrap();
    #[cfg(not(target_os = "linux"))]
    let home_path = "";

    let home_dir = home_path.to_str().unwrap();
    let save_path = format!("{}/{}", home_dir, SAVE_FILE);
    let store = counter::CounterStore::from_json(&save_path)
        .expect("Could not create Counters from save file");

    let mut app = app::App::new(store);

    let fd = get_fd();
    app = app.set_super_user(fd);

    match app.start() {
        Ok(app) => {
            let store = app.end().unwrap();
            store.to_json(&save_path);
        }
        Err(e) => {
            App::default().end().unwrap();
            println!("{}", e);
            panic!()
        }
    };
}

#[cfg(target_os = "linux")]
fn get_fd() -> i32 {
    use nix::fcntl::{open, OFlag};

    let fd = open(
        "/dev/input/event5",
        OFlag::O_RDONLY | OFlag::O_NONBLOCK,
        nix::sys::stat::Mode::empty(),
    )
    .unwrap_or(0);
    return fd;
}

#[cfg(not(target_os = "linux"))]
fn get_fd() -> i32 {
    0
}

fn timeit<F: FnMut() -> T, T>(mut f: F) -> T {
    let start = SystemTime::now();
    let result = f();
    let end = SystemTime::now();
    let duration = end.duration_since(start).unwrap();
    println!("took {} microseconds", duration.as_micros());
    result
}
