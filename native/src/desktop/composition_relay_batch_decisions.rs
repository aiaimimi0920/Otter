//! Batch decision-application tasks share capacity and merge back into batches after harvesting; retain anomalous input and prevent omitted exit journaling.
use super::*;
use crate::surface_batch_decision_runtime::{Error, Input, Job, Outcome, StartError};
impl Sessions {
    pub fn start_surface_batch_decision(
        &mut self,
        config: Config,
        lease: Lease,
        input: Input,
    ) -> Result<(), StartError> {
        let count = input.prepared.receipt.events.len();
        let preparation = (|| {
            if self.closed {
                return Err(io::ErrorKind::BrokenPipe.into());
            }
            if count == 0 {
                return Err(io::ErrorKind::InvalidInput.into());
            }
            if self.occupied() >= MAX_SESSIONS
                || count > surface_events::MAX_EVENT_RECEIPTS.saturating_sub(self.event_backlog())
            {
                return Err(io::ErrorKind::WouldBlock.into());
            }
            self.batch_decision_jobs
                .try_reserve(1)
                .map_err(io::Error::other)?;
            // All batch-task categories may complete before the next harvest.
            self.reserve_batch_outcomes()?;
            self.batch_decision_rejected
                .try_reserve(self.batch_decision_jobs.len() + 1)
                .map_err(io::Error::other)
        })();
        if let Err(error) = preparation {
            return Err(StartError {
                input: Box::new(input),
                error,
            });
        }
        self.batch_decision_jobs
            .push((count, Job::start(config, lease, input)?));
        Ok(())
    }
    pub(super) fn cancel_batch_decisions(&mut self) {
        for (_, job) in &self.batch_decision_jobs {
            job.request_stop();
        }
        for outcome in &mut self.batch_decision_rejected {
            outcome.delivery = Err(crate::device_authorization_lease::Error::Revoked);
        }
    }
    pub(super) fn poll_batch_decisions(&mut self) -> Vec<(String, surfaces::Error)> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|value| u64::try_from(value.as_millis()).ok())
            .unwrap_or(u64::MAX);
        let mut errors = Vec::new();
        let mut index = 0;
        while index < self.batch_decision_jobs.len() {
            if let Some(outcome) = self.batch_decision_jobs[index].1.try_join(now) {
                self.batch_decision_jobs.swap_remove(index);
                let origin = outcome.input.owner.binding().origin().to_owned();
                if let Err(error) = outcome.submission {
                    if error != Error::Stopped {
                        errors.push((origin.clone(), surfaces::Error::Decision(error)));
                    }
                }
                match outcome.into_batch() {
                    Ok(outcome) => self.batch_outcomes.push(outcome),
                    Err(outcome) => {
                        errors.push((origin, surfaces::Error::BatchDecisionMerge));
                        self.batch_decision_rejected.push(*outcome);
                    }
                }
            } else {
                index += 1;
            }
        }
        errors
    }
    /// Explicitly transfer responsibility for saving anomalous input; neither silently discard nor automatically retry decisions.
    pub fn take_rejected_batch_decision(&mut self) -> Option<Outcome> {
        self.batch_decision_rejected.pop()
    }
}
