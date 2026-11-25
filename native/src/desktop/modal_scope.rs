//! Application-wide modal-state observation; weak references neither extend document lifetime nor change independent document Guards.
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

#[derive(Clone, Default)]
pub(super) struct Scope(Rc<RefCell<Vec<Weak<Cell<bool>>>>>);

impl Scope {
    pub fn register(&self, flag: &Rc<Cell<bool>>) {
        let mut flags = self.0.borrow_mut();
        flags.retain(|flag| flag.strong_count() != 0);
        let weak = Rc::downgrade(flag);
        if !flags.iter().any(|entry| entry.ptr_eq(&weak)) {
            flags.push(weak);
        }
    }

    pub fn blocked(&self) -> bool {
        // Query read-only Cells without Win32 or list borrows across modal message loops.
        self.0
            .borrow()
            .iter()
            .filter_map(Weak::upgrade)
            .any(|flag| flag.get())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn observes_other_documents_without_sharing_their_local_lock() {
        let scope = Scope::default();
        let first = Rc::new(Cell::new(false));
        let second = Rc::new(Cell::new(false));
        scope.register(&first);
        scope.register(&second);
        let other = scope.clone();
        let guard = crate::desktop::file_dialog::Guard::begin(&first).unwrap();
        assert!(other.blocked());
        assert!(!second.get());
        let second_guard = crate::desktop::file_dialog::Guard::begin(&second).unwrap();
        drop(guard);
        assert!(other.blocked());
        drop(second_guard);
        assert!(!scope.blocked());
    }
    #[test]
    fn duplicate_registration_and_closed_document_do_not_retain_state() {
        let scope = Scope::default();
        let flag = Rc::new(Cell::new(true));
        scope.register(&flag);
        scope.register(&flag);
        assert_eq!(scope.0.borrow().len(), 1);
        assert_eq!(Rc::strong_count(&flag), 1);
        drop(flag);
        assert!(!scope.blocked());
        let next = Rc::new(Cell::new(false));
        scope.register(&next);
        assert_eq!(scope.0.borrow().len(), 1);
    }
}
