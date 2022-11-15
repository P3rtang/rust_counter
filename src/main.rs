#![allow(dead_code)]
use std::fmt;
use serde_derive::{Serialize, Deserialize};
use std::{fs::OpenOptions, io};
use io::{Write, Result, Read, stdout, stdin};
use termion::{raw::IntoRawMode, event::{Key, Event}, input::TermRead, clear, cursor::{Goto, Hide, Show}};

mod interface;
use interface::*;

const SAVE_FILE: &str = "data.json";

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
struct Counter {
    name: String,
    count: i32,
    active: bool,
}

impl Counter {
    fn new(name: &str) -> Self {
        return Counter { name: name.to_string() , count: 1, active: false }
    }
    fn set_count(&mut self, count: i32)  {
        self.count = count
    }
    fn increase_by (&mut self, amount: i32) {
        self.count += amount
    }
}

impl fmt::Display for Counter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.count)
    } 
}

#[derive(Clone, Serialize, Deserialize)]
struct CounterStore {
    /// An object to hold multiple counters in like a vec but specific for Counter
    ///
    /// Construct this object with new and add your own counters
    /// Or contstruct with a json file generated by a previous instance of CounterStore
    store: Vec<Counter>,
    index: usize,
}

impl CounterStore {
    fn new() -> Self {
        return CounterStore { store: Vec::new(), index: 0 }
    }
    fn get_by_name(&self, name: String) -> Option<&Counter> {
        for counter in &self.store {
            if counter.name == name {
                return Some(&counter)
            }
        }
        return None
    }
    fn push(&mut self, counter: Counter) {
        self.store.push(counter)
    }
    fn len(&self) -> usize {
        self.store.len()
    }
    fn to_json(&self) -> String {
        serde_json::to_string(&self).expect("Could not create json data")
    }
    fn from_json(json_str: &str) -> Result<Self> {
        let store: CounterStore = serde_json::from_str(json_str)?;
        return Ok(store)
    }
}

impl<Idx> std::ops::Index<Idx> for CounterStore
where
    Idx: std::slice::SliceIndex<[Counter]>,
{
    type Output = Idx::Output;

    fn index(&self, index: Idx) -> &Self::Output {
        &self.store[index]
    }
}

impl Iterator for CounterStore {
    type Item = Counter;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.store.len() {
            return None
        }
        let counter = self[self.index].clone();
        self.index += 1;
        return Some(counter);
    }
}

struct InterFace<'counter> {
    counter_store: Box<CounterStore>,
    running: bool,
    state: Vec<(u16, &'counter Counter)>,
}

impl<'counter> InterFace<'counter> {
    fn draw(&self) -> Result<()> {
        for (index, counter) in self.counter_store.clone().enumerate() {
            print!("{}{}: {}",Goto(1, (index + 1) as u16), counter.name, counter.count);
        }
        print!("{}", Hide);
        stdout().flush()?;
        return Ok(())
    }
    fn quit(&mut self) {
        self.running = false;
    }
    fn load() -> Result<InterFace<'counter>> {
        let mut file = OpenOptions::new().read(true).open(SAVE_FILE)?;
        let mut json_str = String::new();
        file.read_to_string(&mut json_str)?;
        return Ok(InterFace { counter_store: Box::new(CounterStore::from_json(&json_str)?), running: false, state: Vec::new() })
    }
    fn save(&self) -> Result<()> {
        let mut file = OpenOptions::new().write(true).open(SAVE_FILE)?;
        write!(&mut file, "{}", self.counter_store.to_json())
    }
    fn start(&mut self) {
        self.running = true;
        let mut stdout = stdout().into_raw_mode().expect("Could not enter raw mode");
        print!("{}", clear::All);
        self.draw().expect("could not draw in terminal");
        while self.running {
            match parse_terminal() {
                Some(Key::Char('q')) => { self.running = false; println!("{}", Show) },
                _ => {}
            }
        }
    }
}

fn parse_terminal() -> Option<Key> {
    let stdin = stdin();
    let _stdout = stdout().into_raw_mode().unwrap();
    for c in stdin.events() {
        let evt = c.unwrap();
        return match evt {
            Event::Key(key) => {
                Some(key)
            }
            _ => { None }
        }
    }
    return None
}

fn main() {
    let mut interface: InterFace;

    if let Ok(interf) = InterFace::load() {
        interface = interf;
        interface.start();
    } else {
        println!("issue found with data.json file could not read save data");
        let mut store = CounterStore::new();
        let names = ["foo", "baz", "bar"];
        for name in names {
            store.push(Counter::new(name))
        }
        interface = InterFace { counter_store: Box::new(store), running: true, state: Vec::new() };
    }
    while interface.running {
        interface.quit();
    }
    let mut window = InterFaceWindow::<Frame<EmptyWidget>>::new();
    let mut frame = Frame::<EmptyWidget>::new((32, 24));
    frame.set_border(Border::Full);
    
    window.attach(frame).unwrap();
    window.run().unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counterstore() {
        let mut store = CounterStore::new();
        let names = ["foo", "baz", "bar"];
        for name in names {
            store.push(Counter::new(name))
        }
        // test counterstore len attribute
        assert_eq!(store.len(), names.len());
        assert_eq!(store[2].name, "bar");
        assert_eq!(store.get_by_name("foo".to_string()).unwrap(), &store[0]);
        for (index, counter) in store.enumerate() {
            assert_eq!(counter.name, names[index]);
        }
    }
    #[test]
    fn test_counter() {
        let mut test = Counter::new("test");
        assert_eq!(test.count, 1);
        test.set_count(5);
        assert_eq!(test.count, 5);
        test.increase_by(7);
        assert_eq!(test.count, 12);
    }
}
