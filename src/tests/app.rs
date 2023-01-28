#[cfg(test)]

mod tests {
    use crate::app::*;

    #[test]
    fn test_modes() {
        let app = App::default();
        app.toggle_mode(AppMode::COUNTING);
        assert_eq!(app.get_mode(), AppMode::from_bits(0b0000_0000_0101).unwrap());
        app.toggle_mode(AppMode::DEBUGGING);
        assert_eq!(app.get_mode(), AppMode::from_bits(0b1000_0000_0101).unwrap());
        app.set_mode(AppMode::DEBUGGING);
        assert_eq!(app.get_mode(), AppMode::from_bits(0b1000_0000_0000).unwrap());
        app.toggle_mode(AppMode::DEBUGGING);
        assert_eq!(app.get_mode(), AppMode::from_bits(0b0000_0000_0000).unwrap());
        app.reset_mode();
        assert_eq!(app.get_mode(), AppMode::from_bits(0b0000_0000_0001).unwrap());
    }
    #[test]
    fn test_dialogs() {
        let mut app = App::default();
        app.open_dialog(Dialog::AddNew).unwrap();
        assert_eq!(app.get_opened_dialog(), &Dialog::AddNew);
        assert!(app.open_dialog(Dialog::Delete).is_err());
        app.close_dialog();
        assert_eq!(app.get_opened_dialog(), &Dialog::None);
        app.close_dialog();
        assert_eq!(app.get_opened_dialog(), &Dialog::None);
    }
    #[test]
    fn test_dialog_field() {
        let mut app = App::default();
        app.open_dialog(Dialog::AddNew).unwrap();
        assert_eq!(app.get_entry_state().get_active_field(), "");
        app.get_entry_state().push('H');
        assert_eq!(app.get_entry_state().get_active_field(), "H");
        app.get_entry_state().push_str("ello, World!");
        assert_eq!(app.get_entry_state().get_active_field(), "Hello, World!");
        app.get_entry_state().next();
        assert_eq!(app.get_entry_state().get_active_field(), "Hello, World!");
        app.get_entry_state().new_field("");
        assert_eq!(app.get_entry_state().get_active_field(), "");
        app.get_entry_state().next();
        assert_eq!(app.get_entry_state().get_active_field(), "Hello, World!");
        app.get_entry_state().pop();
        assert_eq!(app.get_entry_state().get_active_field(), "Hello, World");
    }
}
