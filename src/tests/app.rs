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
}
