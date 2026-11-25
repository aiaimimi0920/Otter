use super::*;
use std::cell::{Cell, RefCell};
#[derive(Default)]
struct Memory {
    command: RefCell<Option<Vec<u8>>>,
    approval: RefCell<Option<Vec<u8>>>,
    writes: Cell<usize>,
    fail_approval: Cell<bool>,
}
impl Store for Memory {
    fn command(&self) -> io::Result<Option<Vec<u8>>> {
        Ok(self.command.borrow().clone())
    }
    fn approval(&self) -> io::Result<Option<Vec<u8>>> {
        Ok(self.approval.borrow().clone())
    }
    fn set_command(&self, command: Option<&[u8]>) -> io::Result<()> {
        self.writes.set(self.writes.get() + 1);
        *self.command.borrow_mut() = command.map(Vec::from);
        Ok(())
    }
    fn approve(&self) -> io::Result<()> {
        if self.fail_approval.get() {
            return Err(io::Error::other("injected approval write failure"));
        }
        *self.approval.borrow_mut() = Some(vec![2; 12]);
        Ok(())
    }
}
#[test]
fn executable_is_quoted_bounded_and_never_a_shell_command() {
    let bytes = command(Path::new("C:\\我的应用\\Otter app\\otter.exe")).unwrap();
    let text = String::from_utf16(
        &bytes
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect::<Vec<_>>(),
    )
    .unwrap();
    assert_eq!(text, "\"C:\\我的应用\\Otter app\\otter.exe\"\0");
    for path in ["relative.exe", "C:\\bad\"name.exe", "C:\\bad\nname.exe", ""] {
        assert!(command(Path::new(path)).is_err());
    }
    assert!(command(Path::new(&format!("C:\\{}.exe", "x".repeat(260)))).is_err());
}
#[test]
fn explicit_enable_honors_disabled_state_and_does_not_claim_other_installations() {
    let store = Memory::default();
    let expected = command(Path::new("C:\\Otter\\otter.exe")).unwrap();
    assert_eq!(status(&store, &expected).unwrap(), Status::Disabled);
    assert_eq!(set(&store, &expected, true).unwrap(), Status::Enabled);
    for flag in [3, 7] {
        *store.approval.borrow_mut() = Some(vec![flag; 12]);
        assert_eq!(status(&store, &expected).unwrap(), Status::Disabled);
        assert_eq!(set(&store, &expected, true).unwrap(), Status::Enabled);
    }
    let other = command(Path::new("C:\\main\\otter.exe")).unwrap();
    *store.command.borrow_mut() = Some(other.clone());
    let writes = store.writes.get();
    assert_eq!(
        status(&store, &expected).unwrap(),
        Status::OtherInstallation
    );
    assert!(set(&store, &expected, true).is_err());
    assert!(set(&store, &expected, false).is_err());
    assert_eq!(store.writes.get(), writes);
    assert_eq!(store.command.borrow().as_ref(), Some(&other));
}
#[test]
fn partial_approval_failure_is_reported_and_disable_is_idempotent() {
    let store = Memory::default();
    let expected = command(Path::new("C:\\Otter\\otter.exe")).unwrap();
    *store.approval.borrow_mut() = Some(vec![3; 12]);
    store.fail_approval.set(true);
    assert!(set(&store, &expected, true).is_err());
    assert_eq!(status(&store, &expected).unwrap(), Status::Disabled);
    assert_eq!(store.command.borrow().as_ref(), Some(&expected));
    assert_eq!(set(&store, &expected, false).unwrap(), Status::Disabled);
    assert_eq!(set(&store, &expected, false).unwrap(), Status::Disabled);
}
