//! 同一宿主的文件确认/选择/保存互斥；仅用 Cell，不跨模态调用持有 RefCell。
use std::cell::Cell;

pub(super) struct Guard<'a>(&'a Cell<bool>);
impl<'a> Guard<'a> {
    pub(super) fn begin(active: &'a Cell<bool>) -> Option<Self> {
        if active.replace(true) {
            None
        } else {
            Some(Self(active))
        }
    }
}
impl Drop for Guard<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nested_entry_cannot_release_owners_gate_and_windows_are_independent() {
        let first = Cell::new(false);
        let second = Cell::new(false);
        let guard = Guard::begin(&first).unwrap();
        assert!(Guard::begin(&first).is_none());
        assert!(first.get());
        let other = Guard::begin(&second).unwrap();
        drop(other);
        assert!(!second.get());
        assert!(first.get());
        drop(guard);
        assert!(!first.get());
    }
    #[test]
    fn error_and_unwind_restore_gate() {
        let active = Cell::new(false);
        let failed = || -> Result<(), ()> {
            let _guard = Guard::begin(&active).unwrap();
            Err(())
        };
        assert!(failed().is_err());
        assert!(!active.get());
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = Guard::begin(&active).unwrap();
            panic!("simulated modal failure");
        }));
        assert!(result.is_err());
        assert!(!active.get());
        assert!(Guard::begin(&active).is_some());
    }
}
