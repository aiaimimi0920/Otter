//! UI-thread shared source-visibility lease, independent of frame-consumer counts; hiding requires arm, without creating a recovery journal itself.
use super::{snapshot::Snapshot, Target};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use windows::{
    core::{Error, Result},
    Win32::Foundation::{E_ABORT, E_INVALIDARG, E_OUTOFMEMORY},
};

const LIMIT: usize = 16;
pub const RETIREMENT_PENDING: windows::core::HRESULT =
    windows::core::HRESULT(0x8000000A_u32 as i32);

/// Retirement waits for multiple sources must not hide real recovery failures; retain the earliest error among equal-priority errors.
pub fn retain_restore_error(first: &mut Option<Error>, error: Error) {
    if first.as_ref().is_none_or(|previous| {
        previous.code() == RETIREMENT_PENDING && error.code() != RETIREMENT_PENDING
    }) {
        *first = Some(error);
    }
}
struct Recovery {
    snapshot: Snapshot,
    retire: Box<dyn Fn() -> Result<()>>,
    restored: Cell<bool>,
    hide_attempted: Cell<bool>,
    hidden: Cell<bool>,
}
struct Entry {
    target: Target,
    leases: Cell<usize>,
    snapshot: RefCell<Option<Rc<Recovery>>>,
    busy: Cell<bool>,
}
struct Busy<'a>(&'a Cell<bool>);
impl Drop for Busy<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
impl Entry {
    fn restore(&self) -> Result<()> {
        if self.busy.replace(true) {
            return Err(Error::new(E_ABORT, "Live visibility operation reentered"));
        }
        let _busy = Busy(&self.busy);
        let snapshot = self.snapshot.borrow().clone();
        if let Some(snapshot) = snapshot {
            // Do not retain RefCell borrows across Win32 calls, allowing conservative rejection after synchronous-message reentry.
            if !snapshot.restored.get() {
                snapshot.snapshot.restore_or_retire()?;
                snapshot.restored.set(true);
            }
            // Recovery-record retirement must also succeed; do not clear local state while leaving old records that the watchdog can replay.
            (snapshot.retire)()?;
            self.snapshot.borrow_mut().take();
        }
        Ok(())
    }
}

/// One application UI owner holds one registry; Rc prevents transferring window-restoration responsibility to background threads.
#[derive(Default)]
pub struct Registry {
    entries: Vec<Rc<Entry>>,
}
impl Registry {
    /// Explicit recall does not decrement lease references; restoration must still await registration retirement for retries by the application-level source owner.
    pub fn restore_source(&self, target: &Target) -> Result<()> {
        let entry = self
            .entries
            .iter()
            .find(|entry| entry.target.same_source(target))
            .ok_or_else(|| Error::new(E_INVALIDARG, "Live visibility source is not owned"))?;
        entry.restore()
    }
    pub fn acquire(&mut self, target: Target) -> Result<Lease> {
        target.validate()?;
        self.prune();
        if let Some(entry) = self
            .entries
            .iter()
            .find(|entry| entry.target.identity == target.identity)
        {
            entry.target.validate()?;
            if entry.busy.get()
                || entry
                    .snapshot
                    .borrow()
                    .as_ref()
                    .is_some_and(|record| entry.leases.get() == 0 || record.restored.get())
            {
                return Err(Error::new(E_ABORT, "Live visibility operation in progress"));
            }
            if entry.leases.get() >= LIMIT {
                return Err(Error::new(
                    E_OUTOFMEMORY,
                    "Live visibility lease limit reached",
                ));
            }
            entry.leases.set(entry.leases.get() + 1);
            return Ok(Lease(entry.clone()));
        }
        if self.entries.len() >= LIMIT {
            return Err(Error::new(
                E_OUTOFMEMORY,
                "Live visibility source limit reached",
            ));
        }
        self.entries
            .try_reserve(1)
            .map_err(|_| Error::new(E_OUTOFMEMORY, "Cannot allocate Live visibility source"))?;
        let entry = Rc::new(Entry {
            target,
            leases: Cell::new(1),
            snapshot: RefCell::new(None),
            busy: Cell::new(false),
        });
        self.entries.push(entry.clone());
        Ok(Lease(entry))
    }
    /// Retry sources with recovery snapshots but no sessions; retain failures, with real errors taking priority over retirement waits.
    pub fn retry_pending(&mut self) -> Result<()> {
        let mut first = None;
        for entry in &self.entries {
            if entry.leases.get() == 0 {
                if let Err(error) = entry.restore() {
                    retain_restore_error(&mut first, error);
                }
            }
        }
        self.prune();
        first.map_or(Ok(()), Err)
    }
    fn prune(&mut self) {
        self.entries
            .retain(|entry| entry.leases.get() != 0 || entry.snapshot.borrow().is_some());
    }
}

pub struct Lease(Rc<Entry>);
impl Lease {
    /// Call only after successful registration with the independent recovery channel and before the first window change. This method alone does not guarantee crash recovery.
    /// A second peer does not replace the first snapshot; false means shared recovery state already exists.
    pub fn arm(
        &self,
        snapshot: Snapshot,
        retire: impl Fn() -> Result<()> + 'static,
    ) -> Result<bool> {
        if self.0.busy.get() {
            return Err(Error::new(E_ABORT, "Live visibility operation in progress"));
        }
        self.0.target.validate()?;
        snapshot.target.validate()?;
        if snapshot.target.identity != self.0.target.identity {
            return Err(Error::new(
                E_INVALIDARG,
                "Live visibility snapshot belongs to another source",
            ));
        }
        let mut saved = self.0.snapshot.borrow_mut();
        if saved.as_ref().is_some_and(|record| record.restored.get()) {
            return Err(Error::new(E_ABORT, "Live recovery retirement is pending"));
        }
        if saved.is_some() {
            return Ok(false);
        }
        *saved = Some(Rc::new(Recovery {
            snapshot,
            retire: Box::new(retire),
            restored: Cell::new(false),
            hide_attempted: Cell::new(false),
            hidden: Cell::new(false),
        }));
        Ok(true)
    }
    pub fn has_recovery(&self) -> bool {
        self.0.snapshot.borrow().is_some()
    }
    pub fn peer_count(&self) -> usize {
        self.0.leases.get()
    }
    /// arm must rely on an independent recovery ACK. false means a same-source peer already hid the window, without replacing the first snapshot.
    /// Attempt rollback on partial failure; rollback/retirement failures retain state, preventing another hide atop unknown state.
    pub fn hide(&self) -> Result<bool> {
        if self.0.busy.replace(true) {
            return Err(Error::new(E_ABORT, "Live visibility operation reentered"));
        }
        let busy = Busy(&self.0.busy);
        let snapshot = self.0.snapshot.borrow().clone().ok_or_else(|| {
            Error::new(E_ABORT, "Live source hide requires recovery registration")
        })?;
        if snapshot.restored.get() {
            return Err(Error::new(E_ABORT, "Live recovery retirement is pending"));
        }
        if snapshot.hidden.get() {
            // Later native minimization stops source rendering; restore logical hiding from the first snapshot without replacing recovery registration.
            self.0.target.validate()?;
            if unsafe { windows::Win32::UI::WindowsAndMessaging::IsIconic(self.0.target.hwnd()) }
                .as_bool()
            {
                snapshot.snapshot.apply_hidden()?;
            }
            return Ok(false);
        }
        if snapshot.hide_attempted.replace(true) {
            return Err(Error::new(E_ABORT, "Live source hide rollback is pending"));
        }
        let result = snapshot.snapshot.apply_hidden();
        if result.is_ok() {
            snapshot.hidden.set(true);
        }
        drop(busy);
        match result {
            Ok(()) => Ok(true),
            Err(error) => {
                // Error paths also retain the first snapshot until both window restoration and registration retirement succeed.
                self.0.restore()?;
                Err(error)
            }
        }
    }
    /// Proactive recall restores the source for all peers but retains every lease, without stopping capture or input owners.
    pub fn restore(&self) -> Result<()> {
        self.0.restore()
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        let remaining = self.0.leases.get() - 1;
        self.0.leases.set(remaining);
        if remaining == 0 {
            if let Err(error) = self.0.restore() {
                // Report only error codes to avoid exposing window titles; the registry and independent journal own subsequent restoration.
                eprintln!("native-live-visibility-restore: {:?}", error.code());
            }
        }
    }
}

#[cfg(test)]
#[path = "live_window_visibility_tests.rs"]
mod tests;
