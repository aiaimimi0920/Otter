//! Notify only local authorization revocation; independent of network wakes, without threads/polling or terminal-state reset.
use std::{
    io,
    os::windows::io::{AsHandle, AsRawHandle, BorrowedHandle, FromRawHandle, OwnedHandle},
    sync::{Arc, Mutex},
};
use windows::Win32::{
    Foundation::HANDLE,
    System::Threading::{CreateEventW, SetEvent},
};
#[derive(Clone)]
pub struct Notification(Arc<OwnedHandle>);
impl Notification {
    pub fn handle(&self) -> BorrowedHandle<'_> {
        self.0.as_handle()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    #[test]
    fn callback_registration_covers_both_revocation_orders_without_lock_reentry_or_duplicates() {
        for before in [false, true] {
            let watch = Arc::new(Watch::default());
            let count = Arc::new(AtomicUsize::new(0));
            if before {
                watch.revoke();
            }
            let weak = Arc::downgrade(&watch);
            let received = count.clone();
            watch.notify(Arc::new(move || {
                let _event = weak.upgrade().unwrap().notification().unwrap();
                received.fetch_add(1, Ordering::AcqRel);
            }));
            if !before {
                assert_eq!(count.load(Ordering::Acquire), 0);
            }
            watch.revoke();
            watch.revoke();
            assert_eq!(count.load(Ordering::Acquire), 1);
        }
    }
    #[test]
    fn callback_is_bounded_to_latest_owner_and_panics_do_not_escape_revocation() {
        let watch = Watch::default();
        watch.notify(Arc::new(|| panic!("replaced notification must not run")));
        let count = Arc::new(AtomicUsize::new(0));
        let received = count.clone();
        watch.notify(Arc::new(move || {
            received.fetch_add(1, Ordering::AcqRel);
        }));
        watch.revoke();
        assert_eq!(count.load(Ordering::Acquire), 1);
        watch.notify(Arc::new(|| panic!("injected late notification failure")));
    }
}
#[derive(Default)]
struct State {
    revoked: bool,
    event: Option<Arc<OwnedHandle>>,
    notify: Option<Arc<dyn Fn() + Send + Sync>>,
}
#[derive(Default)]
pub(super) struct Watch(Mutex<State>);
impl Watch {
    pub(super) fn notification(&self) -> io::Result<Notification> {
        let mut state = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(event) = &state.event {
            return Ok(Notification(event.clone()));
        }
        // Registration and revocation share one lock; prior revocation directly creates an already-signaled event.
        let raw =
            unsafe { CreateEventW(None, true, state.revoked, None) }.map_err(io::Error::other)?;
        let event = Arc::new(unsafe { OwnedHandle::from_raw_handle(raw.0) });
        state.event = Some(event.clone());
        Ok(Notification(event))
    }
    pub(super) fn revoke(&self) {
        let mut state = self.0.lock().unwrap_or_else(|e| e.into_inner());
        state.revoked = true;
        if let Some(event) = &state.event {
            let _ = unsafe { SetEvent(HANDLE(event.as_raw_handle())) };
        }
        let notify = state.notify.take();
        drop(state); // Callbacks may reenter authorization; do not hold the Watch lock or propagate notification panics through destruction.
        if let Some(notify) = notify {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| notify()));
        }
    }
    pub(super) fn notify(&self, notify: Arc<dyn Fn() + Send + Sync>) {
        let mut state = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if !state.revoked {
            state.notify = Some(notify); // The same serial Lease has only one long-lived result owner; listeners do not accumulate.
            return;
        }
        drop(state);
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| notify()));
    }
}
