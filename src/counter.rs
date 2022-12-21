#![allow(dead_code)]
use std::fmt;
use serde_derive::{Serialize, Deserialize};
use std::io::{Result, Write};
use std::fs::File;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, Default)]
pub struct Counter {
    name: String,
    count: i32,
    active: bool,
}

impl Counter {
    pub fn new(name: &str) -> Self {
        return Counter { name: name.to_string() , count: 0, active: false }
    }

    pub fn set_count(&mut self, count: i32)  {
        self.count = count
    }
    pub fn get_count(&self) -> i32 {
        return self.count
    }

    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string()
    }
    pub fn get_name(&self) -> String {
        return self.name.clone()
    }

    pub fn increase_by (&mut self, amount: i32) {
        self.count += amount
    }
}

impl fmt::Display for Counter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.count)
    } 
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CounterStore {
    /// An object to hold multiple counters in like a vec but specific for Counter
    ///
    /// Construct this object with new and add your own counters
    /// Or contstruct with a json file generated by a previous instance of CounterStore
    store: Vec<Counter>,
    index: usize,
}

impl CounterStore {
    pub fn new() -> Self {
        return CounterStore { store: Vec::new(), index: 0 }
    }
    pub fn get(&self, index: usize) -> Option<&Counter> {
        return self.store.get(index)
    }
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Counter> {
        return self.store.get_mut(index)
    }
    pub fn get_by_name(&self, name: String) -> Option<&Counter> {
        for counter in &self.store {
            if counter.name == name {
                return Some(&counter)
            }
        }
        return None
    }
    pub fn push(&mut self, counter: Counter) {
        self.store.push(counter)
    }
    pub fn len(&self) -> usize {
        self.store.len()
    }
    pub fn to_json(&self, json_file: &str) {
        let     save = serde_json::to_string(&self).expect("Could not create json data");
        let mut file = File::create(json_file).unwrap();
        file.write_all(save.as_bytes()).unwrap();
    }
    pub fn from_json(json_file: &str) -> Result<Self> {
        let store: CounterStore = serde_json::from_reader(File::open(json_file).unwrap())?;
        return Ok(store)
    }
    pub fn get_counters(&self) -> Vec<Counter> {
        return self.store.clone()
    }
    pub fn remove(&mut self, id: usize) {
        if (0..self.store.len()).contains(&id) {
            self.store.remove(id);
        }
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
