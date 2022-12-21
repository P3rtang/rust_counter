#![allow(dead_code)]
use std::time::SystemTime;

mod counter;
mod app;
mod ui;
mod entry;

// you can freely change the name of this save file it will create an empty file if none with this
// name exist
const SAVE_FILE: &str = "data.json";

fn main() {
    let store = counter::CounterStore::from_json(SAVE_FILE)
        .expect("Could not create Counters from save file");
    let mut app = app::App::new(250, store.clone());
    app.start().unwrap();
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
        assert_eq!(store[2].get_name(), "bar");
        assert_eq!(store.get_by_name("foo".to_string())
                   .unwrap(), &store[0]);
        for (index, counter) in store.enumerate() {
            assert_eq!(counter.get_name(), names[index]);
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
