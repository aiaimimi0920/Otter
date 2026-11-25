//! Exact ticket handoff for original receive Jobs; original active permits stay until join, without scanning/guessing journals on failure.
use super::Error;
use crate::projection_delivery::{
    journal::{Store, Ticket},
    Invitation,
};
use std::sync::Mutex;

#[derive(Default)]
pub(super) struct Attempt(Mutex<Option<(Ticket, bool)>>);
impl Attempt {
    pub(super) fn begin(
        &self,
        store: &Store,
        invitation: &Invitation,
        device: &str,
    ) -> Result<Ticket, Error> {
        let (ticket, created) = store
            .begin_attempt(invitation, device)
            .map_err(Error::Journal)?;
        self.activate(ticket, created)
    }
    pub(super) fn recover(&self, ticket: Ticket) -> Result<Ticket, Error> {
        // Read-only recovery is not new Accept; failure/panic retains original permits but never automatic-compensation eligibility.
        self.activate(ticket, false)
    }
    pub(super) fn reattach(
        &self,
        store: &Store,
        invitation: &Invitation,
        device: &str,
    ) -> Result<Ticket, Error> {
        if !invitation.accepted() {
            return Err(Error::Stopped);
        }
        let ticket = store.begin(invitation, device).map_err(Error::Journal)?;
        // New local records do not mean new remote associations; failure/cancellation preserves them and must never automatically unlink old receivers.
        self.activate(ticket, false)
    }
    fn activate(&self, mut ticket: Ticket, created: bool) -> Result<Ticket, Error> {
        // Only successful activate permits networking; Busy/CAS failure cannot take another receive's compensation responsibility.
        ticket.activate().map_err(Error::Journal)?;
        let snapshot = ticket.retry();
        *self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some((ticket, created));
        Ok(snapshot)
    }
    pub(super) fn take(&self) -> Option<(Ticket, bool)> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
    }
    pub(super) fn discard_unapplied(&self) -> Result<bool, Error> {
        if let Some((ticket, true)) = self.take() {
            // Only records newly created and retired in this attempt may retry; old records may represent uncertain side effects.
            ticket.retire().map_err(Error::Journal)?;
            return Ok(true);
        }
        Ok(false)
    }
    pub(super) fn abandon(&self) -> Option<Ticket> {
        let (ticket, created) = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()?;
        // marker retries require new activate: return a permit-free snapshot and release the original permit after network reclamation.
        // Reused old intent lacks proof excluding stable associations; retain only the original journal's explicit-recovery responsibility.
        created.then(|| ticket.retry())
    }
}
