#![allow(dead_code)]
use std::fmt;
use std::time::Duration;
use serde_derive::{Serialize, Deserialize};
use std::io::{Result, Write};
use std::fs::File;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, Default)]
pub struct Counter {
    name:     String,
    count:    i32,
    active:   bool,
    time:     Duration,
    progress: Progress,
}

impl Counter {
    pub fn new(name: &str) -> Self {
        return Counter { name: name.to_string() , count: 0, active: false, time: Duration::default(), progress: Progress::default() }
    }

    pub fn set_count(&mut self, count: i32)  {
        self.count = count;
        self.progress.calc_progress(self.count as u64);
    }
    pub fn get_count(&self) -> i32 {
        return self.count
    }

    pub fn set_name(&mut self, name: &str) {
        if name == "" {
            return
        }
        self.name = name.to_string()
    }
    pub fn get_name(&self) -> String {
        return self.name.clone()
    }

    /// Sets the time of this [`Counter`].
    /// time in minutes
    pub fn set_time(&mut self, time: u64) {
        self.time = Duration::from_secs(time * 60)
    }
    pub fn get_time(&self) -> Duration {
        return self.time
    }

    pub fn increase_by (&mut self, amount: i32) {
        self.count += amount;
        self.progress.calc_progress(self.count as u64);
    }

    pub fn increase_time(&mut self, time: Duration) {
        self.time += time;
        self.progress.calc_progress(self.count as u64);
    }

    pub fn get_progress(&self) -> f64 {
        self.progress.progress
    }

    pub fn get_progress_odds(&self) -> u64 {
        match self.progress.kind {
            ProgressKind::Normal(odds) => odds,
            _ => 4096,
        }
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
    pub fn default() -> Self {
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
        let file = File::open(json_file);
        if file.is_err() {
            return Ok(CounterStore::default())
        }
        let store: CounterStore = serde_json::from_reader(file.unwrap())?;
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

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
enum ProgressKind {
    Normal(u64),
    DexNav,
    Sos,
}


#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct Progress {
    progress: f64,
    kind:     ProgressKind,
}

impl Progress {
    fn new(kind: ProgressKind) -> Self {
        Progress { progress: 0.0, kind }
    }
    fn calc_progress(&mut self, steps: u64) -> f64 {
        match self.kind {
            ProgressKind::Normal(odds) => {
                let neg_chance = (odds-1) as f64 / odds as f64;
                self.progress = 1f64 - neg_chance.powf(steps as f64);
                return self.progress
            },
            ProgressKind::DexNav => todo!(),
            ProgressKind::Sos => todo!(),
        }
    }
}

impl Default for Progress {
    fn default() -> Self {
        Self { progress: 0.0, kind: ProgressKind::Normal(8192) }
    }
}
