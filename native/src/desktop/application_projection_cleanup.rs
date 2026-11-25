//! Cleanup state for receiver deletion/new Accept's unsettled intent; retain only the original Ticket, while networking stays in the original Sessions slot.
use super::*;
use crate::projection_delivery::journal::{Error as JournalError, Ticket, MAX_ENTRIES};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
#[path = "application_projection_cleanup_dispatch.rs"]
mod dispatch;
#[path = "application_projection_cleanup_mark.rs"]
mod mark;

#[derive(Default)]
pub(in crate::desktop::application) struct Owner {
    deleted: Vec<crate::projection_delivery::binding::Binding>,
    queued: Vec<Arc<Ticket>>,
    mark: Option<mark::Job>,
    mark_failures: u8,
    mark_due: Option<Instant>,
    network: Option<Arc<Ticket>>,
    retries: Vec<Retry>,
    wake: Option<Instant>,
}
struct Retry {
    ticket: Arc<Ticket>,
    failures: u8,
    due: Instant,
}
fn delay(failures: u8) -> Duration {
    Duration::from_secs((1u64 << failures.min(5)).min(30))
}
impl Owner {
    pub(in crate::desktop::application) fn abandon(&mut self, ticket: Ticket) {
        self.enqueue(Arc::new(ticket));
    }
    pub(in crate::desktop::application) fn resolve(&mut self, profile: &profile::Owner) {
        let Some(entries) = profile.entries() else {
            return;
        };
        for binding in std::mem::take(&mut self.deleted) {
            if let Some(ticket) = entries.iter().find(|ticket| binding.matches_ticket(ticket)) {
                self.enqueue(ticket.clone());
            }
        }
        self.prune(profile);
    }
    pub(in crate::desktop::application) fn prune(&mut self, profile: &profile::Owner) {
        if let Some(entries) = profile.entries() {
            if self.mark.is_none() {
                self.queued
                    .retain(|old| entries.iter().any(|entry| entry.same_intent(old)));
            }
            self.retries
                .retain(|retry| entries.iter().any(|entry| entry.same_intent(&retry.ticket)));
        }
    }
    fn enqueue(&mut self, ticket: Arc<Ticket>) {
        if !ticket.cleanup_pending() && !self.queued.iter().any(|old| old.same_intent(&ticket)) {
            // At most 128 original intents per profile; an unpersisted deletion fence prevents new receive admission.
            debug_assert!(self.queued.len() < MAX_ENTRIES);
            self.queued.push(ticket);
        }
    }
    pub(in crate::desktop::application) fn pending_local(&self) -> bool {
        !self.deleted.is_empty() || !self.queued.is_empty() || self.mark.is_some()
    }
    pub(in crate::desktop::application) fn deadline(&self) -> Option<Instant> {
        self.wake
            .into_iter()
            .chain(self.mark.as_ref().and_then(mark::Job::reap_deadline))
            .min()
    }
    fn schedule(&mut self, due: Instant) {
        self.wake = Some(self.wake.map_or(due, |wake| wake.min(due)));
    }
    fn fail_mark(&mut self, now: Instant) {
        self.mark_due = Some(now + delay(self.mark_failures));
        self.mark_failures = self.mark_failures.saturating_add(1);
    }
    fn due(&self, ticket: &Ticket) -> Option<Instant> {
        self.retries
            .iter()
            .find(|retry| retry.ticket.same_intent(ticket))
            .map(|retry| retry.due)
    }
    fn retry_network(&mut self, ticket: Arc<Ticket>, now: Instant) -> Instant {
        if let Some(retry) = self
            .retries
            .iter_mut()
            .find(|old| old.ticket.same_intent(&ticket))
        {
            retry.due = now + delay(retry.failures);
            retry.failures = retry.failures.saturating_add(1);
            retry.due
        } else {
            let due = now + delay(0);
            self.retries.push(Retry {
                ticket,
                failures: 1,
                due,
            });
            due
        }
    }
    /// At final-Unit/application terminal state, preserve deletion markers before allowing an empty checkpoint; do not network or unlink stable associations here.
    pub(in crate::desktop::application) fn flush(
        &mut self,
    ) -> std::result::Result<(), JournalError> {
        self.network = None;
        self.retries.clear();
        if !self.deleted.is_empty() {
            return Err(JournalError::Read);
        }
        if let Some(job) = self.mark.take() {
            if job.finish().is_ok() {
                self.queued.remove(0);
            }
        }
        while let Some(ticket) = self.queued.first() {
            drop(ticket.retry().queue_cleanup()?);
            self.queued.remove(0);
        }
        self.mark_due = None;
        self.wake = None;
        Ok(())
    }
}
