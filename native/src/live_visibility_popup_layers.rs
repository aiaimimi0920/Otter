//! Reuse the application recovery journal to adopt still-visible ownerless menus; no persistent hierarchy borrow before ACK.
use super::*;
use crate::live_window_target::recovery::popup_layer::{self, Recovery};
use otter_live_hover_hook::{
    native_popup::Lease as PopupLease,
    native_popup_layer::{MANAGED, TEMPORARY},
    native_protocol::value,
};
use windows::{core::Result, Win32::UI::WindowsAndMessaging::*};

struct Entry {
    lease: PopupLease,
    recovery: Recovery,
    ticket: Ticket,
    wanted: bool,
    borrowed: bool,
    restored: bool,
    retiring: bool,
}
#[derive(Default)]
pub(super) struct Layers {
    entries: Vec<Entry>,
    rejected: Vec<PopupLease>,
}
fn eligible(lease: PopupLease) -> bool {
    if !lease.valid() {
        return false;
    }
    let popup = unsafe { GetWindowLongPtrW(lease.popup, GWL_EXSTYLE) } as u32;
    let root = unsafe { GetWindowLongPtrW(lease.root, GWL_EXSTYLE) } as u32;
    popup & WS_EX_NOACTIVATE.0 != 0
        && root & WS_EX_TOPMOST.0 != 0
        && (popup & WS_EX_TOPMOST.0 == 0
            || value(lease.popup, TEMPORARY) == lease.instance
            || value(lease.popup, MANAGED) == lease.instance)
}
impl Layers {
    pub(super) fn deadline(&self) -> Option<std::time::Instant> {
        self.entries
            .iter()
            .filter(|entry| !entry.restored)
            .filter_map(|entry| entry.recovery.deadline())
            .min()
    }
    pub(super) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub(super) fn sync(
        &mut self,
        requested: &[PopupLease],
        journal: &mut Journal,
    ) -> io::Result<()> {
        self.rejected
            .retain(|lease| requested.contains(lease) && lease.valid());
        for entry in &mut self.entries {
            entry.wanted = requested.contains(&entry.lease);
        }
        for &lease in requested {
            // Do not query HWND again for already-registered/rejected entries; poll independently rechecks registered entries.
            if self.rejected.contains(&lease)
                || self
                    .entries
                    .iter()
                    .any(|entry| entry.lease.popup == lease.popup)
                || !eligible(lease)
            {
                continue;
            }
            if self.entries.len() >= crate::live_recovery_wire::MAX_RECORDS {
                return Err(io::Error::other("Live popup layer limit reached"));
            }
            match Entry::register(lease, journal) {
                Ok(entry) => self.entries.push(entry),
                Err(error) if lease.valid() => return Err(error),
                // Menus may normally hide/destroy after discovery; hierarchy is not yet borrowed, so do not escalate an old display instance's
                // registration race to an application-exit error, rebind, or replay input.
                Err(_) => {}
            }
        }
        Ok(())
    }
    pub(super) fn poll(&mut self, closing: bool) -> Result<()> {
        let mut first = None;
        for entry in &mut self.entries {
            if entry.ticket.rejected() && !self.rejected.contains(&entry.lease) {
                self.rejected.push(entry.lease);
            }
            // Entries being returned/already returned only await retirement, avoiding further source-window/hierarchy-property queries.
            let keep = !closing
                && entry.wanted
                && !entry.retiring
                && !entry.restored
                && entry.ticket.available()
                && !entry.ticket.rejected()
                && eligible(entry.lease);
            let result = if keep {
                if entry.ticket.ready() {
                    entry.borrow()
                } else {
                    Ok(())
                }
            } else {
                entry.retire()
            };
            if let Err(error) = result {
                retain_restore_error(&mut first, error);
            }
        }
        self.entries
            .retain(|entry| !entry.restored || !entry.ticket.retired());
        first.map_or(Ok(()), Err)
    }
}
impl Entry {
    fn register(lease: PopupLease, journal: &mut Journal) -> io::Result<Self> {
        let target = Target::bind(lease.popup).map_err(io::Error::other)?;
        let layer = popup_layer::wire(lease);
        let recovery = Recovery::bind(target.clone(), &layer).map_err(io::Error::other)?;
        // Register only hierarchy responsibility for the exact display instance; never read/fabricate original UpdateLayeredWindow properties.
        let record = recovery.record().map_err(io::Error::other)?;
        let ticket = journal.register(record)?;
        Ok(Self {
            lease,
            recovery,
            ticket,
            wanted: true,
            borrowed: false,
            restored: false,
            retiring: false,
        })
    }
    fn borrow(&mut self) -> Result<()> {
        if !self.lease.valid() {
            return self.retire();
        }
        // Admit the single slot only after ACK and protect retirement responsibility before borrowing. Posted admission does not mean source execution/return.
        self.ticket.require_restore();
        self.borrowed = true;
        let result = self.recovery.acquire();
        match result {
            Err(_) if !self.lease.same_instance() => self.retire(),
            Err(error) if error.code() != RETIREMENT_PENDING => {
                self.retiring = true;
                Err(error)
            }
            result => result,
        }
    }
    fn retire(&mut self) -> Result<()> {
        self.retiring = true; // The same registration, already returning asynchronously, cannot be borrowed again when wanted is re-enabled.
        if !self.restored {
            if self.borrowed {
                self.recovery.restore()?;
            }
            self.restored = true;
        }
        self.ticket.retire()
    }
}
impl Drop for Entry {
    fn drop(&mut self) {
        // Application unwinding/final destruction still first attempts local return; unconfirmed records remain held by the independent child.
        if self.borrowed && !self.restored {
            match self.recovery.restore() {
                Ok(()) => {
                    let _ = self.ticket.retire();
                }
                Err(error) => eprintln!("native-live-popup-layer-final-restore: {error}"),
            }
        }
    }
}

impl Owner {
    pub fn sync_popup_layers(&mut self, leases: &[PopupLease]) -> io::Result<()> {
        if self.closing || self.draining {
            return Ok(());
        }
        if leases.is_empty() && self.popup_layers.is_empty() {
            return Ok(());
        }
        if self.journal.is_none() {
            self.journal = Some(Journal::start()?);
        }
        self.popup_layers
            .sync(leases, self.journal.as_mut().unwrap())
    }
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-live")))]
#[path = "live_visibility_popup_layer_tests.rs"]
mod tests;
