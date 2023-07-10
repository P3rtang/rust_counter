#![allow(dead_code)]
use app::App;

mod app;
mod counter;
mod debugging;
mod input;
mod settings;
mod tests;
mod ui;
mod widgets;

fn main() {
    let save_path = get_save_location();
    let store = counter::CounterStore::from_json(&save_path)
        .expect("Could not create Counters from save file");

    let app = App::new(store, save_path.clone());

    match app.start() {
        Ok(app) => {
            let store = app.end().unwrap();
            store.to_json(save_path);
        }
        Err(e) => {
            app::cleanup_terminal_state().unwrap();
            eprintln!("{}", e);
            panic!()
        }
    };
}

#[cfg(target_os = "linux")]
fn get_save_location() -> String {
    let home_path = dirs::home_dir().unwrap();
    let home_dir = home_path.to_str().unwrap();
    format!("{}/{}", home_dir, ".local/share/counter-tui/data.json")
}

#[cfg(target_os = "windows")]
fn get_save_location() -> String {
    let save_path = "data.json".to_string();
    save_path
}

#[cfg(target_os = "windows")]
fn get_fd() -> i32 {
    0
}
