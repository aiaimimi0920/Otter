//! Retain original Grant/shared capacity/same-intent movable permits across background preparation/append/checkpoint; not proof of persistence/display.
use super::{Error, Lease, Slot};
use crate::projection_delivery::journal::Ticket;
use std::sync::Arc;

pub struct Hold {
    pub(super) lease: Arc<Lease>,
    pub(super) _slot: Slot,
    pub(super) intent: Ticket,
    pub(super) fresh: bool,
}
impl Hold {
    pub fn ticket(&self) -> &Ticket {
        &self.intent
    }
    pub(crate) fn cleanup_ticket(&self) -> Option<Ticket> {
        self.fresh.then(|| self.intent.retry())
    }
    /// Relinquish automatic compensation after actual saving; unknown indexes may conservatively downgrade without granting durable/display proof.
    pub(crate) fn stabilize(&mut self) {
        self.fresh = false;
    }
    pub fn origin(&self) -> &str {
        self.intent.origin()
    }
    pub fn expires_at_ms(&self) -> u64 {
        self.lease.expires_at_ms()
    }
    pub fn validate(&self, current: &Lease, now: u64) -> Result<(), Error> {
        self.check(now)?;
        current
            .validate(self.origin(), now)
            .map_err(|_| Error::Stopped)?;
        if !self.lease.same_grant(current) {
            return Err(Error::Stopped);
        }
        Ok(())
    }
    pub(crate) fn check(&self, now: u64) -> Result<(), Error> {
        self.lease
            .validate(self.origin(), now)
            .map_err(|_| Error::Stopped)
    }
    pub(crate) fn lease(&self) -> &Arc<Lease> {
        &self.lease
    }
}
