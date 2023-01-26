use serde_derive::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Settings {
    active_keyboard: String,
    keybinds: Keybinds,
}

impl Settings {
    fn new(atc_kbd: String) -> Self {
        Self { active_keyboard: atc_kbd, keybinds: Keybinds::default() }
    }
}

#[derive(Default, Serialize, Deserialize)]
struct Keybinds {
    
}
