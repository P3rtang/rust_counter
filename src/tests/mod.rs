mod app;

#[cfg(test)]
mod tests {
    use crate::counter::*;

    #[test]
    fn test_counterstore() {
        let mut store = CounterStore::default();
        let names = ["foo", "baz", "bar"];
        for name in names {
            store.push(Counter::new(name))
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
        let mut test = Counter::new("test");
        assert_eq!(test.get_count(), 0);
        test.set_count(5);
        assert_eq!(test.get_count(), 5);
        test.increase_by(7);
        assert_eq!(test.get_count(), 12);
        test.increase_by(-20);
        assert_eq!(test.get_count(), -8)
    }
}
