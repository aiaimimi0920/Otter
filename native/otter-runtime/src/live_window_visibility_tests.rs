use super::*;
use windows::{
    core::w,
    Win32::{Foundation::HWND, UI::WindowsAndMessaging::*},
};
struct Window(HWND);

#[test]
fn restoration_errors_are_not_hidden_by_another_sources_pending_retirement() {
    let pending = || Error::new(RETIREMENT_PENDING, "pending retirement");
    let failure = || Error::new(E_ABORT, "actual restore failure");
    let mut first = None;
    retain_restore_error(&mut first, pending());
    assert_eq!(first.as_ref().unwrap().code(), RETIREMENT_PENDING);
    retain_restore_error(&mut first, failure());
    assert_eq!(first.as_ref().unwrap().code(), E_ABORT);
    retain_restore_error(&mut first, pending());
    retain_restore_error(&mut first, Error::new(E_INVALIDARG, "later failure"));
    assert_eq!(first.unwrap().message(), failure().message());
}

#[test]
fn registry_reports_a_later_restore_failure_instead_of_pending_retirement() {
    let mut registry = Registry::default();
    let first = Window::new();
    let second = Window::new();
    let waiting = registry.acquire(first.target()).unwrap();
    let failing = registry.acquire(second.target()).unwrap();
    let finish = Rc::new(Cell::new(false));
    let waiting_finish = finish.clone();
    waiting
        .arm(Snapshot::read(&first.target()).unwrap(), move || {
            if waiting_finish.get() {
                Ok(())
            } else {
                Err(Error::from_hresult(RETIREMENT_PENDING))
            }
        })
        .unwrap();
    let failing_finish = finish.clone();
    failing
        .arm(Snapshot::read(&second.target()).unwrap(), move || {
            if failing_finish.get() {
                Ok(())
            } else {
                Err(Error::new(E_ABORT, "fixture restore failure"))
            }
        })
        .unwrap();
    drop(waiting);
    drop(failing);
    assert_eq!(registry.retry_pending().unwrap_err().code(), E_ABORT);
    assert_eq!(
        registry.entries.len(),
        2,
        "both responsibilities remain owned"
    );
    finish.set(true);
    registry.retry_pending().unwrap();
    assert!(registry.entries.is_empty());
}
impl Window {
    fn new() -> Self {
        Self(unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
                w!("STATIC"),
                w!("visibility fixture"),
                WS_POPUP,
                20,
                20,
                32,
                32,
                None,
                None,
                None,
                None,
            )
            .unwrap()
        })
    }
    fn target(&self) -> Target {
        Target::bind(self.0).unwrap()
    }
    fn style(&self) -> isize {
        unsafe { GetWindowLongPtrW(self.0, GWL_EXSTYLE) }
    }
    fn change(&self) {
        unsafe {
            SetWindowLongPtrW(
                self.0,
                GWL_EXSTYLE,
                self.style() | WS_EX_TRANSPARENT.0 as isize,
            );
        }
    }
}

#[test]
fn logical_hide_requires_armed_snapshot_is_shared_and_restores_original_state() {
    let window = Window::new();
    let snapshot = Snapshot::read(&window.target()).unwrap();
    let before = snapshot.recovery_record().unwrap();
    let mut registry = Registry::default();
    let first = registry.acquire(window.target()).unwrap();
    let second = registry.acquire(window.target()).unwrap();
    assert!(first.hide().is_err());
    assert_eq!(window.style(), snapshot.extended_style);
    first
        .arm(Snapshot::read(&window.target()).unwrap(), || Ok(()))
        .unwrap();
    assert!(first.hide().unwrap());
    assert!(!second.hide().unwrap());
    let hidden = Snapshot::read(&window.target()).unwrap();
    assert_eq!(hidden.layered.unwrap().alpha, 1);
    assert_ne!(hidden.extended_style & WS_EX_TRANSPARENT.0 as isize, 0);
    assert!(!hidden.visible);
    drop(first);
    assert!(second.has_recovery());
    drop(second);
    assert_eq!(
        Snapshot::read(&window.target())
            .unwrap()
            .recovery_record()
            .unwrap(),
        before
    );
}

#[test]
fn minimized_hidden_source_is_restored_without_replacing_original_snapshot() {
    let window = Window::new();
    unsafe {
        let _ = ShowWindow(window.0, SW_SHOWNOACTIVATE);
    }
    let before = Snapshot::read(&window.target())
        .unwrap()
        .recovery_record()
        .unwrap();
    let mut registry = Registry::default();
    let lease = registry.acquire(window.target()).unwrap();
    lease
        .arm(Snapshot::read(&window.target()).unwrap(), || Ok(()))
        .unwrap();
    assert!(lease.hide().unwrap());
    for _ in 0..3 {
        unsafe {
            let _ = ShowWindow(window.0, SW_SHOWMINNOACTIVE);
        }
        assert!(unsafe { IsIconic(window.0).as_bool() });
        assert!(!lease.hide().unwrap());
        assert!(!unsafe { IsIconic(window.0).as_bool() });
        let hidden = Snapshot::read(&window.target()).unwrap();
        assert_eq!(hidden.layered.unwrap().alpha, 1);
        assert_ne!(hidden.extended_style & WS_EX_TRANSPARENT.0 as isize, 0);
    }
    drop(lease);
    assert_eq!(
        Snapshot::read(&window.target())
            .unwrap()
            .recovery_record()
            .unwrap(),
        before
    );
}

#[test]
fn pending_retirement_cannot_be_hidden_again() {
    let window = Window::new();
    let mut registry = Registry::default();
    let lease = registry.acquire(window.target()).unwrap();
    lease
        .arm(Snapshot::read(&window.target()).unwrap(), || {
            Err(Error::new(E_ABORT, "fixture retirement pending"))
        })
        .unwrap();
    lease.hide().unwrap();
    assert!(lease.restore().is_err());
    assert!(lease.hide().is_err());
    assert!(lease.has_recovery());
    drop(window);
    assert!(lease.restore().is_err());
}
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.0);
        }
    }
}

#[test]
fn peers_share_first_snapshot_and_only_last_drop_restores() {
    let window = Window::new();
    let original = window.style();
    let mut registry = Registry::default();
    let first = registry.acquire(window.target()).unwrap();
    let second = registry.acquire(window.target()).unwrap();
    assert_eq!(first.peer_count(), 2);
    assert!(first
        .arm(Snapshot::read(&window.target()).unwrap(), || Ok(()))
        .unwrap());
    window.change();
    assert!(!second
        .arm(Snapshot::read(&window.target()).unwrap(), || Ok(()))
        .unwrap());
    drop(first);
    assert_eq!(second.peer_count(), 1);
    assert_ne!(window.style(), original);
    drop(second);
    assert_eq!(window.style(), original);
    registry.retry_pending().unwrap();
    assert!(registry.entries.is_empty());
}

#[test]
fn explicit_reclaim_preserves_peers_and_allows_a_fresh_hide_cycle() {
    let window = Window::new();
    let original = window.style();
    let mut registry = Registry::default();
    let first = registry.acquire(window.target()).unwrap();
    let second = registry.acquire(window.target()).unwrap();
    for _ in 0..2 {
        assert!(first
            .arm(Snapshot::read(&window.target()).unwrap(), || Ok(()))
            .unwrap());
        window.change();
        second.restore().unwrap();
        assert_eq!(window.style(), original);
        assert_eq!(first.peer_count(), 2);
        assert!(!first.has_recovery() && !second.has_recovery());
    }
}

#[test]
fn failed_last_drop_retains_recovery_until_successful_retry() {
    let window = Window::new();
    let original = window.style();
    let mut registry = Registry::default();
    let lease = registry.acquire(window.target()).unwrap();
    lease
        .arm(Snapshot::read(&window.target()).unwrap(), || Ok(()))
        .unwrap();
    window.change();
    // Only the fixture corrupts the snapshot to inject restoration failure deterministically, without relying on permissions/system races.
    let original_rect = {
        let mut saved = lease.0.snapshot.borrow_mut();
        let snapshot = &mut Rc::get_mut(saved.as_mut().unwrap()).unwrap().snapshot;
        let rect = snapshot.rect;
        snapshot.rect.right = snapshot.rect.left;
        rect
    };
    drop(lease);
    assert!(registry.retry_pending().is_err());
    assert_eq!(registry.entries.len(), 1);
    assert_ne!(window.style(), original);
    {
        let mut saved = registry.entries[0].snapshot.borrow_mut();
        Rc::get_mut(saved.as_mut().unwrap()).unwrap().snapshot.rect = original_rect;
    }
    registry.retry_pending().unwrap();
    assert_eq!(window.style(), original);
    assert!(registry.entries.is_empty());
}

#[test]
fn rejects_foreign_snapshot_reentrancy_and_excess_peers() {
    let window = Window::new();
    let other = Window::new();
    let mut registry = Registry::default();
    let leases: Vec<_> = (0..LIMIT)
        .map(|_| registry.acquire(window.target()).unwrap())
        .collect();
    assert!(registry.acquire(window.target()).is_err());
    assert!(leases[0]
        .arm(Snapshot::read(&other.target()).unwrap(), || Ok(()))
        .is_err());
    assert!(!leases[0].has_recovery());
    leases[0].0.busy.set(true);
    assert!(leases[0].restore().is_err());
    assert!(registry.acquire(window.target()).is_err());
    leases[0].0.busy.set(false);
    drop(leases);
    registry.retry_pending().unwrap();
    assert!(registry.entries.is_empty());
}

#[test]
fn retirement_failure_is_retained_and_retry_does_not_rewrite_restored_window() {
    let window = Window::new();
    let original = window.style();
    let mut registry = Registry::default();
    let lease = registry.acquire(window.target()).unwrap();
    let ready = Rc::new(Cell::new(false));
    let permit = ready.clone();
    lease
        .arm(Snapshot::read(&window.target()).unwrap(), move || {
            if permit.get() {
                Ok(())
            } else {
                Err(Error::new(E_ABORT, "injected journal retirement failure"))
            }
        })
        .unwrap();
    window.change();
    assert!(lease.restore().is_err());
    assert_eq!(window.style(), original);
    assert!(lease.has_recovery());
    assert!(registry.acquire(window.target()).is_err());
    assert!(lease
        .arm(Snapshot::read(&window.target()).unwrap(), || Ok(()))
        .is_err());
    // After successful window restoration, registration-retirement retries must not overwrite the user's new changes again.
    window.change();
    ready.set(true);
    lease.restore().unwrap();
    assert_ne!(window.style(), original);
    assert!(!lease.has_recovery());
    drop(lease);
    registry.retry_pending().unwrap();
    assert!(registry.entries.is_empty());
}
#[test]
fn distinct_source_admission_is_bounded_and_reusable() {
    let windows: Vec<_> = (0..=LIMIT).map(|_| Window::new()).collect();
    let mut registry = Registry::default();
    let mut leases: Vec<_> = windows[..LIMIT]
        .iter()
        .map(|window| registry.acquire(window.target()).unwrap())
        .collect();
    assert!(registry.acquire(windows[LIMIT].target()).is_err());
    leases.pop();
    leases.push(registry.acquire(windows[LIMIT].target()).unwrap());
    assert_eq!(registry.entries.len(), LIMIT);
}

#[test]
fn vanished_source_still_waits_for_retirement_then_releases_registry_slot() {
    let window = Window::new();
    let mut registry = Registry::default();
    let lease = registry.acquire(window.target()).unwrap();
    let ready = Rc::new(Cell::new(false));
    let permit = ready.clone();
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    lease
        .arm(Snapshot::read(&window.target()).unwrap(), move || {
            observed.set(observed.get() + 1);
            if permit.get() {
                Ok(())
            } else {
                Err(Error::new(E_ABORT, "retirement pending"))
            }
        })
        .unwrap();
    lease.hide().unwrap();
    drop(window);
    assert!(lease.restore().is_err());
    assert_eq!(
        calls.get(),
        1,
        "Vanished window must not prevent journal retirement"
    );
    assert!(lease.has_recovery());
    drop(lease);
    assert!(registry.retry_pending().is_err());
    assert_eq!(registry.entries.len(), 1);
    ready.set(true);
    registry.retry_pending().unwrap();
    assert!(registry.entries.is_empty());
}
