//! 已解码投送结果保留原 Lease/全局槽；同 Grant 的后续 owner 才能领取，不复制 token。
use super::{Error, Lease, Slot};
use crate::projection_delivery::accept::{Received, Verified};
use std::sync::Arc;
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};

pub struct Ready {
    value: Verified,
    lease: Arc<Lease>,
    _slot: Slot,
    intent: crate::projection_delivery::journal::Ticket,
    fresh: bool,
}
impl Ready {
    pub(super) fn new(
        value: Verified,
        lease: Arc<Lease>,
        slot: Slot,
        intent: crate::projection_delivery::journal::Ticket,
        fresh: bool,
    ) -> Self {
        Self {
            value,
            lease,
            _slot: slot,
            intent,
            fresh,
        }
    }
    pub fn origin(&self) -> &str {
        self.intent.origin()
    }
    pub fn expires_at_ms(&self) -> u64 {
        self.lease.expires_at_ms()
    }
    pub(crate) fn ticket(&self) -> &crate::projection_delivery::journal::Ticket {
        &self.intent
    }
    pub(crate) fn cleanup_ticket(&self) -> Option<crate::projection_delivery::journal::Ticket> {
        self.fresh.then(|| self.intent.retry())
    }
    pub fn validate(&self, destination: &Lease, now: u64) -> Result<&Verified, Error> {
        self.lease
            .validate(self.origin(), now)
            .map_err(|_| Error::Stopped)?;
        destination
            .validate(self.origin(), now)
            .map_err(|_| Error::Stopped)?;
        if !self.lease.same_grant(destination) {
            return Err(Error::Stopped);
        }
        Ok(&self.value)
    }
    pub fn transfer(self, destination: &Lease, now: u64) -> Result<(Verified, super::Hold), Error> {
        self.validate(destination, now)?;
        Ok((
            self.value,
            super::Hold {
                lease: self.lease,
                _slot: self._slot,
                intent: self.intent,
                fresh: self.fresh,
            },
        ))
    }
    pub fn request_stop(&self) {
        self.lease.revoke();
    }
}
impl std::fmt::Debug for Ready {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProjectionReady")
            .field("value", &self.value)
            .finish_non_exhaustive()
    }
}
struct Apartment;
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}
pub(super) fn decode(
    value: Received,
    lease: &Lease,
    intent: &crate::projection_delivery::journal::Ticket,
) -> Result<Verified, Error> {
    if lease.is_revoked() {
        return Err(Error::Stopped);
    }
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok() }
        .map_err(|_| Error::Decode(crate::projection_delivery::Error::Snapshot))?;
    let _apartment = Apartment;
    value
        .verify(|| lease.is_revoked())
        .and_then(|value| value.bind_intent(intent))
        .map_err(Error::Decode)
}
