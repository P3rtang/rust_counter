#![allow(dead_code)]
use std::time::SystemTime;
use nix::fcntl::{open, OFlag};

mod counter;
mod app;
mod ui;
mod widgets;
mod input;

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
        Err(e) => { app.debug_info.push(e.to_string()); 0}
    };
    app = app.set_super_user(fd);

    app = app.start().unwrap();
    let store = app.end().unwrap();
    println!("Debug Info:\n{}", 
        app.debug_info.iter().map(|debug_line| debug_line.to_string() + "\n")
        .collect::<String>()
    );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counterstore() {
        let mut store = counter::CounterStore::default();
        let names = ["foo", "baz", "bar"];
        for name in names {
            store.push(counter::Counter::new(name))
        }
        // test counterstore len attribute
        assert_eq!(store.len(), names.len());
        assert_eq!(store[2].borrow().get_name(), "bar");
        for (index, counter) in store.enumerate() {
            assert_eq!(counter.borrow().get_name(), names[index]);
        }
    }
    #[test]
    fn test_counter() {
        let mut test = counter::Counter::new("test");
        assert_eq!(test.get_count(), 1);
        test.set_count(5);
        assert_eq!(test.get_count(), 5);
        test.increase_by(7);
        assert_eq!(test.get_count(), 12);
    }
}
