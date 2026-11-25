//! Application Surface-event task collection; completion caches still consume capacity, and rejection/cancellation must preserve event evidence.
use super::*;
use crate::{
    desktop::surface_authorized_window::EventReceipt,
    surface_click_event::ClickEvent,
    surface_event_runtime::{
        Error as EventError, Job as EventJob, Outcome as EventOutcome, StartError,
    },
    surface_lifecycle_owner::Owner,
};
// Receipts are independent of network slots and must not block attachment deactivation; retain at most 32 unpersisted actions.
pub(super) const MAX_EVENT_RECEIPTS: usize = 32;
impl Sessions {
    pub(super) fn event_backlog(&self) -> usize {
        self.event_receipts.len()
            + self.batch_event_backlog()
            + self.event_jobs.len()
            + self.event_outcomes.len()
            + self.decision_jobs.len()
    }
    pub fn start_surface_event(
        &mut self,
        config: Config,
        lease: Lease,
        owner: Owner,
        revision: u64,
        event: ClickEvent,
    ) -> Result<(), StartError> {
        let error = if self.closed {
            Some(io::ErrorKind::BrokenPipe.into())
        } else if self.occupied() >= MAX_SESSIONS || self.event_backlog() >= MAX_EVENT_RECEIPTS {
            Some(io::ErrorKind::WouldBlock.into())
        } else {
            self.event_jobs.try_reserve(1).err().map(io::Error::other)
        };
        if let Some(error) = error {
            return Err(StartError {
                owner: Box::new(owner),
                event: Box::new(event),
                error,
            });
        }
        // Reserve completion-cache space for every in-flight task so allocation failure after reclamation cannot lose owners.
        if let Err(error) = self
            .event_outcomes
            .try_reserve(self.event_jobs.len() + self.decision_jobs.len() + 1)
        {
            return Err(StartError {
                owner: Box::new(owner),
                event: Box::new(event),
                error: io::Error::other(error),
            });
        }
        self.event_jobs
            .push(EventJob::start(config, lease, owner, revision, event)?);
        Ok(())
    }
    pub(super) fn cancel_surface_events(&mut self) {
        for job in &self.event_jobs {
            job.request_stop();
        }
        // Harvested results retain ACK/failure evidence; revoke only delivery eligibility.
        for outcome in &mut self.event_outcomes {
            outcome.delivery = Err(crate::device_authorization_lease::Error::Revoked);
        }
    }
    pub(super) fn poll_surface_events(&mut self) -> Vec<(String, surfaces::Error)> {
        // On clock failure use expiry time, never extending authorization with time 0; no periodic timer.
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|value| u64::try_from(value.as_millis()).ok())
            .unwrap_or(u64::MAX);
        let mut errors = Vec::new();
        let mut index = 0;
        while index < self.event_jobs.len() {
            if let Some(outcome) = self.event_jobs[index].try_join(now) {
                self.event_jobs.swap_remove(index);
                let error = outcome
                    .result
                    .as_ref()
                    .err()
                    .or_else(|| {
                        outcome
                            .confirmation
                            .as_ref()
                            .and_then(|result| result.as_ref().err())
                    })
                    .or_else(|| {
                        outcome
                            .convergence
                            .as_ref()
                            .and_then(|result| result.as_ref().err())
                    });
                if let Some(error) = error {
                    if !matches!(error, EventError::Stopped | EventError::Convergence(
                        crate::live_relay_http::ConvergenceError::Stopped
                        | crate::live_relay_http::ConvergenceError::ConfirmationRequired
                    ) | EventError::Confirmation(crate::live_relay_http::ConfirmationError::Stopped)) {
                        errors.push((
                            outcome.owner.binding().origin().to_owned(),
                            surfaces::Error::Event(*error),
                        ));
                    }
                }
                self.event_outcomes.push(outcome);
            } else {
                index += 1;
            }
        }
        errors
    }
    /// false leaves the original result in the collection; rejected results for old attachments must not block later matching results.
    pub fn deliver_surface_event(
        &mut self,
        mut deliver: impl FnMut(EventOutcome) -> Result<(), Box<EventOutcome>>,
    ) -> bool {
        for index in (0..self.event_outcomes.len()).rev() {
            let outcome = self.event_outcomes.remove(index);
            match deliver(outcome) {
                Ok(()) => return true,
                Err(outcome) => self.event_outcomes.insert(index, *outcome),
            }
        }
        false
    }
    pub fn take_surface_event_outcome(&mut self) -> Option<EventOutcome> {
        self.event_outcomes.pop()
    }
    /// Reserve receipt memory before calling windows; preserve complete rejected results without blocking other matching events.
    pub fn settle_surface_events(
        &mut self,
        mut deliver: impl FnMut(EventOutcome) -> Result<EventReceipt, Box<EventOutcome>>,
    ) -> io::Result<usize> {
        let available = MAX_EVENT_RECEIPTS.saturating_sub(self.event_receipts.len());
        self.event_receipts
            .try_reserve(available.min(self.event_outcomes.len()))
            .map_err(io::Error::other)?;
        let mut accepted = 0;
        for index in (0..self.event_outcomes.len()).rev() {
            if accepted == available {
                break;
            }
            let outcome = self.event_outcomes.remove(index);
            match deliver(outcome) {
                Ok(receipt) => {
                    self.event_receipts.push(receipt);
                    accepted += 1;
                }
                Err(outcome) => self.event_outcomes.insert(index, *outcome),
            }
        }
        Ok(accepted)
    }
    /// Explicitly transfer evidence ownership; callers must handle later persistence rather than discard it as ordinary UI notification.
    pub fn take_surface_event_receipt(&mut self) -> Option<EventReceipt> {
        self.event_receipts.pop()
    }
}
