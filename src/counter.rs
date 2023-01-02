#![allow(dead_code)]
use std::cell::{RefCell, Ref, RefMut};
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
    phases:   Vec<Phase>,
}

impl Counter {
    pub fn new(name: impl Into<String>) -> Self {
        Counter { 
            name: name.into(),
            count: 0,
            active: false,
            time: Duration::default(),
            progress: Progress::default(), 
            phases: vec![ Phase::new("Phase 1", 0, Duration::default()) ],
        }
    }

    pub fn set_count(&mut self, count: i32)  {
        let diff = count - self.count;
        self.count += diff;
        self.phases[0].count += diff;
        self.progress.calc_progress(self.count as u64);
    }
    pub fn get_count(&self) -> i32 {
        self.count
    }

    pub fn get_phase_count(&self) -> i32 {
        self.phases[0].count
    }
    pub fn get_nphase_count(&self, index: usize) -> i32 {
        if index >= self.phases.len() { panic!() }
        self.phases[index].count
    }
    pub fn get_phase_time(&self) -> Duration {
        self.phases[0].time
    }
    pub fn get_nphase_time(&self, index: usize) -> Duration {
        self.phases[index].time
    }

    pub fn set_name(&mut self, name: impl Into<String> + Copy) {
        if name.into() == "" {
            return
        }
        self.name = name.into()
    }
    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    /// Sets the time of this [`Counter`].
    /// time in minutes
    pub fn set_time(&mut self, time: u64) {
        self.time = Duration::from_secs(time * 60)
    }
    pub fn get_time(&self) -> Duration {
        self.time
    }

    pub fn increase_by (&mut self, amount: i32) {
        self.count += amount;
        self.phases[0].count += amount;
        self.progress.calc_progress(self.count as u64);
    }

    pub fn increase_time(&mut self, time: Duration) {
        self.time += time;
        self.phases[0].time += time;
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
    
    pub fn new_phase(&mut self) {
        self.phases.insert(0, Phase::new(&format!("Phase {}", self.phases.len() + 1), 0, Duration::default()))
    }
    pub fn get_phase_len(&self) -> usize {
        self.phases.len()
    }
    pub fn get_phases(&self) -> Vec<Phase> {
        self.phases.clone()
    }
    pub fn get_phase_name(&self, index: usize) -> String {
        self.phases[index].name.clone()
    }
    pub fn remove_phase(&mut self, mut index: usize) {
        if self.get_phase_len() <= 1 { panic!() };
        let phase = self.phases.remove(index);
        if index >= self.get_phase_len() { index = 0 }
        self.phases[index].count += phase.count
    }
    pub fn set_phase_name(&mut self, index: usize, name: impl Into<String>) {
        self.phases[index].name = name.into()
    }
}

impl fmt::Display for Counter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.count)
    } 
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct CounterStore {
    /// An object to hold multiple counters in like a vec but specific for Counter
    ///
    /// Construct this object with new and add your own counters
    /// Or contstruct with a json file generated by a previous instance of CounterStore
    store: Vec<RefCell<Counter>>,
    index: usize,
}

impl CounterStore {
    pub fn get(&self, index: usize) -> Option<Ref<Counter>>  {
        return self.store.get(index).map(|counter| counter.borrow())
    }
    pub fn get_mut(&self, index: usize) -> Option<RefMut<Counter>> {
        return self.store.get(index).map(|counter| counter.borrow_mut())
    }
    pub fn get_by_name(&self, name: impl Into<String> + Copy) -> Option<Ref<Counter>> {
        for counter in &self.store {
            if counter.borrow().name == name.into() {
                return Some(counter.borrow())
            }
        }
        None
    }
    pub fn push(&mut self, counter: Counter) {
        self.store.push(RefCell::new(counter))
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
        Ok(store)
    }
    pub fn get_counters(&self) -> Vec<RefCell<Counter>> {
        self.store.clone()
    }
    pub fn remove(&mut self, id: usize) {
        if (0..self.store.len()).contains(&id) {
            self.store.remove(id);
        }
    }
}

impl<Idx> std::ops::Index<Idx> for CounterStore
where
    Idx: std::slice::SliceIndex<[RefCell<Counter>]>,
{
    type Output = Idx::Output;

    fn index(&self, index: Idx) -> &Self::Output {
        &self.store[index]
    }
}

impl Iterator for CounterStore {
    type Item = RefCell<Counter>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.store.len() {
            return None
        }
        let counter = self[self.index].clone();
        self.index += 1;
        Some(counter)
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
                self.progress
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

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Phase {
    name:  String,
    count: i32,
    time:  Duration,
}

impl Phase {
    fn new(name: impl Into<String>, count: i32, time: Duration) -> Self {
        Self { name: name.into(), count, time }
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }
}
