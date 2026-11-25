//! Partial restoration of the original display instance; revoke only TOPMOST borrowed by Otter, without restoring the entire application-window snapshot.
use super::*;
use crate::live_popup_layer_request::Channel;
use crate::live_recovery_wire::PopupLayer;
use otter_live_hover_hook::native_popup::Lease;
#[cfg(test)]
use otter_live_hover_hook::{native_popup_layer::MANAGED, native_protocol::value};
use std::{cell::RefCell, time::Instant};

pub struct Recovery {
    target: Target,
    lease: Lease,
    channel: RefCell<Channel>,
}
fn word(value: u64) -> Result<usize> {
    usize::try_from(value)
        .map_err(|_| Error::new(E_INVALIDARG, "Popup layer identity out of range"))
}
pub fn wire(lease: Lease) -> PopupLayer {
    PopupLayer {
        owner: lease.owner.0 as u64,
        root: lease.root.0 as u64,
        cookie: lease.cookie as u64,
        epoch: lease.epoch as u64,
        request: lease.request as u64,
        instance: lease.instance as u64,
    }
}
impl Recovery {
    pub fn bind(target: Target, layer: &PopupLayer) -> Result<Self> {
        let lease = Lease {
            popup: target.hwnd(),
            owner: HWND(word(layer.owner)? as *mut _),
            root: HWND(word(layer.root)? as *mut _),
            cookie: word(layer.cookie)?,
            epoch: word(layer.epoch)?,
            request: word(layer.request)?,
            instance: word(layer.instance)?,
        };
        target.validate()?;
        if !lease.same_instance() {
            return Err(Error::new(E_ABORT, "Popup display instance changed"));
        }
        // Both host and independent child hold their own source-thread hooks before ACK/actual borrowing, independent of B/input worker.
        let channel = RefCell::new(Channel::new(&target, lease)?);
        Ok(Self {
            target,
            lease,
            channel,
        })
    }
    pub fn record(&self) -> Result<Record> {
        let record = make_record(&self.target, Restoration::PopupLayer(wire(self.lease)))?;
        if !self.lease.same_instance() {
            return Err(Error::new(E_ABORT, "Popup display instance changed"));
        }
        Ok(record)
    }
    pub fn acquire(&self) -> Result<()> {
        self.channel.borrow_mut().acquire()
    }
    pub fn restore(&self) -> Result<()> {
        self.channel.borrow_mut().restore()
    }
    pub fn deadline(&self) -> Option<Instant> {
        self.channel.borrow().deadline()
    }
    /// Called only by the independent helper's recovery loop; do not pump messages while UI/Channel RefCells are borrowed.
    pub fn drain_helper_notifications(&self) {
        let window = self.channel.borrow().notification_window();
        unsafe {
            // Each owner has only one completion-notification slot; consume boundedly and take only its own hidden request window.
            for _ in 0..4 {
                let mut message = MSG::default();
                if !PeekMessageW(&mut message, Some(window), 0, 0, PM_REMOVE).as_bool() {
                    break;
                }
                DispatchMessageW(&message);
            }
        }
    }
}
#[cfg(any(test, feature = "test-support"))]
#[path = "live_popup_layer_fixture.rs"]
pub mod fixture;
#[cfg(test)]
#[path = "live_recovery_popup_layer_tests.rs"]
mod tests;
